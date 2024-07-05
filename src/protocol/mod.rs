#![allow(unused_imports)]
pub mod api;
mod buffers;
pub mod parser;
mod wire_messages;

use self::api::{
    WaylandEvent, WaylandEventMessage, WaylandObject, WaylandRequest,
};
use crate::protocol::buffers::ByteBuffer;

use api::ObjectId;
use buffers::SharedBuffer;
use log::{debug, error, info, trace, warn};
use std::{
    borrow::BorrowMut,
    collections::HashMap,
    io::{IoSlice, Read},
    iter,
    os::{
        fd::{AsFd, AsRawFd},
        unix::net::{SocketAncillary, UnixStream},
    },
    sync::{
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

type Locked<T> = Arc<Mutex<T>>;
type CustomResult<T> = std::result::Result<T, WaylandClientError>;
type LockedProtocolIds = Locked<HashMap<u32, WaylandObject>>;
type Result<T> = color_eyre::Result<T>;
pub type BufferId = usize;

struct ProtocolState {
    shm_format: Vec<u32>,
}

impl ProtocolState {
    fn new() -> Self {
        ProtocolState {
            shm_format: Vec::new(),
        }
    }
}

pub struct WaylandClient {
    #[allow(dead_code)]
    proto_state: Arc<Mutex<ProtocolState>>,
    socket: UnixStream,
    ids: LockedProtocolIds,
    mapped: HashMap<WaylandObject, u32>,
    ids_count: u32,
    display_id: u32,
    channel: Receiver<WaylandEventMessage>,
    buffers: Vec<SharedBuffer>,
}

#[derive(Debug)]
enum WaylandClientError {
    NoMessage,
    #[allow(dead_code)]
    RandomError(String),
}

impl<T> From<T> for WaylandClientError
where
    T: std::fmt::Display,
{
    fn from(value: T) -> Self {
        WaylandClientError::RandomError(format!("Error {value}"))
    }
}

impl WaylandClient {
    pub fn connect(compositor_sock_path: &str) -> std::io::Result<Self> {
        let mapped = HashMap::from([(WaylandObject::Display, 1)]);
        let ids = Arc::new(Mutex::new(
            mapped.iter().map(|(&k, &v)| (v, k)).collect(),
        ));

        let proto_state = Arc::new(Mutex::new(ProtocolState::new()));

        let (sender, receiver) = mpsc::channel::<WaylandEventMessage>();
        let socket = UnixStream::connect(compositor_sock_path)?;

        let thread = ReceiverThread::new(
            sender,
            socket.try_clone().unwrap(),
            Arc::clone(&ids),
            Arc::clone(&proto_state),
        );
        thread.start();

        Ok(Self {
            socket,
            ids,
            proto_state,
            channel: receiver,
            ids_count: 2,
            display_id: mapped.get(&WaylandObject::Display).copied().unwrap(),
            mapped,
            buffers: Vec::new(),
        })
    }

    pub fn map_global(
        &mut self,
        name: u32,
        interface: &str,
        version: u32,
        object: WaylandObject,
    ) -> color_eyre::Result<()> {
        let mapping_id = self.new_id(object);
        let registry_id =
            self.get_global_mapping(WaylandObject::Registry).unwrap();
        info!("Binding '{interface}' with name {name} to {mapping_id}");
        self.send_request(
            registry_id,
            WaylandRequest::RegistryBind {
                name,
                version,
                interface: interface.to_string(),
                new_id: mapping_id,
            },
        )?;

        assert!(
            self.mapped.insert(object, mapping_id).is_none(),
            "Remapping of {:?}",
            object
        );

        Ok(())
    }


    pub fn get_shared_buffer(&mut self, buffer_id : usize) -> Option<&mut SharedBuffer> {
        if buffer_id >= self.buffers.len() {
            return None
        }

        Some(&mut self.buffers[buffer_id])
    }

    pub fn create_pool(
        &mut self,
        size: usize,
    ) -> Result<(ObjectId, usize)> {
        let shm_id = self.get_global_mapping(WaylandObject::Shm).unwrap();
        let pool_id = self.new_id(WaylandObject::ShmPool);
        let shared_buffer = SharedBuffer::alloc(size)?;

        let mut payload = Vec::new();
        let shm_file_fd = shared_buffer.file_fd();
        wire_messages::make_request(
            &mut payload,
            shm_id,
            WaylandRequest::ShmCreatePool {
                pool_id,
                fd: shm_file_fd,
                size: size as i32,
            },
        )?;

        let mut ancillary_buffer = [0; 128];
        let mut ancillary = SocketAncillary::new(&mut ancillary_buffer[..]);
        ancillary.add_fds(&[shm_file_fd][..]);

        self.socket.send_vectored_with_ancillary(
            &[IoSlice::new(&payload)][..],
            &mut ancillary,
        )?;

        self.buffers.push(shared_buffer);
        Ok((pool_id, self.buffers.len() - 1))
    }

    pub fn get_global_mapping(&self, obj_type: WaylandObject) -> Option<u32> {
        self.mapped.get(&obj_type).copied()
    }

    pub fn new_id(&mut self, obj_type: WaylandObject) -> u32 {
        let id = self.ids_count;
        self.ids_count = id + 1;
        self.ids.lock().unwrap().borrow_mut().insert(id, obj_type);
        if let WaylandObject::Registry = obj_type {
            self.mapped.insert(obj_type, id);
        }
        id
    }

    pub fn send_request(
        &mut self,
        object_id: u32,
        request: WaylandRequest,
    ) -> color_eyre::Result<()> {
        debug!("Sending request {:?} to {object_id}", request);
        wire_messages::make_request(&mut self.socket, object_id, request)
    }

    pub fn load_interfaces(&mut self) -> color_eyre::Result<()> {
        let display_id = self.display_id;
        let registry_id = self.new_id(WaylandObject::Registry);
        self.send_request(
            display_id,
            WaylandRequest::DisplayGetRegistry(registry_id),
        )?;

        let messages = self.pull_messages()?;
        info!("You got {}", messages.len());
        for event_msg in messages {
            if let WaylandEvent::RegistryGlobal {
                name,
                interface,
                version,
            } = event_msg.event
            {
                match WaylandObject::from_interface(interface.as_str()) {
                    Some(object) => {
                        info!("Mapped into global: {name} -> {interface}@{version}");
                        self.map_global(name, &interface, version, object)?
                    }
                    None => {
                        debug!("Unknown interface {interface}@{version}");
                    }
                }
            } else {
                warn!("Received {:?} while loading interfaces", event_msg)
            }
        }
        Ok(())
    }

    // pub fn create_pull() -> core

    pub fn pull_messages(
        &mut self,
    ) -> color_eyre::Result<Vec<WaylandEventMessage>> {
        let callback_id = self.new_id(WaylandObject::CallBack);
        self.send_request(
            self.display_id,
            WaylandRequest::DisplaySync(callback_id),
        )?;

        let mut results = Vec::new();
        loop {
            if let Some(msg) = self.get_next_msg() {
                match msg.event {
                    WaylandEvent::CallBackDone(_)
                        if msg.sender_id == callback_id =>
                    {
                        return Ok(results);
                    }
                    _ => results.push(msg),
                }
            } else {
                thread::sleep(Duration::from_millis(100))
            }
        }
    }

    fn get_next_msg(&self) -> Option<WaylandEventMessage> {
        match self.channel.try_recv() {
            Ok(value) => Some(value),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                panic!("The other side disconnected")
            }
        }
    }
}

struct ReceiverThread {
    channel: Sender<WaylandEventMessage>,
    stream: UnixStream,
    ids: LockedProtocolIds,
    callbacks: AsyncCallBack,
    buffer: ByteBuffer,
}

impl ReceiverThread {
    fn new(
        channel: Sender<WaylandEventMessage>,
        stream: UnixStream,
        ids: LockedProtocolIds,
        proto_state: Locked<ProtocolState>,
    ) -> Self {
        let proto_ids = Arc::clone(&ids);
        Self {
            channel,
            stream,
            ids,
            buffer: ByteBuffer::new(1 << 12),
            callbacks: AsyncCallBack::new(proto_state, proto_ids),
        }
    }

    fn read_stream_bytes(
        &mut self,
        size: usize,
    ) -> CustomResult<Option<&[u8]>> {
        Ok(self.buffer.read_bytes(size, &mut self.stream)?)
    }

    fn get_object(&mut self, object_id: u32) -> Option<WaylandObject> {
        self.ids.lock().unwrap().get(&object_id).copied()
    }

    fn wait_for_msg(&mut self) -> CustomResult<WaylandEventMessage> {
        let header_bytes =
            self.read_stream_bytes(wire_messages::HEADER_SIZE)?;

        if header_bytes.is_none() {
            return Err(WaylandClientError::NoMessage);
        }

        let header = wire_messages::read_header(header_bytes.unwrap());
        let object = self.get_object(header.object_id);

        if object.is_none() {
            return Err(WaylandClientError::RandomError(format!(
                "Didn't find mapping of object-id {} to an object.",
                header.object_id
            )));
        }

        let object = object.unwrap();
        debug!(
            "Received op = {} from = {} for obj = {:?}",
            header.method_id, header.object_id, object
        );

        let event = object.parse_event(
            header.method_id,
            self.read_stream_bytes(
                header.length as usize - wire_messages::HEADER_SIZE,
            )?
            .expect("Couldn't read event body"), // TODO: handle this
        )?;

        debug!("{event:?}");
        Ok(WaylandEventMessage {
            sender_id: header.object_id,
            sender_obj: object,
            event,
        })
    }

    fn start(mut self) {
        thread::spawn(move || loop {
            let msg = self.wait_for_msg();

            if let Err(err) = msg {
                match err {
                    WaylandClientError::NoMessage => {
                        warn!("Received no message waiting a few milliseconds");
                        thread::sleep(Duration::from_millis(100));
                    },
                    WaylandClientError::RandomError(err) => {
                        error!("Error processing a message {err:?}");
                    }
                }
                continue;
            }

            let msg = msg.unwrap();
            if let Some(msg) = self.callbacks.try_call(msg) {
                trace!("Shipping +1 message");
                self.channel
                    .send(msg)
                    .expect("Couldn't send event message over channel");
            }
        });
    }
}

struct AsyncCallBack {
    proto_state: Locked<ProtocolState>,
    proto_ids: LockedProtocolIds,
}

impl AsyncCallBack {
    fn new(
        proto_state: Locked<ProtocolState>,
        proto_ids: LockedProtocolIds,
    ) -> Self {
        Self {
            proto_state,
            proto_ids,
        }
    }

    fn try_call(
        &mut self,
        msg: WaylandEventMessage,
    ) -> Option<WaylandEventMessage> {
        match msg.event {
            WaylandEvent::ShmFormat(format) => {
                let mut mutex = self.proto_state.lock().unwrap();
                let state = mutex.borrow_mut();
                state.shm_format.push(format);
                warn!("Adding +1 shm format!");
            }

            WaylandEvent::DisplayError { message, .. } => {
                panic!("Display error {message}");
            }

            WaylandEvent::DisplayDelete(item) => {
                warn!("Received delecting msg for {item}");
                self.proto_ids.lock().unwrap().remove(&item);
            }
            _ => return Some(msg),
        }
        None
    }
}
