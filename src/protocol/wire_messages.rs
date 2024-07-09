use color_eyre::eyre::eyre;
use log::{debug, trace};
use std::io::Write;

use super::api::WaylandRequest;

#[derive(Debug)]
pub struct MessageHeader {
    pub object_id: u32,
    pub length: u16,
    pub method_id: u16,
}

pub const HEADER_SIZE: usize = 4 + 2 + 2;
pub fn make_request(
    sock: &mut impl Write,
    object_id: u32,
    request: WaylandRequest,
) -> color_eyre::Result<()> {
    trace!("Serializing {:?} to {}", request, object_id);
    let mut buffer = Vec::new();

    let request_id = request.request_id();
    buffer.write_all(&object_id.to_ne_bytes())?;
    buffer.write_all(&0u32.to_ne_bytes())?;
    request.write_data(&mut buffer)?;

    let size = buffer.len();
    if size % 4 != 0 || size > u16::MAX as usize {
        return Err(eyre!("Data size {} missaligned or overflowed", size));
    }

    let size_and_event_id = (size as u32) << 16 | request_id as u32;
    let bytes = size_and_event_id.to_ne_bytes();
    buffer[4..8].copy_from_slice(&bytes);

    sock.write_all(&buffer)?;
    debug!("buffer = {:?}", buffer);
    Ok(())
}

pub fn read_header(buf: &[u8]) -> MessageHeader {
    let object_id = u32::from_ne_bytes(buf[..4].try_into().unwrap());
    let len_and_event_id = u32::from_ne_bytes(buf[4..8].try_into().unwrap());

    MessageHeader {
        object_id,
        method_id: (len_and_event_id & 0xFFFF) as u16,
        length: (len_and_event_id >> 16) as u16,
    }
}
