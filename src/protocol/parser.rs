use std::io::Write;

use color_eyre::eyre::{eyre, Context};
use log::debug;

#[derive(Clone, Copy, Debug)]
pub enum MsgArgType {
    ObjectId,
    // NewId, // FIXME: handle this some day
    Uint32,
    String,
}

#[derive(Debug)]
pub enum MsgArgValue {
    ObjectId(u32),
    Uint32(u32),
    String(String),
}

impl MsgArgValue {
    pub fn into_str(self) -> String {
        match self {
            MsgArgValue::String(val) => val,
            x => panic!("Expected String arg value  but found {x:?}"),
        }
    }

    pub fn into_u32(self) -> u32 {
        match self {
            MsgArgValue::ObjectId(val) | MsgArgValue::Uint32(val) => val,
            x => panic!("Expected int arg value but found {x:?}"),
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
                MsgArgType::ObjectId => MsgArgValue::ObjectId(value),
                MsgArgType::String => {
                    let str_size = value as usize;

                    if buf.len() < str_size {
                        return Err(eyre!(
                            "Buffer doesn't have enough space. Couldn't parse arg {i}."
                        ));
                    }

                    let message = std::str::from_utf8(&buf[..str_size - 1])
                        .wrap_err("parsing request string")?;
                    debug!("str_size: {str_size}");

                    consumed += ((str_size - 1 + 4) / 4) * 4;
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
            MsgArgValue::ObjectId(val) | MsgArgValue::Uint32(val) => {
                writer.write_all(&val.to_ne_bytes()).unwrap()
            }
            _ => todo!(),
        }
    }

    Ok(())
}
