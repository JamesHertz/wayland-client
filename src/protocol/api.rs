use std::{io::Write, str::FromStr};

use color_eyre::eyre::eyre;

use super::parser::{self, MsgArgType, MsgArgValue, Parser};

pub type ObjectId = u32;

#[derive(Debug)]
pub struct WaylandEventMessage {
    pub sender_id: ObjectId,
    pub sender_obj: ObjectType,
    pub event: WaylandEvent,
}

#[derive(Debug)]
pub enum WaylandRequest {
    DisplaySync(u32),
    DisplayGetRegistry(u32),
}

#[derive(Debug)]
pub enum WaylandEvent {
    DisplayError {
        object_id: u32,
        code: u32,
        message: String,
    },

    RegistryGlobal {
        name: u32,
        interface: String,
        version: u32,
    },

    CallBackDone(u32),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub enum ObjectType {
    Display,
    Registry,
    CallBack,
    Compositor,
}

impl FromStr for ObjectType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "wl_compositor" => Ok(ObjectType::Compositor),
            name      => Err(format!("Type '{name}' not supported yet!"))
        }
    }
}

macro_rules! parse_event {

    (@helper $init : expr, ObjectId) => {
       $init.into_u32()
    };
    
    (@helper $init : expr, Uint32) => {
        $init.into_u32()
    };

    (@helper $init : expr, String) => {
        $init.into_str()
    };

    (@helper $init : expr, $x : ident) => {
        compile_error!(concat!("Unexpected type: ", stringify!($x)))
    };

    (@expand_results $buffer : ident, $($type : ident,)+) => {
            Parser::new(&[
                $( MsgArgType::$type, )+
            ]).parse($buffer)?
    };

    ($buffer : ident, $struct : ident ( $($type : ident)+ )) => {
            {
                let mut results = parse_event!(@expand_results $buffer, $($type,)+);
                Ok(WaylandEvent::$struct(
                    $(parse_event!(@helper results.next().unwrap(), $type ),)+
                ))
            }
    };

    ($buffer : ident, $struct : ident { $( $name : ident => $type : ident, )+ }) => {
            {
                let mut results = parse_event!(@expand_results $buffer, $($type,)+);
                Ok(WaylandEvent::$struct {
                        $( $name : parse_event!(@helper results.next().unwrap(), $type ), )+
                    }
                )
            }
        
    };
}

impl ObjectType {
    pub fn parse_event(&self, event_id: u16, buffer: &[u8]) -> color_eyre::Result<WaylandEvent> {
        match self {
            Self::Display if event_id == 0 => {
                parse_event!(buffer, DisplayError {
                    object_id => ObjectId,
                    code      => Uint32,
                    message   => String,
                })
            }
            Self::Registry if event_id == 0 => {
                parse_event!(buffer, RegistryGlobal {
                    name      => Uint32,
                    interface => String,
                    version   => Uint32,
                })
            }
            Self::CallBack => {
                if event_id != 0 {
                    return Err(eyre!("Invalid event id to CallBack"));
                }
                parse_event!(buffer, CallBackDone(Uint32))
            }

            _ => todo!(),
        }
    }

}

macro_rules!  write_bytes {
    ($buffer : ident, $($type : ident ( $value : expr) ),+) => {
        parser::write_bytes(
            &[$(MsgArgValue::$type($value),)+], $buffer
        )
    };
}

impl WaylandRequest {
    pub fn request_id(&self) -> u16 {
        match self {
            Self::DisplaySync(_) => 0,
            Self::DisplayGetRegistry(_) => 1,
        }
    }

    pub fn write_data(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        match self {
            Self::DisplaySync(value) | Self::DisplayGetRegistry(value) => {
                write_bytes!(buffer, Uint32(*value))
            }
        }
    }
}

#[allow(unused_variables)]
impl WaylandEvent {
    pub fn event_id(&self) -> u16 {
        match self {
            Self::DisplayError {
                object_id,
                code,
                message,
            } => 0,
            Self::RegistryGlobal {
                name,
                interface,
                version,
            } => 0,
            Self::CallBackDone(_) => 0,
        }
    }
}
