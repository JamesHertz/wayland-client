// TODO: move this to the parser module c:
macro_rules! unmarshall_event {

    (@helper $init : expr, Uint32) => {
        $init.into_u32()
    };

    (@helper $init : expr, String) => {
        $init.into_str()
    };

    (@helper $init : expr, $x : ident) => {
        compile_error!(concat!("Unexpected type: ", stringify!($x), ". I should be either Uint32 or String."))
    };

    (@expand_results $buffer : ident, $($type : ident,)+) => {
            Parser::new(&[
                $( MsgArgType::$type, )+
            ]).parse($buffer)?
    };

    ($buffer : ident, $struct : ident ( $($type : ident)+ )) => {
            {
                let mut results = unmarshall_event!(@expand_results $buffer, $($type,)+);
                Ok(WaylandEvent::$struct(
                    $(unmarshall_event!(@helper results.next().unwrap(), $type ),)+
                ))
            }
    };

    ($buffer : ident, $struct : ident { $( $name : ident => $type : ident, )+ }) => {
            {
                let mut results = unmarshall_event!(@expand_results $buffer, $($type,)+);
                Ok(WaylandEvent::$struct {
                        $( $name : unmarshall_event!(@helper results.next().unwrap(), $type ), )+
                    }
                )
            }

    };
}

// TODO: same as above
macro_rules! marshall_values {
    ($buffer : ident, $($type : ident ( $value : expr) ),+) => {
        parser::write_bytes(
            &[$(MsgArgValue::$type($value),)+], $buffer
        )
    };
}

use super::parser::{self, MsgArgType, MsgArgValue, Parser};
use color_eyre::eyre::eyre;
use std::{io::Write, str::FromStr};

pub type ObjectId = u32;

#[derive(Debug, PartialEq, Eq)]
pub struct WaylandEventMessage {
    pub sender_id: ObjectId,
    pub sender_obj: WaylandObject,
    pub event: WaylandEvent,
}

#[derive(Debug, PartialEq, Eq)]
pub enum WaylandEvent {
    DisplayError {
        object_id: u32,
        code: u32,
        message: String,
    },
    DisplayDelete(u32),
    RegistryGlobal {
        name: u32,
        interface: String,
        version: u32,
    },
    CallBackDone(u32),
    ShmFormat(u32),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub enum WaylandObject {
    Display,
    Registry,
    CallBack,
    Compositor,
    Surface,
    XdgSurface,
    // ZLayerShell,
    Shm,
    ShmPool,
    XdgWmBase,
    Buffer,
}

impl WaylandObject {
    pub fn parse_event(
        &self,
        event_id: u16,
        buffer: &[u8],
    ) -> color_eyre::Result<WaylandEvent> {
        match self {
            Self::Display if event_id == 0 => {
                unmarshall_event!(buffer, DisplayError {
                    object_id => Uint32,
                    code      => Uint32,
                    message   => String,
                })
            }
            Self::Display if event_id == 1 => {
                unmarshall_event!(buffer, DisplayDelete(Uint32))
            }

            Self::Registry if event_id == 0 => {
                unmarshall_event!(buffer, RegistryGlobal {
                    name      => Uint32,
                    interface => String,
                    version   => Uint32,
                })
            }
            Self::CallBack if event_id == 0 => {
                unmarshall_event!(buffer, CallBackDone(Uint32))
            }

            Self::Shm if event_id == 0 => {
                unmarshall_event!(buffer, ShmFormat(Uint32))
            }

            Self::Display | Self::CallBack | Self::Shm => {
                Err(eyre!("Invalid message {event_id} for {:?}", self))
            }
            _ => todo!(),
        }
    }

    pub fn from_interface(interface: &str) -> Option<Self> {
        match interface {
            "wl_compositor" => Some(WaylandObject::Compositor),
            "xdg_wm_base" => Some(WaylandObject::XdgWmBase),
            "wl_shm" => Some(WaylandObject::Shm),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum ShmPixelFormat {
    Argb = 0,
    Xrgb = 1,
}

#[derive(Debug)]
pub enum WaylandRequest {
    DisplaySync(u32),
    DisplayGetRegistry(u32),
    CompositorCreateSurface(u32),
    XdgWmGetSurface {
        new_id: u32,
        surface: u32,
    },
    RegistryBind {
        name: u32,
        interface: String,
        version: u32,
        new_id: u32,
    },
    ShmCreatePool {
        pool_id: ObjectId,
        fd: i32,
        size: i32,
    },
    ShmPoolCreateBuffer {
        buffer_id: ObjectId,
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
        pixel_format: ShmPixelFormat,
    },
}

impl WaylandRequest {
    pub fn request_id(&self) -> u16 {
        match self {
            Self::DisplaySync(_) => 0,
            Self::DisplayGetRegistry(_) => 1,
            Self::CompositorCreateSurface(_) => 0,
            Self::XdgWmGetSurface { .. } => 0,
            Self::RegistryBind { .. } => 0,
            Self::ShmCreatePool { .. } => 0,
            Self::ShmPoolCreateBuffer { .. } => 0,
        }
    }

    pub fn write_data(self, buffer: &mut impl Write) -> std::io::Result<()> {
        match self {
            Self::DisplaySync(value)
            | Self::DisplayGetRegistry(value)
            | Self::CompositorCreateSurface(value) => {
                marshall_values!(buffer, Uint32(value))
            }
            Self::XdgWmGetSurface { new_id, surface } => {
                marshall_values!(buffer, Uint32(new_id), Uint32(surface))
            }
            Self::RegistryBind {
                name,
                interface,
                version,
                new_id,
            } => {
                marshall_values!(
                    buffer,
                    Uint32(name),
                    String(interface),
                    Uint32(version),
                    Uint32(new_id)
                )
            }
            Self::ShmCreatePool { pool_id, fd, size } => {
                // u32 as i32 doesn't make any difference for marshalling
                marshall_values!(
                    buffer,
                    Uint32(pool_id),
                    Int32(fd),
                    Int32(size)
                )
            }

            Self::ShmPoolCreateBuffer {
                buffer_id,
                offset,
                width,
                height,
                stride,
                pixel_format,
            } => {
                marshall_values!(
                    buffer,
                    Uint32(buffer_id),
                    Int32(dbg!(offset)),
                    Int32(dbg!(width)),
                    Int32(dbg!(height)),
                    Int32(dbg!(stride)),
                    Uint32(pixel_format as u32)
                )
            }
        }
    }
}
