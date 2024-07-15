use std::{io::Write, iter};

use color_eyre::eyre::{eyre, Context};
use log::debug;

pub type Array = Vec<u32>;

#[derive(Clone, Copy, Debug)]
pub enum MsgArgType {
    Uint32,
    String,
    Int32,
    Array,
}

#[derive(Debug)]
pub enum MsgArgValue {
    Uint32(u32),
    Int32(i32),
    String(String),
    Array(Array),
}

// returns the size aligned to 4 bytes (means 32 bits)
fn str_aligned_size(base_size: usize) -> usize {
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

    pub fn into_array(self) -> Array {
        match self {
            MsgArgValue::Array(val) => val,
            x => panic!("Expected array arg value but found {x:?}"),
        }
    }
}

pub struct Parser<'a>(&'a [MsgArgType]);
impl<'a> Parser<'a> {
    pub fn new(pattern: &'a [MsgArgType]) -> Self {
        Parser(pattern)
    }

    pub fn parse(
        &self,
        buffer: &[u8],
    ) -> color_eyre::Result<impl Iterator<Item = MsgArgValue>> {
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

            let value = u32::from_ne_bytes(
                buffer[consumed..consumed + 4].try_into().unwrap(),
            );

            consumed += 4;
            let buf = &buffer[consumed..];
            values.push(match arg {
                MsgArgType::Uint32 => MsgArgValue::Uint32(value),
                MsgArgType::Int32  => MsgArgValue::Int32(value as i32),
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
                },
                MsgArgType::Array => {
                    let arr_size = value as usize;

                    if buf.len() != arr_size  && buf.len() % 4 == 0{
                        return Err(eyre!(
                            "Array has {arr_size} elements but size in bytes is {}. Couldn't parse arg {i}.",
                            buf.len()
                        ));
                    }

                    // consumed += arr_size;

                    todo!()
                    // consumed += arr_size * 4;
                    //
                    // MsgArgValue::Array(
                    //     buf.iter()
                    //
                    // )
                }
            });
        }

        // for value
        Ok(values.into_iter())
    }
}

pub fn write_bytes(
    values: &[MsgArgValue],
    writer: &mut impl Write,
) -> std::io::Result<()> {
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
                writer.write_all(&[0; 1]).unwrap(); // the 'null terminator'

                let size = size as usize;
                let padding = str_aligned_size(size) - size;

                let buf: Vec<u8> = iter::repeat(0u8).take(padding).collect();
                assert!(buf.len() == padding);
                writer.write_all(&buf).unwrap();
            }
            MsgArgValue::Array(arr) => {
                writer.write_all(&arr.len().to_ne_bytes()).unwrap();
                for value in arr {
                    writer.write_all(&value.to_ne_bytes()).unwrap();
                }
            }
        }
    }

    Ok(())
}

// macro_rules! unmarshall_event {
//
//     (@helper $init : expr, Uint32) => {
//         $init.into_u32()
//     };
//
//     (@helper $init : expr, String) => {
//         $init.into_str()
//     };
//
//     (@helper $init : expr, Int32) => {
//         $init.into_i32()
//     };
//
//     (@helper $init : expr, Array) => {
//         $init.into_array()
//     };
//
//     (@helper $init : expr, $x : ident) => {
//         compile_error!(concat!("Unexpected type: ", stringify!($x), ". I should be Int32, Uint32, String or Array."))
//     };
//
//     (@expand_results $buffer : ident, $($type : ident,)+) => {
//             Parser::new(&[
//                 $($crate::parser::MsgArgType::$type, )+
//             ]).parse($buffer)?
//     };
//
//     ($buffer : ident, $struct : ident ( $($type : ident)+ )) => {
//             {
//                 let mut results = $crate::parser::unmarshall_event!(
//                     @expand_results $buffer, $($type,)+
//                 );
//                 Ok($struct(
//                     $(unmarshall_event!(@helper results.next().unwrap(), $type ),)+
//                 ))
//             }
//     };
//
//     ($buffer : ident, $struct : path { $( $name : ident => $type : ident, )+ }) => {
//             {
//                 let mut results = $crate::protocol::parser::unmarshall_event!(
//                     @expand_results $buffer, $($type,)+
//                 );
//                 Ok($struct {
//                         $( $name : $crate::protocol::parser::unmarshall_event!(@helper results.next().unwrap(), $type ), )+
//                     }
//                 )
//             }
//     };
// }
//
// macro_rules! marshall_values {
//     ($buffer : ident, $($type : ident ( $value : expr) ),+) => {
//         parser::write_bytes(
//             &[$($crate::protocol::parser::MsgArgValue::$type($value),)+], $buffer
//         )
//     };
// }
//
// pub(super) use marshall_values;
// pub(super) use unmarshall_event;
