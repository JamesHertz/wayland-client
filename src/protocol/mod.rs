pub mod api;
pub mod parser;
mod messages;

use color_eyre::eyre::eyre;
use log::{debug, info};
use std::{collections::HashMap, io::Read, os::unix::net::UnixStream};
use self::api::{ObjectType, WaylandEvent, WaylandEventMessage, WaylandRequest};

pub struct WaylandClient {
    socket: UnixStream,
    buffer: Box<[u8]>,
    ids: HashMap<u32, ObjectType>,
    globals: HashMap<ObjectType, u32>,
    head: usize,
    tail: usize,
    ids_count: u32,
}

impl WaylandClient {
    pub fn connect(compositor_sock_path: &str) -> std::io::Result<Self> {
        let globals = HashMap::from([(ObjectType::Display, 1)]);

        Ok(Self {
            socket: UnixStream::connect(compositor_sock_path)?,
            buffer: Box::new([0; 1 << 12]), // allocates 4 kilo bytes c:
            ids: globals.iter().map(|(&k, &v)| (v, k)).collect(),
            head: 0,
            tail: 0,
            ids_count: 2,
            globals,
        })
    }

    pub fn map_global(&mut self, id: u32, obj_type: ObjectType) {
        assert!(self.globals.insert(obj_type, id).is_none());
    }

    fn get_global(&mut self, obj_type: ObjectType) -> Option<u32> {
        self.globals.get(&obj_type).copied()
    }

    pub fn new_id(&mut self, obj_type: ObjectType) -> u32 {
        let id = self.ids_count;
        self.ids_count += 1;
        self.ids.insert(id, obj_type);

        if let ObjectType::Registry = obj_type {
            self.map_global(id, obj_type);
        }
        id
    }

    pub fn send_request(
        &mut self,
        object_id: u32,
        request: WaylandRequest,
    ) -> color_eyre::Result<()> {
        messages::make_request(&mut self.socket, object_id, request)
    }

    pub fn load_interfaces(&mut self) -> color_eyre::Result<()> {
        let display_id = self.get_global(ObjectType::Display).unwrap();
        let registry_id = self.new_id(ObjectType::Registry);
        self.send_request(display_id, WaylandRequest::DisplayGetRegistry(registry_id))?;

        let callback_id = self.new_id(ObjectType::CallBack);
        self.send_request(display_id, WaylandRequest::DisplaySync(callback_id))?;

        loop {
            let event_msg = self.next_event()?;
            match event_msg.event {
                WaylandEvent::DisplayError { message, .. } => {
                    return Err(eyre!("DisplayError: {message}"));
                }
                WaylandEvent::RegistryGlobal {
                    name,
                    interface,
                    version,
                } => {
                    match &interface.parse::<ObjectType>() {
                        Err(e) => debug!("Error parsing {interface}@{version} {e}"),
                        Ok(value) => {
                            info!("Mapped into global: {name} -> {interface}@{version}");
                            self.map_global( name, *value)
                        }
                    }
                }
                WaylandEvent::CallBackDone(_) => {
                    info!("Done!");
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn next_event(&mut self) -> color_eyre::Result<WaylandEventMessage> {
        let header = messages::read_header(
            self.read_bytes(messages::HEADER_SIZE)?
                .expect("Couldn't read header :c"),
        );

        let object_typ = *self
            .ids
            .get(&header.object_id)
            .expect("Object doesn't exist");

        debug!(
            "Received op = {} from = {} for obj = {:?}",
            header.method_id, header.object_id, object_typ
        );

        let event = object_typ.parse_event(
            header.method_id,
            self.read_bytes(header.length as usize - messages::HEADER_SIZE)?
                .expect("Couldn't read event body"),
        )?;

        debug!("{event:?}");

        if let ObjectType::CallBack = object_typ {
            self.ids.remove(&header.object_id);
        }

        Ok(WaylandEventMessage {
            sender_id: header.object_id,
            sender_obj: object_typ,
            event,
        })
    }

    fn read_bytes(&mut self, bytes: usize) -> color_eyre::Result<Option<&[u8]>> {
        assert!(bytes < 1 << 11);
        let cached_bytes = self.tail - self.head;
        assert!(cached_bytes == 0 || cached_bytes < 2 << 14);
        if bytes <= cached_bytes {
            let res = &self.buffer[self.head..self.head + bytes];
            self.head += bytes;
            return Ok(Some(res));
        }

        let left_space = self.buffer.len() - self.tail;
        if cached_bytes + left_space < bytes {
            debug!("Doing moves ...");
            self.buffer.copy_within(self.head..self.tail, 0);
            self.head = 0;
            self.tail = cached_bytes;
        }

        let size = self.socket.read(&mut self.buffer[self.tail..])?;
        self.tail += size;

        debug!("new request of {size} bytes");

        if size + cached_bytes < bytes {
            return Ok(None);
        }

        let res = &self.buffer[self.head..self.head + bytes];
        self.head += bytes;
        Ok(Some(res))
    }
}
