use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Read, Write},
    iter,
    ops::Deref,
    os::unix::net::UnixStream,
    process,
    rc::Rc,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

// mod wire_protocol;

use crate::{
    error::{error_context, fallback_error, fatal_error, Error, Result},
    protocol::{
        base::*, EventParseResult, WaylandId, WaylandInterface, WaylandStream,
        WireMessage, WlEvent, WlInterfaceId,
    },
    wire_format::{ClientStream, WireMsgHeader},
};

use log::{debug, error, info, trace};

pub(super) type Locked<T> = Arc<Mutex<T>>;
pub(super) type Shared<T> = Arc<RefCell<T>>;

pub type WlEventId = WaylandId;
pub type WlObjectId = WaylandId;
type EventParseFunc =
    for<'a> fn(WlEventId, WlObjectId, &'a [u8]) -> EventParseResult<WlEvent>;
type IdsMapping = HashMap<WaylandId, EventParseFunc>;

#[derive(Debug, Clone, Copy)]
struct WlObjectInfo {
    event_parse_func: EventParseFunc,
    interface_id: WlInterfaceId,
}

// TODO: implement this (https://wayland-book.com/protocol-design/wire-protocol.html#object-ids)
struct WlIdManager {
    object_ids: HashMap<WaylandId, WlObjectInfo>,
    id_count: u32,
}

impl WlIdManager {
    fn new() -> Self {
        Self {
            object_ids: HashMap::new(),
            id_count: 0,
        }
    }

    fn new_id<T: WaylandInterface>(&mut self) -> WaylandId {
        assert!(self.id_count < WaylandId::MAX);

        self.id_count += 1;
        let object_id = self.id_count;

        assert!(
            self.object_ids
                .insert(
                    object_id,
                    WlObjectInfo {
                        event_parse_func: T::parse_event,
                        interface_id: T::get_interface_id()
                    }
                )
                .is_none(),
            "for object_id = {object_id}"
        );
        object_id
    }

    fn get_object_info(&self, object_id: WaylandId) -> Option<WlObjectInfo> {
        self.object_ids.get(&object_id).copied()
    }
}

pub struct WaylandClient {
    globals: HashMap<WlInterfaceId, WaylandId>,
    ids: Locked<WlIdManager>,
    wl_stream: Rc<ClientStream>,
    event_receiver: Receiver<WlEvent>,
}

impl WaylandClient {
    pub fn connect(socket_path: &str) -> Result<Self> {
        let (event_sender, event_receiver) = mpsc::channel();
        let socket = error_context!(
            UnixStream::connect(dbg!(socket_path)),
            "Failed to establish connection"
        )?;

        let object_ids = Arc::new(Mutex::new(WlIdManager::new()));
        let mut client = Self {
            event_receiver,
            globals: HashMap::new(),
            wl_stream: Rc::new(ClientStream::new(
                socket.try_clone().expect("Unable to clone UnixStream"),
            )),
            ids: Arc::clone(&object_ids),
        };

        let display: WlDisplay = client.new_global();
        assert!(display.get_object_id() == 1);
        ReceiverThread::new(event_sender, socket, object_ids).start();

        let registry: WlRegistry = client.new_global();
        display.get_registry(registry.get_object_id())?;

        // TODO: wait for all the messages

        Ok(client)
    }

    pub fn get_global<T: WaylandInterface>(&self) -> Option<T> {
        self.globals
            .get(&T::get_interface_id())
            .map(|&id| self.build_wlobject(id))
    }

    pub fn new_object<T: WaylandInterface>(&mut self) -> T {
        let object_id = self.ids.lock().unwrap().new_id::<T>();
        self.build_wlobject(object_id)
    }

    fn new_global<T: WaylandInterface>(&mut self) -> T {
        let object = self.new_object::<T>();
        self.globals
            .insert(T::get_interface_id(), object.get_object_id());
        object
    }

    fn build_wlobject<T: WaylandInterface>(&self, object_id: WaylandId) -> T {
        T::build(object_id, self.wl_stream.clone())
    }
}

// -------------------------------------------------------------
// -------------------------------------------------------------

// struct SharedStream(Shared<UnixStream>);
// unsafe impl Send for SharedStream {}

struct ReceiverThread {
    channel: Sender<WlEvent>,
    stream: UnixStream,
    object_ids: Locked<WlIdManager>,
    buffer: ByteBuffer,
    // callbacks: AsyncCallBack,
}

impl ReceiverThread {
    fn new(
        channel: Sender<WlEvent>,
        stream: UnixStream,
        object_ids: Locked<WlIdManager>,
    ) -> Self {
        Self {
            channel,
            object_ids,
            stream,
            buffer: ByteBuffer::new(4 * 1024),
        }
    }

    //     fn read_stream_bytes(
    //         &mut self,
    //         size: usize,
    //     ) -> CustomResult<Option<&[u8]>> {
    //         Ok(self.buffer.read_bytes(size, &mut self.stream)?)
    //     }
    //
    fn get_object_info(&mut self, object_id: u32) -> Result<WlObjectInfo> {
        let ids = self.object_ids.lock().unwrap();
        ids.get_object_info(object_id)
            .ok_or(fallback_error!("No info registered for object {object_id}"))
    }

    fn read_bytes(&mut self, size: usize) -> Result<&[u8]> {
        // TODO: why??
        // let refcell: &RefCell<UnixStream> = self.stream.0.deref();
        // let reader: &mut UnixStream = &mut refcell.borrow_mut();
        self.buffer.read_bytes(size, &mut self.stream)
    }

    fn wait_for_msg(&mut self) -> Result<WlEvent> {
        let header = WireMsgHeader::build(error_context!(
            self.read_bytes(WireMsgHeader::WIRE_SIZE),
            "Failed to read wire message header"
        )?);

        trace!(
            "Received a msg from {} with {} size",
            header.object_id, header.length
        );
        let obj_info = self.get_object_info(header.object_id)?;

        // 40 - 6 -> 34
        let payload = error_context!(
            self.read_bytes(header.length as usize - WireMsgHeader::WIRE_SIZE),
            "Failed to read message payload"
        )?;

        error_context!(
            (obj_info.event_parse_func)(
                header.object_id,
                header.method_id.into(),
                payload
            ),
            "Unable to parse event {} for object {}@{:?}",
            header.method_id,
            header.object_id,
            obj_info.interface_id
        )
    }

    fn start(mut self) {
        info!("Launching background thread.");
        thread::spawn(move || loop {
            // TODO: fix this later
            match self.wait_for_msg() {
                Err(err) if err.is_fatal() => {
                    error!("Fatal error: {err:#?}");
                    process::exit(1)
                }
                Err(err) => error!("Got error {err:#?} reading message!"),
                Ok(msg) => {
                    // trace!("Shipping event {msg:#?}");
                    self.channel
                        .send(msg)
                        .expect("Couldn't send event message over channel");
                }
            }

            // let msg = msg.unwrap();
            // if let Some(msg) = self.callbacks.try_call(msg) {
            //     trace!("Shipping +1 message");
            //     self.channel
            //         .send(msg)
            //         .expect("Couldn't send event message over channel");
            // }
        });
    }
}

// struct AsyncCallBack {
//     proto_state: Locked<ProtocolState>,
//     proto_ids: LockedProtocolIds,
//     stream: UnixStream,
// }
//
// impl AsyncCallBack {
//     fn new(
//         proto_state: Locked<ProtocolState>,
//         proto_ids: LockedProtocolIds,
//         stream: UnixStream,
//     ) -> Self {
//         Self {
//             proto_state,
//             proto_ids,
//             stream,
//         }
//     }
//
//     fn try_call(
//         &mut self,
//         msg: WaylandEventMessage,
//     ) -> Option<WaylandEventMessage> {
//         match msg.event {
//             WaylandEvent::ShmFormat(format) => {
//                 let mut mutex = self.proto_state.lock().unwrap();
//                 let state = mutex.borrow_mut();
//                 state.shm_format.push(format);
//                 warn!("Adding +1 shm format!");
//             }
//
//             WaylandEvent::DisplayError { message, .. } => {
//                 panic!("Display error {message}");
//             }
//
//             WaylandEvent::DisplayDelete(item) => {
//                 warn!("Received delecting msg for {item}");
//                 self.proto_ids.lock().unwrap().remove(&item);
//             }
//
//             // WaylandEvent::XdgSurfaceConfigure(value) => {
//             //     warn!("Received")
//             // }
//             WaylandEvent::XdgSurfaceConfigure(serial) => {
//                 warn!("Received configure with serial = {serial}");
//                 match wire_messages::make_request(
//                     &mut self.stream,
//                     msg.sender_id,
//                     WaylandRequest::XdgSurfaceAckConfigure(serial),
//                 ) {
//                     Ok(_) => warn!("Configure-ack sent with success"),
//                     Err(err) => warn!("Error sending configure-ack {err:?}"),
//                 };
//             }
//             _ => return Some(msg),
//         }
//         None
//     }
// }

// TODO: review this implementation later
pub struct ByteBuffer {
    data: Box<[u8]>,
    head: usize,
    tail: usize,
}

impl ByteBuffer {
    pub fn new(size: usize) -> Self {
        ByteBuffer {
            head: 0,
            tail: 0,
            data: vec![0; size].into_boxed_slice(),
        }
    }

    fn cached_bytes(&self) -> usize {
        self.tail - self.head
    }

    fn tail_space(&self) -> usize {
        self.data.len() - self.tail
    }

    pub fn read_bytes(
        &mut self,
        bytes: usize,
        stream: &mut impl Read,
    ) -> Result<&[u8]> {
        let cached_bytes = self.cached_bytes();

        assert!(
            bytes < self.data.len() && cached_bytes <= self.data.len(),
            "bytes = {bytes}, cached = {cached_bytes}, data_size = {}",
            self.data.len()
        );

        if bytes <= cached_bytes {
            let res = &self.data[self.head..self.head + bytes];
            self.head += bytes;
            return Ok(res);
        }

        let left_space = self.tail_space();
        if cached_bytes + left_space < bytes {
            self.data.copy_within(self.head..self.tail, 0);
            self.head = 0;
            self.tail = cached_bytes;
        }

        let size = stream.read(&mut self.data[self.tail..])?;
        self.tail += size;

        let gotten = size + cached_bytes;

        if gotten == 0 {
            Err(fatal_error!(
                "Read 0 bytes from stream! Maybe it was closed..."
            ))
        } else if gotten < bytes {
            Err(fallback_error!(
                "Failed to read {bytes} bytes, only able to read {gotten}!"
            ))
        } else {
            let res = &self.data[self.head..self.head + bytes];
            self.head += bytes;
            Ok(res)
        }
    }
}
