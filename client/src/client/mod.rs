#![allow(unused)]
pub mod memory;

use std::{collections::HashMap, io::Read, os::unix::net::UnixStream, process, rc::Rc, result::Result as StdResult};

use crate::{
    error::{error_context, fallback_error, Result},
    protocol::{base::*, xdg_shell::*, *},
    wire_format::{ClientStream, WireMsgHeader},
};

use log::{debug, error, info, trace, warn};

use memory::SharedBuffer;
use v2::{RawMessage, WlEventParseError};

type WlEventId = WaylandId;
type WlObjectId = WaylandId;
type EventParseFunc = for<'a> fn(WlEventId, WlObjectId, &'a [u8]) -> EventParseResult<WlEvent>;

type WlEventMsg = (WlObjectId, WlEvent);

type WlEventHandler = dyn FnMut(&mut WaylandClient, WlEventMsg) -> Option<WlEventMsg>;
pub struct WaylandClient<'a> {
    globals: HashMap<WlInterfaceId, WaylandId>,
    objects: WlObjectManager<'a>,
    wl_stream: Rc<ClientStream>,
    stream: UnixStream,
    buffer: ByteBuffer,
}

impl<'a> WaylandClient<'a> {
    pub fn connect(socket_path: &str) -> Result<Self> {
        todo!()
        //let stream = error_context!(
        //    UnixStream::connect(dbg!(socket_path)),
        //    "Failed to establish connection."
        //)?;
        //
        //let mut client = Self {
        //    objects: WlObjectManager::new(),
        //    globals: HashMap::new(),
        //    buffer: ByteBuffer::new(4 * 1024),
        //    wl_stream: Rc::new(ClientStream::new(
        //        stream.try_clone().expect("Unable to clone UnixStream"),
        //    )),
        //    stream,
        //};
        //
        //let display: WlDisplay = client.new_global();
        //assert!(display.get_object_id() == 1);
        //
        //let registry: WlRegistry = client.new_global();
        //display.get_registry(&registry)?;
        //
        //client.add_event_handler(&registry, |client, msg| {
        //    if let WlRegistryGlobal {
        //        name,
        //        interface,
        //        version,
        //    } = msg.1
        //    {
        //        let object_id = match interface.as_str() {
        //            "wl_compositor" => client.new_global_id::<WlCompositor>(),
        //            "xdg_wm_base" => client.new_global_id::<XdgWmBase>(),
        //            "wl_shm" => client.new_global_id::<WlShm>(),
        //            _ => return None,
        //        };
        //
        //        info!("Mapping {interface} to global");
        //        let registry: WlRegistry = client.get_global().unwrap();
        //        registry.bind(name, interface, version, object_id).unwrap(); // TODO: add proper error message
        //        return None;
        //    }
        //    Some(msg)
        //})?;
        //
        //Ok(client)
    }

    pub fn new_object<E, T: v2::WlInterface<E>>(&mut self) -> T {
        T::build(0, self.wl_stream.clone())
    }

    pub fn get_reference<E, T: v2::WlInterface<E>>(&self, object_id: u32) -> Option<T> {
        todo!()
    }

    fn add_event_handler<T, E, F>(&mut self, object: &T, handler: F)
    where
        T: v2::WlInterface<E>,
        F: FnMut(&mut Self, v2::WlEventMsg<E>) -> Option<v2::WlEventMsg<E>>,
    {
        todo!()
    }

    //fn new_global<T : WlInterface<_>>(&mut self) -> T {}

    //pub fn add_event_handler(
    //    &mut self,
    //    object: &impl WaylandInterface,
    //    handler: impl FnMut(&mut Self, WlEventMsg) -> Option<WlEventMsg> + 'static,
    //) -> Result<()> {
    //    self.objects.add_event_handler(object, Box::new(handler))
    //}

    //fn load_globals(&mut self) -> Result<()> {
    //    let registry: WlRegistry = self.new_global();
    //    let callback: WlCallBack = self.new_object();
    //    let display: WlDisplay =
    //        self.get_global().expect("Failed to get WlDisplay");
    //
    //    display.get_registry(&registry)?;
    //    display.sync(&callback)?;
    //
    //    loop {
    //        //let msg = self.next_msg()?;
    //        //if msg.is_none() {
    //        //  continue
    //        //}
    //
    //        let (obj_id, event) = self.next_msg()?;
    //        match event {
    //            WlRegistryGlobal {
    //                name,
    //                interface,
    //                version,
    //            } => {
    //                let object_id = match interface.as_str() {
    //                    "wl_compositor" => self.new_global_id::<WlCompositor>(),
    //                    "xdg_wm_base" => self.new_global_id::<XdgWmBase>(),
    //                    "wl_shm" => self.new_global_id::<WlShm>(),
    //                    _ => continue,
    //                };
    //
    //                info!("Mapping {interface} to global");
    //                registry.bind(name, interface, version, object_id)?;
    //            }
    //            WlCallBackDone(_) if obj_id == callback.get_object_id() => {
    //                return Ok(())
    //            }
    //            other => panic!("Unexpected message {other:?}"),
    //        }
    //    }
    //}

    //pub fn create_pool(
    //    &mut self,
    //    size: i32,
    //) -> Result<(WlShmPool, SharedBuffer)> {
    //    assert!(size > 0);
    //
    //    // TODO: remove object if error occurrs
    //    let shm: WlShm = self.get_global().expect("Failed to get global WlShm");
    //    let pool: WlShmPool = self.new_global();
    //    let buffer = SharedBuffer::alloc(size as usize)?;
    //
    //    shm.create_pool(&pool, buffer.as_file_descriptor(), size)?;
    //
    //    // TODO: find another way to save the buffer
    //    Ok((pool, buffer))
    //}

    //pub fn get_global<T: WaylandInterface>(&self) -> Option<T> {
    //    self.globals
    //        .get(&T::get_interface_id())
    //        .map(|&id| self.build_wlobject(id))
    //}
    //
    //pub fn new_object<T: WaylandInterface>(&mut self) -> T {
    //    let object_id = self.objects.new_id::<T>();
    //    self.build_wlobject(object_id)
    //}

    // TODO: add a method to get an object from its integer id c:
    pub fn event_loop(mut self) -> ! {
        loop {
            match self.next_msg() {
                //Err(err) if err.is_fatal() => {
                //    // TODO: fix this, I don't like it c:
                //    error!("Fatal error: {err:#?}");
                //    process::exit(1)
                //}
                Err(err) => error!("Got error {err:#?} reading message!"),
                Ok(msg) => {
                    let Some(handlers) = self.objects.get_handlers(msg.object_id) else {
                        error!("Received message to an non-existant object: {}", msg.object_id);
                        continue;
                    };

                    let mut current = Some(msg);
                    for handler in handlers.iter_mut().rev() {
                        let Some(msg) = current.take() else { break };
                        match handler(msg) {
                            Ok(msg) => current = msg,
                            Err(err) => {
                                info!("Error: {err:?}");
                                break;
                            }
                        }
                    }

                    if let Some(msg) = current {
                        warn!("No handler for event {} of object {}", msg.event_id, msg.object_id);
                    }
                }
            }
        }
    }

    //// helper functions
    //#[inline]
    //fn new_global_id<T: WaylandInterface>(&mut self) -> WaylandId {
    //    self.new_global::<T>().get_object_id()
    //}

    //#[inline]
    //fn build_wlobject<T: WaylandInterface>(&self, object_id: WaylandId) -> T {
    //    T::build(object_id, self.wl_stream.clone())
    //}
    //
    //fn new_global<T: WaylandInterface>(&mut self) -> T {
    //    let object = self.new_object::<T>();
    //    self.globals
    //        .insert(T::get_interface_id(), object.get_object_id());
    //    object
    //}
    //
    //fn get_object_info(&mut self, object_id: u32) -> Result<WlObjectInfo> {
    //    self.objects
    //        .get_object_info(object_id)
    //        .ok_or(fallback_error!("No info registered for object {object_id}"))
    //}

    fn next_msg(&mut self) -> Result<RawMessage> {
        let header = WireMsgHeader::build(error_context!(
            self.read_bytes(WireMsgHeader::WIRE_SIZE),
            "Failed to read wire message header"
        )?);

        trace!("Received a msg from {} with {} size", header.object_id, header.length);

        let payload = error_context!(
            self.read_bytes(header.length as usize - WireMsgHeader::WIRE_SIZE),
            "Failed to read message payload"
        )?;

        Ok(RawMessage {
            object_id: header.object_id,
            event_id: header.method_id,
            payload: payload.into(),
        })
    }

    //fn next_msg(&mut self) -> Result<WlEventMsg> {
    //    let header = WireMsgHeader::build(error_context!(
    //        self.read_bytes(WireMsgHeader::WIRE_SIZE),
    //        "Failed to read wire message header"
    //    )?);
    //
    //    trace!(
    //        "Received a msg from {} with {} size",
    //        header.object_id,
    //        header.length
    //    );
    //
    //    let obj_info = self.get_object_info(header.object_id)?;
    //    let payload = error_context!(
    //        self.read_bytes(header.length as usize - WireMsgHeader::WIRE_SIZE),
    //        "Failed to read message payload"
    //    )?;
    //
    //    let event = error_context!(
    //        (obj_info.event_parse_func)(
    //            header.object_id,
    //            header.method_id.into(),
    //            payload
    //        ),
    //        "Unable to parse event {} for object {} @ {:?}",
    //        header.method_id,
    //        header.object_id,
    //        obj_info.interface_id
    //    )?;
    //
    //    Ok((header.object_id, event))
    //}
    //
    fn read_bytes(&mut self, size: usize) -> Result<&[u8]> {
        self.buffer.read_bytes(size, &mut self.stream)
    }
    //
    //fn try_call(&mut self, msg: WlEventMsg) -> Option<WlEventMsg> {
    //    match &msg.1 {
    //        // TODO: add interface type for each of the messages
    //        WlDisplayError {
    //            object, message, ..
    //        } => {
    //            error!("Error {message:?} for object {object}.");
    //        }
    //
    //        WlDisplayDeleteId(object_id) => {
    //            debug!("Delecting object {object_id}.");
    //            self.objects.delete_id(*object_id);
    //        }
    //        _ => return Some(msg),
    //    }
    //    None
    //}
}

type MockingHandler<'a> = Box<dyn FnMut(RawMessage) -> StdResult<Option<RawMessage>, WlEventParseError> + 'a>;

struct WlObjectManager<'a> {
    objects: HashMap<WaylandId, Vec<MockingHandler<'a>>>,
    objects_id_count: u32,
}

impl<'a> WlObjectManager<'a> {
    fn new() -> Self {
        Self {
            objects: HashMap::new(),
            objects_id_count: 0,
        }
    }

    fn new_id(&mut self) -> WaylandId {
        self.objects_id_count += 1;
        assert!(self.objects_id_count < WaylandId::MAX);
        assert!(self.objects.insert(self.objects_id_count, Vec::new()).is_none());
        self.objects_id_count
    }

    fn add_handler(&mut self, object_id: WaylandId, handler: MockingHandler<'a>) {
        let mut handlers = self
            .objects
            .get_mut(&object_id)
            .unwrap_or_else(|| panic!("Failed to get handlers for object {object_id}"));
        handlers.push(handler);
    }

    fn get_handlers(&mut self, object_id: WaylandId) -> Option<&mut [MockingHandler<'a>]> {
        self.objects.get_mut(&object_id).map(|handlers| handlers.as_mut_slice())
    }
}

//type Handlers = Vec<Box<WlEventHandler>>;
// TODO: implement this (https://wayland-book.com/protocol-design/wire-protocol.html#object-ids)
//struct WlObjectManager {
//    objects: HashMap<WaylandId, (WlObjectInfo, Option<Handlers>)>,
//    objects_id_count: u32, // TODO: implement this properly
//}
//
//#[derive(Debug)]
//#[derive(Debug, Clone, Copy)]
//struct WlObjectInfo {
//    event_parse_func: EventParseFunc,
//    interface_id: WlInterfaceId,
//    //event_handlers: Option<Vec<Box<WlEventHandler>>>,
//}
//
//impl WlObjectManager {
//    fn new() -> Self {
//        Self {
//            objects: HashMap::new(),
//            objects_id_count: 0,
//        }
//    }
//
//    // TODO: make this less type dependent
//    fn new_id<T: WaylandInterface>(&mut self) -> WaylandId {
//        assert!(self.objects_id_count < WaylandId::MAX);
//
//        self.objects_id_count += 1;
//        let object_id = self.objects_id_count;
//
//        assert!(
//            self.objects
//                .insert(
//                    object_id,
//                    (
//                        WlObjectInfo {
//                            event_parse_func: T::parse_event,
//                            interface_id: T::get_interface_id(),
//                        },
//                        None
//                    )
//                )
//                .is_none(),
//            "for object_id = {object_id}"
//        );
//        object_id
//    }
//
//    fn delete_id(&mut self, object_id: WaylandId) {
//        assert!(self.objects.remove(&object_id).is_some());
//    }
//
//    // TODO: handle call to call handler function
//    fn add_event_handler(&mut self, object: &impl WaylandInterface, handler: Box<WlEventHandler>) -> Result<()> {
//        if let Some(obj_info) = self.objects.get_mut(&object.get_object_id()) {
//            // TODO: do I really want to use an Option<..> here?
//            match obj_info.1.as_mut() {
//                Some(handlers) => handlers.push(handler),
//                None => obj_info.1 = Some(vec![handler]),
//            }
//        } else {
//            todo!("Object not found ... provide an error for this ...")
//        }
//        Ok(())
//    }
//
//    // TODO: handle this
//    //fn call_listener(
//    //    &self,
//    //    client: &mut WaylandClient,
//    //    msg: WlEventMsg,
//    //) -> Option<WlEventMsg> {
//    //    if let (_, Some(handlers)) =
//    //        self.objects.get(&msg.0).expect("Failed to get object")
//    //    {
//    //        let mut result = Some(msg);
//    //        for handler in handlers {
//    //            if let Some(msg) = handler(client, result.unwrap()) {
//    //                result = Some(msg)
//    //            } else {
//    //                break;
//    //            }
//    //        }
//    //        None
//    //    } else {
//    //        Some(msg)
//    //    }
//    //}
//
//    fn get_object_info(&self, object_id: WaylandId) -> Option<WlObjectInfo> {
//        self.objects.get(&object_id).map(|info| info.0)
//    }
//}

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

    pub fn read_bytes(&mut self, bytes: usize, stream: &mut impl Read) -> Result<&[u8]> {
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
            panic!("Read 0 bytes from stream! Maybe it was closed...")
            //Err(fatal_error!("Read 0 bytes from stream! Maybe it was closed..."))
        } else if gotten < bytes {
            //Err(fallback_error!(
            //    "Failed to read {bytes} bytes, only able to read {gotten}!"
            //))
            panic!("Failed to read {bytes} bytes, only able to read {gotten}!")
        } else {
            let res = &self.data[self.head..self.head + bytes];
            self.head += bytes;
            Ok(res)
        }
    }
}
