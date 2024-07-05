use std::{io::Write, iter};

use color_eyre::eyre::{eyre, Context};
use log::debug;

#[derive(Clone, Copy, Debug)]
pub enum MsgArgType {
    Uint32,
    String,
}

#[derive(Debug)]
pub enum MsgArgValue {
    Uint32(u32),
    Int32(i32),
    String(String),
}

// returns the size aligned to 4 bytes (means 32 bits)
fn str_aligned_size(base_size : usize) -> usize {
    ((base_size + 4 - 1) / 4) * 4
}

impl MsgArgValue {
    pub fn into_str(self) -> String {
        match self {
            MsgArgValue::String(val) => val,
            x => panic!("Expected String arg value  but found {x:?}"),
        }
    }


    pub fn into_i32(self) -> i32 {
        match self {
            MsgArgValue::Int32(val) => val,
            x => panic!("Expected i32 arg value but found {x:?}"),
        }
    }
    pub fn into_u32(self) -> u32 {
        match self {
            MsgArgValue::Uint32(val) => val,
            x => panic!("Expected u32 arg value but found {x:?}"),
        }
    }
}

pub struct Parser<'a>(&'a [MsgArgType]);
impl<'a> Parser<'a> {
    pub fn new(pattern: &'a [MsgArgType]) -> Self {
        Parser(pattern)
    }

    pub fn parse(&self, buffer: &[u8]) -> color_eyre::Result<impl Iterator<Item = MsgArgValue>> {
        if buffer.len() < self.0.len() * 4 {
            return Err(eyre!(
                "Buffer size {} not enough for all {} elements",
                buffer.len(),
                self.0.len()
            ));
        }

        let mut values = Vec::with_capacity(self.0.len());
        let mut consumed = 0usize;
        for (i, arg) in self.0.iter().enumerate() {
            let diff = buffer.len() - consumed;
            if diff < 4 {
                return Err(eyre!(
                    "Buffer doesn't have enough space. Couldn't parse arg {i}."
                ));
            }

            let value = u32::from_ne_bytes(buffer[consumed..consumed + 4].try_into().unwrap());
            consumed += 4;
            let buf = &buffer[consumed..];
            values.push(match arg {
                MsgArgType::Uint32 => MsgArgValue::Uint32(value),
                MsgArgType::String => {
                    let str_size = value as usize;

                    if buf.len() < str_size {
                        return Err(eyre!(
                            "Buffer doesn't have enough space. Couldn't parse arg {i}."
                        ));
                    }

                    let message = std::str::from_utf8(&buf[..str_size - 1])
                        .wrap_err("parsing request string")?;

                    consumed += str_aligned_size(str_size); //((str_size - 1 + 4) / 4) * 4;
                    MsgArgValue::String(String::from(message))
                }
            });
        }

        // for value
        Ok(values.into_iter())
    }
}

pub fn write_bytes(values: &[MsgArgValue], writer: &mut impl Write) -> std::io::Result<()> {
    for value in values {
        match value {
            MsgArgValue::Uint32(val) => {
                writer.write_all(&val.to_ne_bytes()).unwrap()
            }
            MsgArgValue::Int32(val) => {
                writer.write_all(&val.to_ne_bytes()).unwrap()
            }
            MsgArgValue::String(val) => {
                let bytes = val.as_bytes();
                let size = 1 + bytes.len() as u32; // +1 because of the 'null terminator'
                writer.write_all(&size.to_ne_bytes()).unwrap();
                writer.write_all(bytes).unwrap();
                writer.write_all(&[0;1]).unwrap(); // the 'null terminator'

                let size = size as usize;
                let padding = str_aligned_size(size) - size;

                let buf : Vec<u8> = iter::repeat(0u8).take(padding).collect();
                assert!(buf.len() == padding);
                writer.write_all(&buf).unwrap();
            }
        }
    }

    Ok(())
}
