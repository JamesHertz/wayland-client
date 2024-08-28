use super::Shared;
use crate::{protocol::*, Result, Error};
use std::{ 
    io::Write,
    os::unix::net::UnixStream,
    sync::Arc,
    cell::RefCell,
    ops::Deref,
};

use log::debug;


#[derive(Debug)]
pub struct WireMsgHeader {
    pub object_id: u32,
    pub length: u16,
    pub method_id: u16,
}

impl WireMsgHeader {
    pub const WIRE_SIZE: usize = 4 + 2 + 2;

    pub fn build(data: &[u8]) -> Self {
        assert!(data.len() == Self::WIRE_SIZE);

        let object_id = u32_from_bytes(&data[..4]);
        let len_and_event_id = u32_from_bytes(&data[4..8]);

        Self {
            object_id,
            method_id: (len_and_event_id & 0xFFFF) as u16,
            length: (len_and_event_id >> 16) as u16,
        }
    }
}

fn u32_from_bytes(data: &[u8]) -> u32 {
    assert!(data.len() == 4);
    u32::from_ne_bytes(data.try_into().unwrap())
}

pub struct ClientStream(Shared<UnixStream>);
impl ClientStream {
    pub fn new(stream: Shared<UnixStream>) -> Self {
        Self(stream)
    }
}

impl WaylandStream for ClientStream {
    fn send(&self, msg: WireMessage<'_>) -> Result<usize> {
        debug!("Sending message {msg:#?}");
        let mut buffer = Vec::with_capacity(512);

        write_u32(&mut buffer, msg.object_id);
        write_u32(&mut buffer, 0); // will be filled in the end

        for value in msg.values {
            match value {
                Uint32(value) => write_u32(&mut buffer, *value),
                Int32(value)  => write_u32(&mut buffer, *value as u32),
                Str(value)    => {
                    let bytes = value.as_bytes();
                    let str_size = 1 + bytes.len() as u32; // +1 because of the 'null terminator'
                    write_u32(&mut buffer, str_size);
                    write_bytes(&mut buffer, bytes);
                    write_bytes(&mut buffer, &[0; 1]);

                    let str_size = str_size as usize;
                    let aligned_size = str_aligned_size(str_size);

                    for _ in 0..(aligned_size - str_size) {
                        buffer.push(0u8);
                    }
                }
                // Array(Vec<u8>),
                // FileDesc(i32),
                other => {
                    todo!("Implement serialization for {other:?}")
                }
            }
        }

        let total_size = buffer.len();
        if total_size % 4 != 0 {
            panic!(
                "Bug, the total size ({total_size}) of message {msg:?} isn't a multiple of 32 bits"
            )
        }

        let size_and_event_id = (total_size as u32) << 16 | msg.request_id as u32;
        let bytes = size_and_event_id.to_ne_bytes();
        buffer[4..8].copy_from_slice(&bytes);

        // let refcell: &RefCell<UnixStream> = self.0.deref();
        // let socket : &mut UnixStream = &mut refcell.borrow_mut();
        self.0.borrow_mut()
              .write(&buffer).map_err(Error::IoError)
    }
}

#[inline]
fn write_u32(writer: &mut impl Write, value: u32) {
    writer.write_all(&value.to_ne_bytes()).unwrap()
}

#[inline]
fn write_bytes(writer: &mut impl Write, data: &[u8]) {
    writer.write_all(data).unwrap()
}

#[inline]
fn str_aligned_size(base_size: usize) -> usize {
    ((base_size + 4 - 1) / 4) * 4
}
