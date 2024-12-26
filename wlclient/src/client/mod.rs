pub mod memory;

use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    env,
    io::Read,
    os::{fd::AsRawFd, unix::net::UnixStream},
    rc::Rc,
};

use crate::{
    error::{error_context, fallback_error, Error, Result},
    protocol::{base::*, xdg_shell::*, *},
};

use log::{error, info, trace, warn};
use memory::SharedBuffer;

pub struct WaylandClient<S = ()> {
    globals: HashMap<WlInterfaceId, WaylandId>,
    objects: WlObjectManager<S>,
    stream: Rc<ClientStream>,
    socket: UnixStream,
    buffer: ByteBuffer,
    state: Option<S>,
}

// TODO:
// - Think about if you really want to keep the lifetime
// - Rethink about interface to interact with the state
// - Should handler function return `Result<(), Error>`??
impl<S> WaylandClient<S> {
    pub fn connect() -> Result<Self> {
        // TODO: should I add: 'Failed to build from default'?
        Self::connect_to(&get_wayland_socket_path()?)
    }

    pub fn connect_to(socket_path: &str) -> Result<Self> {
        log::debug!("Connecting to socket_path = {socket_path}");
        let socket = error_context!(
            UnixStream::connect(socket_path),
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
            state: None,
        };

        client.init_globals()?;
        Ok(client)
    }

    pub fn get_custom_state(&mut self) -> Option<&mut S> {
        self.state.as_mut()
    }

    pub fn set_custom_state(&mut self, state: S) {
        self.state = Some(state)
    }

    pub fn get_global<T: WlInterface<Event = E>, E>(&self) -> Option<T> {
        self.get_reference(self.globals.get(&T::get_interface_id()).copied()?)
    }

    // TODO: use actual enumerate for errors
    pub fn upgrade_to_global<T: WlInterface<Event = E>, E>(&mut self, object: &T) -> Result<()> {
        let object_id = object.get_object_id();
        let Some(interface_id) = self.objects.get_object_interface(object_id) else {
            return Err(fallback_error!("No such object"));
        };

        if interface_id != T::get_interface_id() {
            return Err(fallback_error!("Invalid object interface"));
        }

        // TODO: think about this c:
        if self.globals.contains_key(&interface_id) {
            return Err(fallback_error!(
                "Global of '{}' already registered",
                T::get_display_name()
            ));
        }

        let _ = self.globals.insert(interface_id, object_id);
        Ok(())
    }

    pub fn get_reference<T: WlInterface<Event = E>, E>(&self, object_id: u32) -> Option<T> {
        match self.objects.get_object_interface(object_id) {
            Some(interface_id) if interface_id == T::get_interface_id() => {
                Some(T::build(object_id, self.stream.clone()))
            }
            _ => None,
        }
    }

    pub fn add_event_handler<T, E, F>(&mut self, object: &T, mut handler: F) -> Result<()>
    where
        T: WlInterface<Event = E>,
        F: FnMut(&mut WaylandClient<S>, WlEventMsg<E>) + 'static,
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

    pub fn remove_event_handler<T: WlInterface<Event = E>, E>(&mut self, object: &T) -> Result<()> {
        self.objects.remove_handler(object.get_object_id())
    }

    pub fn event_loop(mut self) {
        loop {
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

    pub fn create_pool(&mut self, size: i32) -> Result<(WlShmPool, SharedBuffer)> {
        assert!(size > 0);

        // TODO: remove object if error occurrs
        // TODO: remove expect and actually return an error
        let shm: WlShm = self.get_global().expect("Failed to get global WlShm");
        let pool: WlShmPool = self.new_object();
        let (buffer, file) = SharedBuffer::alloc(size as usize)?;

        shm.create_pool(&pool, file.as_raw_fd(), size)?;

        // TODO: find another way to save the buffer
        Ok((pool, buffer))
    }

    fn init_globals(&mut self) -> Result<()> {
        let display: WlDisplay = self.new_global();
        assert!(display.get_object_id() == 1);

        self.add_event_handler(&display, |client, msg| match msg.event {
            WlDisplayEvent::Error {
                object_id,
                code,
                message,
            } => error!("Wayland error {code} for object {object_id}: {message:?}"), // TODO: add more context c: (display the object interface)
            WlDisplayEvent::DeleteId { id } => match client.objects.get_object_entry_copy(id) {
                Some(obj) => {
                    log::debug!("Delecting object {id} @ {}", obj.interface.display_name);
                    client.objects.remove_object(id);
                }
                None => log::error!("Received delete for a non existant object {id}"),
            },
        })?;

        let registry: WlRegistry = self.new_global();
        display.get_registry(&registry)?;

        self.add_event_handler(&registry, |client, msg| {
            let WlRegistryEvent::Global {
                name,
                interface,
                version,
            } = msg.event;

            let object_id = match interface.as_str() {
                "wl_compositor" => client.new_global_id::<WlCompositor, _>(),
                "xdg_wm_base" => {
                    let wm: XdgWmBase = client.new_global();
                    client
                        .add_event_handler(&wm, |client, msg| {
                            let wm: XdgWmBase = client.get_reference(msg.object_id).unwrap();
                            let XdgWmBaseEvent::Ping { serial } = msg.event;
                            wm.pong(serial).unwrap();
                        })
                        .unwrap();

                    wm.get_object_id()
                }
                "wl_shm" => {
                    let shm: WlShm = client.new_global();

                    // Ignore shm messages, I don't really care about those, since Argb and Xrgb
                    // are mandatory to be implemented by the server, so I am cool c:
                    // This is to surpress the warning messages: 'No handler for object ...'
                    client.add_event_handler(&shm, |_, _| {}).unwrap();
                    shm.get_object_id()
                }
                _ => return,
            };

            info!("Mapping {interface} to global");
            let registry: WlRegistry = client.get_global().unwrap();
            registry.bind(name, interface, version, object_id).unwrap(); // TODO: add proper error message
        })?;

        let callback: WlCallBack = self.new_object();
        display.sync(&callback)?;

        let completed = Rc::new(RefCell::new(false));
        {
            let flag = Rc::clone(&completed);
            self.add_event_handler(&callback, move |_, _| {
                *flag.borrow_mut() = true;
            })
            .unwrap();
        }

        while !*completed.borrow() {
            let msg = self.next_msg()?;
            self.handle_msg(msg)?;
        }

        Ok(())
    }

    #[inline(always)]
    fn new_global_id<T: WlInterface<Event = E>, E: Sized + 'static>(&mut self) -> WaylandId {
        self.new_global::<T, E>().get_object_id()
    }

    fn new_global<T: WlInterface<Event = E>, E: Sized + 'static>(&mut self) -> T {
        let object: T = self.new_object();
        assert!(
            self.globals
                .insert(T::get_interface_id(), object.get_object_id())
                .is_none(),
            "Creating global twice :c"
        );
        object
    }

    pub fn new_object<T: WlInterface<Event = E>, E: Sized + 'static>(&mut self) -> T {
        let object_id = self.objects.new_object(
            WlObjectInterfaceInfo {
                id: T::get_interface_id(),
                display_name: T::get_display_name(),
            },
            Box::new(|raw_msg| Ok(T::parse_msg(raw_msg).map(|value| value.to_any())?)),
        );

        T::build(object_id, self.stream.clone())
    }

    fn handle_msg(&mut self, msg: RawMessage) -> Result<()> {
        let object_id = msg.object_id;
        let event_id = msg.event_id;

        let mut entry = self
            .objects
            .get_object_entry_copy(msg.object_id)
            .ok_or_else(|| fallback_error!("Received message to an non-existant object: {object_id}"))?;

        let interface_id = entry.interface.id;
        let display_name = entry.interface.display_name;

        // TODO: add a few methods for the type entry c: (like parse and take c:)
        let msg = error_context!((entry.parser)(msg), "Of object {object_id} @ {display_name}")?;
        let mut handler = entry.handler.take().ok_or_else(|| {
            fallback_error!("No handler object {object_id} @ {display_name} (received event {event_id})")
        })?;

        handler(self, msg);
        let _ = self.objects.add_handler(object_id, interface_id, handler); // try to restore handler
        Ok(())
    }

    fn next_msg(&mut self) -> Result<RawMessage> {
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

    fn read_bytes(&mut self, size: usize) -> Result<&[u8]> {
        self.buffer.read_bytes(size, &mut self.socket)
    }
}

// TODO: follow the atual protocol
pub fn get_wayland_socket_path() -> Result<String> {
    let xdg_dir = error_context!(env::var("XDG_RUNTIME_DIR"), "Failed to get XDG_RUNTIME_DIR var")?;
    let socket_file = error_context!(env::var("WAYLAND_DISPLAY"), "Failed to get WAYLAND_DISPLAY var")?;
    Ok(format!("{xdg_dir}/{socket_file}"))
}

type MockingHandler<S> = Box<dyn FnMut(&mut WaylandClient<S>, Box<dyn Any>) + 'static>;
type WlParserWrapper = dyn Fn(RawMessage) -> Result<Box<dyn Any>>;

struct WlObjectEntry<S> {
    interface: WlObjectInterfaceInfo,
    event_parser: Box<WlParserWrapper>,
    handler: Option<MockingHandler<S>>,
}

struct WlObjectManager<S> {
    objects: HashMap<WaylandId, WlObjectEntry<S>>,
    objects_id_count: u32,
}

struct WlObjectEntryCopy<'a, S> {
    interface: WlObjectInterfaceInfo,
    handler: Option<MockingHandler<S>>,
    parser: &'a WlParserWrapper,
}

#[derive(Copy, Clone)]
struct WlObjectInterfaceInfo {
    id: u32,
    display_name: &'static str,
}

impl<S> WlObjectManager<S> {
    fn new() -> Self {
        Self {
            objects: HashMap::new(),
            objects_id_count: 0,
        }
    }

    fn new_object(
        &mut self,
        interface: WlObjectInterfaceInfo,
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

    // If it returns an error, it will be: WlHandlerRegistryError::NoSuchObject
    fn remove_handler(&mut self, object_id: WaylandId) -> Result<()> {
        let entry = self.objects.get_mut(&object_id).ok_or(Error::NoSuchObject)?;
        entry.handler = None;
        Ok(())
    }

    fn add_handler(
        &mut self,
        object_id: WaylandId,
        interface_id: WlInterfaceId,
        handler: MockingHandler<S>,
    ) -> Result<()> {
        let entry = self.objects.get_mut(&object_id).ok_or(Error::NoSuchObject)?;

        if entry.interface.id != interface_id {
            Err(Error::InvalidInterface)
        } else if entry.handler.is_some() {
            Err(Error::HandlerAlreadlyInPlace)
        } else {
            entry.handler = Some(handler);
            Ok(())
        }
    }

    fn get_object_interface(&self, object_id: WaylandId) -> Option<WlInterfaceId> {
        self.objects.get(&object_id).map(|e| e.interface.id)
    }

    fn get_object_entry_copy(
        &mut self,
        object_id: WaylandId,
    ) -> Option<WlObjectEntryCopy<'_, S>> {
        let entry = self.objects.get_mut(&object_id)?;

        Some(WlObjectEntryCopy {
            handler: entry.handler.take(),
            parser: entry.event_parser.as_ref(),
            interface: entry.interface,
        })
    }

    fn remove_object(&mut self, object_id: WaylandId) {
        self.objects.remove(&object_id);
    }
}

// TODO: you probably want to delete this struct or something
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

//#[cfg(test)]
//mod test {}
