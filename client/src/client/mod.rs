//#![allow(unused)]
pub mod memory;

use std::{
    any::Any, cell::RefCell, collections::HashMap, io::Read, os::unix::net::UnixStream, rc::Rc
};

use crate::{
    error::{error_context, fallback_error, Error},
    protocol::{base::*, xdg_shell::*, *},
    wire_format::{ClientStream, WireMsgHeader},
};

use memory::SharedBuffer;
use log::{info, trace, warn, error};

pub struct WaylandClient<'a> {
    globals: HashMap<WlInterfaceId, WaylandId>,
    objects: WlObjectManager<'a>,
    stream: Rc<ClientStream>,
    socket: UnixStream,
    buffer: ByteBuffer,
}

impl<'a> WaylandClient<'a> {
    pub fn connect(socket_path: &str) -> Result<Self, Error> {
        let socket = error_context!(
            UnixStream::connect(dbg!(socket_path)),
            "Failed to establish connection."
        )?;

        let mut client = Self {
            objects: WlObjectManager::new(),
            globals: HashMap::new(),
            buffer: ByteBuffer::new(4 * 1024),
            stream: Rc::new(ClientStream::new(
                socket.try_clone().expect("Unable to clone UnixStream"),
            )),
            socket,
        };

        client.init_globals()?;
        Ok(client)
    }

    pub fn get_global<E, T: WlInterface<Event = E>>(&self) -> Option<T> {
        self.get_reference(self.globals.get(&T::get_interface_id()).copied()?)
    }

    pub fn get_reference<E, T: WlInterface<Event = E>>(&self, object_id: u32) -> Option<T> {
        match self.objects.get_object_interface(object_id) {
            Some(interface_id) if interface_id == T::get_interface_id() => {
                Some(T::build(object_id, self.stream.clone()))
            }
            _ => None,
        }
    }

    pub fn add_event_handler<T, E, F>(
        &mut self,
        object: &T,
        mut handler: F,
    ) -> Result<(), WlHandlerRegistryError>
    where
        T: WlInterface<Event = E>,
        F: FnMut(&mut WaylandClient, WlEventMsg<E>) + 'a,
        E: 'static,
    {
        let object_id = object.get_object_id();

        self.objects.add_handler(
            object_id,
            T::get_interface_id(),
            Box::new(move |client, msg| match WlEventMsg::from_any(msg) {
                Some(msg) => {
                    assert_eq!(object_id, msg.object_id);
                    handler(client, msg);
                }
                None => panic!("Unable to get WlEventMsg<...> for object {object_id}"),
            }),
        )
    }

    pub fn event_loop(mut self)  {
        loop  {
             match self.next_msg() {
                Err(err) => error!("Reading message from the wire: {err:#?}!"),
                Ok(msg) => {
                    if let Err(err) = self.handle_msg(msg) {
                        warn!("Error handling message message: {err:?}")
                    }
                }
            }
        }
    }

    pub fn create_pool(
        &mut self,
        size: i32,
    ) -> Result<(WlShmPool, SharedBuffer), Error> {
        assert!(size > 0);

        // TODO: remove object if error occurrs
        let shm: WlShm = self.get_global().expect("Failed to get global WlShm");
        let pool: WlShmPool = self.new_global();
        let buffer = SharedBuffer::alloc(size as usize)?;

        shm.create_pool(&pool, buffer.as_file_descriptor(), size)?;

        // TODO: find another way to save the buffer
        Ok((pool, buffer))
    }

    fn init_globals(&mut self) -> Result<(), Error> {
        let display: WlDisplay = self.new_global();
        assert!(display.get_object_id() == 1);

        self.add_event_handler(&display, |client, msg| match msg.event {
            WlDisplayEvent::Error {
                object_id,
                code,
                message,
            } => error!("Wayland error {code} for object {object_id}: {message:?}"), // TODO: add more context c: (display the object interface)
            WlDisplayEvent::DeleteId { id} => {
                match client.objects.get_object_entry_copy(id) {
                    Some(obj) => {
                        log::debug!("Delecting object {id} @ {}", obj.interface.display_name);
                        client.objects.remove_object(id);
                    }
                    None => log::error!("Received delete for a non existant object {id}")
                }
            }
        })?;

        let registry : WlRegistry = self.new_global();
        display.get_registry(&registry)?;

        self.add_event_handler(&registry, |client, msg| {
            let WlRegistryEvent::Global {
                name,
                interface,
                version,
            } = msg.event;

            let object_id = match interface.as_str() {
                "wl_compositor" => client.new_global_id::<WlCompositor, _>(),
                "xdg_wm_base" => client.new_global_id::<XdgWmBase, _>(),
                "wl_shm" => client.new_global_id::<WlShm, _>(),
                _ => return,
            };

            info!("Mapping {interface} to global");
            let registry: WlRegistry = client.get_global().unwrap();
            registry.bind(name, interface, version, object_id).unwrap(); // TODO: add proper error message
        })?;

        let callback : WlCallBack = self.new_object();
        display.sync(&callback)?;

        let completed = Rc::new(RefCell::new(false));
        {
            let flag = Rc::clone(&completed);
            self.add_event_handler(&callback, move |_, _| {
                *flag.borrow_mut() = true;
            }).unwrap();
        }

        while !*completed.borrow() {
            let msg = self.next_msg()?;
            self.handle_msg(msg)?;
        }

        Ok(())
    }

    #[inline(always)]
    fn new_global_id<T: WlInterface<Event = E>, E: Sized + 'static>(&mut self) -> WaylandId {
        self.new_global::<E, T>().get_object_id()
    }

    fn new_global<E: Sized + 'static, T: WlInterface<Event = E>>(&mut self) -> T {
        let object: T = self.new_object();
        assert!(
            self.globals
                .insert(T::get_interface_id(), object.get_object_id())
                .is_none(),
            "Creating global twice :c"
        );
        object
    }

    pub fn new_object<E: Sized + 'static, T: WlInterface<Event = E>>(&mut self) -> T {
        let object_id = self.objects.new_object(
            WlObjectInterfaceInfo {
                id: T::get_interface_id(),
                display_name: T::get_display_name()
            },
            Box::new(|raw_msg| T::parse_msg(raw_msg).map(|value| value.to_any())),
        );

        T::build(object_id, self.stream.clone())
    }

    fn handle_msg(&mut self, msg: RawMessage) -> Result<(), Error> {
        let object_id = msg.object_id;
        let event_id = msg.event_id;
        
        let mut entry = self.objects.get_object_entry_copy(msg.object_id).ok_or_else(|| {
            fallback_error!("Received message to an non-existant object: {object_id}")
        })?;

        let interface_id = entry.interface.id;
        let display_name = entry.interface.display_name;

        // TODO: add a few methods for the type entry c: (like parse and take c:)
        let msg = error_context!(
            (entry.parser)(msg), "Of object {object_id} @ {display_name}"
        )?;
        let mut handler = entry.handler.take().ok_or_else(|| {
            fallback_error!("No handler for event {event_id} of object {object_id} @ {display_name}")
        })?;

        handler(self, msg);
        let _ = self.objects.add_handler(object_id, interface_id, handler); // try to restore handler
        Ok(())
    }

    fn next_msg(&mut self) -> Result<RawMessage, Error> {
        let header = WireMsgHeader::build(error_context!(
            self.read_bytes(WireMsgHeader::WIRE_SIZE),
            "Failed to read wire message header"
        )?);

        trace!(
            "Received a msg from {} with {} size",
            header.object_id,
            header.length
        );

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

    fn read_bytes(&mut self, size: usize) -> Result<&[u8], Error> {
        self.buffer.read_bytes(size, &mut self.socket)
    }
}

#[derive(Debug)]
pub enum WlHandlerRegistryError {
    NoSuchObject,
    HandlerAlreadlyInPlace,
    InvalidInterface,
}

impl std::fmt::Display for WlHandlerRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

type MockingHandler<'a> = Box<dyn FnMut(&mut WaylandClient, Box<dyn Any>) + 'a>;
type WlParserWrapper = dyn Fn(RawMessage) -> Result<Box<dyn Any>, WlEventParseError>;

struct WlObjectEntry<'a> {
    interface : WlObjectInterfaceInfo,
    event_parser: Box<WlParserWrapper>,
    handler: Option<MockingHandler<'a>>,
}

struct WlObjectManager<'a> {
    objects: HashMap<WaylandId, WlObjectEntry<'a>>,
    objects_id_count: u32,
}

struct WlObjectEntryCopy<'a, 'b> {
    interface : WlObjectInterfaceInfo,
    handler: Option<MockingHandler<'a>>,
    parser: &'b WlParserWrapper,
}

#[derive(Copy, Clone)]
struct WlObjectInterfaceInfo {
    id : u32,
    display_name : &'static str
}

impl<'a> WlObjectManager<'a> {
    fn new() -> Self {
        Self {
            objects: HashMap::new(),
            objects_id_count: 0,
        }
    }

    fn new_object(
        &mut self,
        interface : WlObjectInterfaceInfo,
        event_parser: Box<WlParserWrapper>,
    ) -> WaylandId {
        self.objects_id_count += 1;
        assert!(self.objects_id_count < WaylandId::MAX); // FIXME: use the right upper bound
        assert!(self
            .objects
            .insert(
                self.objects_id_count,
                WlObjectEntry {
                    interface,
                    event_parser,
                    handler: None
                }
            )
            .is_none());
        self.objects_id_count
    }

    fn add_handler(
        &mut self,
        object_id: WaylandId,
        interface_id: WlInterfaceId,
        handler: MockingHandler<'a>,
    ) -> Result<(), WlHandlerRegistryError> {
        let entry = self
            .objects
            .get_mut(&object_id)
            .ok_or(WlHandlerRegistryError::NoSuchObject)?;

        if entry.interface.id != interface_id {
            Err(WlHandlerRegistryError::InvalidInterface)
        } else if entry.handler.is_some() {
            Err(WlHandlerRegistryError::HandlerAlreadlyInPlace)
        } else {
            entry.handler = Some(handler);
            Ok(())
        }
    }

    fn get_object_interface(&self, object_id: WaylandId) -> Option<WlInterfaceId> {
        self.objects.get(&object_id).map(|e| e.interface.id)
    }

    fn get_object_entry_copy<'b>(
        &'b mut self,
        object_id: WaylandId,
    ) -> Option<WlObjectEntryCopy<'a, 'b>> {
        let entry = self.objects.get_mut(&object_id)?;

        Some(WlObjectEntryCopy {
            handler: entry.handler.take(),
            parser: entry.event_parser.as_ref(),
            interface: entry.interface
        })
    }

    fn remove_object(&mut self, object_id: WaylandId) {
        self.objects.remove(&object_id);
    }
}

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

    pub fn read_bytes(&mut self, bytes: usize, stream: &mut impl Read) -> Result<&[u8], Error> {
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
