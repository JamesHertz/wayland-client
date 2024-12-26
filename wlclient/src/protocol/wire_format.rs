use super::*;
use std::{
    cell::RefCell,
    io::{IoSlice, Write},
    os::unix::net::{SocketAncillary, UnixStream},
};

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


pub struct ClientStream(RefCell<UnixStream>);
impl ClientStream {
    pub fn new(stream: UnixStream) -> Self {
        Self(RefCell::new(stream))
    }
}

impl WaylandStream for ClientStream {
    fn send(&self, msg: WireMessage<'_>) -> Result<usize> {
        // 128 bytes seems to be a good default. For what I've seen I think that
        // hardly any request will need so many bytes.
        let mut buffer = Vec::with_capacity(128);

        write_u32(&mut buffer, msg.object_id);
        write_u32(&mut buffer, 0); // will be filled in the end

        let mut file_desc: Option<i32> = None;

        for value in msg.values {
            match value {
                Uint32(value) => write_u32(&mut buffer, *value),
                Int32(value) => write_u32(&mut buffer, *value as u32),
                Str(value) => {
                    let bytes = value.as_bytes();
                    let str_size = 1 + bytes.len() as u32; // +1 because of the 'null terminator'
                    write_u32(&mut buffer, str_size);
                    write_bytes(&mut buffer, bytes);
                    write_bytes(&mut buffer, &[0; 1]); // the string null terminator

                    let str_size = str_size as usize;
                    let aligned_size = str_aligned_size(str_size);

                    // buffer.resize(aligned_size, 0u8)
                    for _ in 0..(aligned_size - str_size) {
                        buffer.push(0u8);
                    }
                }
                FileDesc(fd) => {
                    assert!(file_desc.is_none());
                    file_desc = Some(*fd);
                }
                // Array(Vec<u8>),
                other => {
                    todo!("Implement serialization for {other:?}")
                }
            }
        }

        let total_size = buffer.len();
        if total_size % 4 != 0 {
            panic!("Bug, the total size ({total_size}) of message {msg:?} isn't a multiple of 32 bits")
        }

        let size_and_event_id = (total_size as u32) << 16 | msg.request_id;
        //(total_size as u32) << 16 | msg.request_id as u32;
        let bytes = size_and_event_id.to_ne_bytes();
        buffer[4..8].copy_from_slice(&bytes);

        let size = match file_desc {
            Some(fd) => {
                // 32 is s total random number, I think I only need 4 but I am not sure
                // TODO: do research on this ...
                let mut ancillary_buffer = [0; 32];
                let mut ancillary = SocketAncillary::new(&mut ancillary_buffer[..]);
                ancillary.add_fds(&[fd][..]);

                self.0
                    .borrow_mut()
                    .send_vectored_with_ancillary(&[IoSlice::new(&buffer)][..], &mut ancillary)?
            }
            None => self.0.borrow_mut().write(&buffer)?,
        };

        Ok(size)
    }
}

// helper functions
fn u32_from_bytes(data: &[u8]) -> u32 {
    assert!(data.len() == 4);
    u32::from_ne_bytes(data.try_into().unwrap())
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

// parsing helper functions
pub mod parsing {
    use super::{str_aligned_size, u32_from_bytes};
    use std::str;


    //use super::*;
    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Debug)]
    pub enum Error {
        MissingField(&'static str),
        InvalidUtfString(str::Utf8Error),
        InvalidArrayByteSize(u32), // array byte size should be a mutiple of 32 bits
        CustomError(String)
    }

    impl From<str::Utf8Error> for Error {
        fn from(value: str::Utf8Error) -> Self {
            Self::InvalidUtfString(value)
        }
    }

    macro_rules! parse_field {
        ($value : expr, $field_desc : tt) => {
            $value.map_err(|_| Error::MissingField($field_desc))
        };
    }

    macro_rules! custom_err {
        ($($t : tt)+) => {
            Error::CustomError(format!($($t)+))
        };
    }

    pub fn parse_u32(iter: &mut impl Iterator<Item = u8>) -> Result<u32> {
        let bytes = parse_field!(
            iter.next_chunk::<4>(), "4 bytes for u32/i32 integer"
        )?;

        Ok(u32_from_bytes(&bytes))
    }

    pub fn parse_i32(iter: &mut impl Iterator<Item = u8>) -> Result<i32> {
        Ok(parse_u32(iter)? as i32)
    }

    pub fn parse_u32_array(iter: &mut impl Iterator<Item = u8>) -> Result<Vec<u32>> {
        let size = parse_field!(parse_u32(iter), "u32 array size")?;

        // FIXME: remove this condition and start checking if all the messages/events
        // payload size (in bytes) are multiple or 32 bits
        if size % 4 != 0 {
            return Err(Error::InvalidArrayByteSize(size));
        };

        let array_size = size / 4;
        let mut array = Vec::with_capacity(array_size as usize);
        for i in 0..array_size {
            let elem = parse_u32(iter).map_err(
                |_| custom_err!("Failed to get all {array_size} elements of array, only gotten = {i}")
            )?;
            array.push(elem);
        }

        Ok(array)
    }

    pub fn parse_str(iter: &mut impl Iterator<Item = u8>) -> Result<String> {
        let str_size = parse_field!(parse_u32(iter), "Failed to get String size.")? as usize;

        let str_data = next_n_bytes(iter, str_size)
            .ok_or(custom_err!("Failed to get {str_size} bytes for str data."))?;

        let result = str::from_utf8(&str_data[..str_size - 1])?;

        let padding = str_aligned_size(str_size) - str_size;
        iter.advance_by(padding)
            .map_err(|_| custom_err!("Failed to get the {padding} padding bytes!"))?;

        Ok(result.to_string())
    }

    fn next_n_bytes(iter: &mut impl Iterator<Item = u8>, items: usize) -> Option<Vec<u8>> {
        let mut values = Vec::with_capacity(items);

        for _ in 0..items {
            values.push(iter.next()?)
        }

        Some(values)
    }
}
