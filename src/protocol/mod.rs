#![allow(unused_imports)]
pub mod api;
mod wire_messages;
pub mod parser;

use self::api::{
    WaylandEvent, WaylandEventMessage, WaylandObject, WaylandRequest,
};
use log::{debug, error, info, warn};
use std::{
    borrow::BorrowMut,
    collections::HashMap,
    io::Read,
    iter,
    os::unix::net::UnixStream,
    sync::{
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

type CustomResult<T> = Result<T, WaylandClientError>;
struct ByteBuffer {
    data: Box<[u8]>,
    head: usize,
    tail: usize,
}

type Locked<T> = Arc<Mutex<T>>;
pub struct WaylandClient {
    socket: UnixStream,
    ids: Locked<HashMap<u32, WaylandObject>>,
    mapped: HashMap<WaylandObject, u32>,
    ids_count: u32,
    display_id: u32,
    channel: Receiver<WaylandEventMessage>,
}

impl ByteBuffer {
    fn new(size: usize) -> Self {
        ByteBuffer {
            head: 0,
            tail: 0,
            data: iter::repeat(0u8)
                .take(size)
                .collect::<Vec<u8>>()
                .into_boxed_slice(),
        }
    }

    fn cached_bytes(&self) -> usize {
        self.tail - self.head
    }

    fn tail_space(&self) -> usize {
        self.data.len() - self.tail
    }

    fn read_bytes(
        &mut self,
        bytes: usize,
        reader: &mut impl Read,
    ) -> std::io::Result<Option<&[u8]>> {
        assert!(bytes < 1 << 11);
        let cached_bytes = self.cached_bytes();
        assert!(cached_bytes == 0 || cached_bytes < 2 << 14);
        if bytes <= cached_bytes {
            let res = &self.data[self.head..self.head + bytes];
            self.head += bytes;
            return Ok(Some(res));
        }

        let left_space = self.tail_space();
        if cached_bytes + left_space < bytes {
            debug!("Doing moves ...");
            self.data.copy_within(self.head..self.tail, 0);
            self.head = 0;
            self.tail = cached_bytes;
        }

        let size = reader.read(&mut self.data[self.tail..])?;
        self.tail += size;

        debug!("new request of {size} bytes");

        if size + cached_bytes < bytes {
            return Ok(None);
        }

        let res = &self.data[self.head..self.head + bytes];
        self.head += bytes;
        Ok(Some(res))
    }
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

        let (sender, receiver) = mpsc::channel::<WaylandEventMessage>();
        let socket = UnixStream::connect(compositor_sock_path)?;

        let rcv_socket = socket.try_clone().unwrap();
        let ids_cpy = Arc::clone(&ids);
        thread::spawn(move || background_thread(sender, rcv_socket, ids_cpy));

        Ok(Self {
            socket,
            ids,
            display_id: mapped.get(&WaylandObject::Display).copied().unwrap(),
            ids_count: 2,
            channel: receiver,
            mapped,
        })
    }

    // TODO: I dont quite understand this very well, so you should come ack to this
    pub fn map_global(
        &mut self,
        name: u32,
        interface: &str,
        version : u32,
        object : WaylandObject,
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
                        self.map_global(name, &interface, version,  object)?
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

    pub fn pull_messages(&mut self) -> color_eyre::Result<Vec<WaylandEventMessage>> {
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

fn background_thread(
    channel: Sender<WaylandEventMessage>,
    mut stream: UnixStream,
    ids: Locked<HashMap<u32, WaylandObject>>,
) {
    let mut buffer = ByteBuffer::new(1 << 12);
    loop {
        let msg = wait_for_msg(&mut buffer, &mut stream, &ids)
            .expect("Error waiting for next message");
        match msg.event {
            WaylandEvent::DisplayError { message, .. } => {
                panic!("Display error {message}");
            }
            WaylandEvent::DisplayDelete(item) => {
                warn!("Received delecting msg for {item}");
                ids.lock().unwrap().remove(&item);
            }
            _ => channel.send(msg).expect("sending event"),
        }
    }
}

fn wait_for_msg(
    buffer: &mut ByteBuffer,
    stream: &mut UnixStream,
    ids: &Locked<HashMap<u32, WaylandObject>>,
) -> CustomResult<WaylandEventMessage> {
    let header_bytes = buffer.read_bytes(wire_messages::HEADER_SIZE, stream)?;

    if header_bytes.is_none() {
        return Err(WaylandClientError::NoMessage);
    }

    let header = wire_messages::read_header(header_bytes.unwrap());
    let object_typ = *ids
        .lock()
        .unwrap()
        .get(&header.object_id)
        .expect("Object doesn't exist");

    debug!(
        "Received op = {} from = {} for obj = {:?}",
        header.method_id, header.object_id, object_typ
    );

    let event = object_typ.parse_event(
        header.method_id,
        buffer
            .read_bytes(header.length as usize - wire_messages::HEADER_SIZE, stream)?
            .expect("Couldn't read event body"),
    )?;

    debug!("{event:?}");

    if let WaylandObject::CallBack = object_typ {
        ids.lock().unwrap().remove(&header.object_id);
    }

    Ok(WaylandEventMessage {
        sender_id: header.object_id,
        sender_obj: object_typ,
        event,
    })
}
