// TODO: move this to the parser module c:
macro_rules! unmarshall_event {

    (@helper $init : expr, Uint32) => {
        $init.into_u32()
    };

    (@helper $init : expr, String) => {
        $init.into_str()
    };

    (@helper $init : expr, Int32) => {
        $init.into_i32()
    };

    (@helper $init : expr, Array) => {
        $init.into_array()
    };

    (@helper $init : expr, $x : ident) => {
        compile_error!(concat!("Unexpected type: ", stringify!($x), ". I should be Int32, Uint32, String or Array."))
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

use super::parser::{self, Array, MsgArgType, MsgArgValue, Parser};
use color_eyre::eyre::eyre;
use log::warn;
use std::{io::Write, str::FromStr};

pub type ObjectId = u32;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
pub enum TopLevelState {
    None = 0,
    Maximized,
    Fullscreen,
    Resizing,
    Activated,
    TiledLeft,
    TiledRight,
    TiledTop,
    TiledBottom,
    Suspended,
}

impl TryFrom<u8> for TopLevelState {
    type Error = color_eyre::Report;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= Self::None as u8 && value <= Self::Suspended as u8 {
            unsafe { Ok(std::mem::transmute::<u8, Self>(value)) }
        } else {
            Err(eyre!("Invalid TopLevelState {}", value))
        }
    }
}

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
    XdgSurfaceConfigure(u32),
    XdgTopLevelConfigure {
        width: u32,
        height: u32,
        states: Vec<TopLevelState>,
    }, // XdgSurfaceTopLevelConfigure {
       //     width  : i32,
       //     height : i32,
       // }
       // EventCapabities()
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
    XdgTopLevel,
}

impl WaylandObject {
    pub fn parse_event(
        &self,
        event_id: u16,
        buffer: &[u8],
    ) -> color_eyre::Result<WaylandEvent> {
        match (self, event_id) {
            (Self::Display, 0) => {
                unmarshall_event!(buffer, DisplayError { object_id => Uint32,
                    code      => Uint32,
                    message   => String,
                })
            }
            (Self::Display, 1) => {
                unmarshall_event!(buffer, DisplayDelete(Uint32))
            }

            (Self::Registry, 0) => {
                unmarshall_event!(buffer, RegistryGlobal {
                    name      => Uint32,
                    interface => String,
                    version   => Uint32,
                })
            }

            (Self::CallBack, 0) => {
                unmarshall_event!(buffer, CallBackDone(Uint32))
            }

            (Self::Shm, 0) => {
                unmarshall_event!(buffer, ShmFormat(Uint32))
            }

            (Self::Display | Self::CallBack | Self::Shm, _) => {
                Err(eyre!("Invalid message {event_id} for {:?}", self))
            }
            (Self::XdgTopLevel, 3) | (Self::Buffer, 0) => {
                warn!("Receiving event {event_id} for {self:?} ... Ignoring and emitting ShmFormat(0)");
                Ok(WaylandEvent::ShmFormat(0))
            }
            (Self::XdgSurface, 0) => {
                unmarshall_event!(buffer, XdgSurfaceConfigure(Uint32))
            }

            // (Self::XdgTopLevel, 0) => {
            //     // FIXME: I am a mess, fixme c:
            //     let mut results = Parser::new(&[
            //         MsgArgType::Uint32,
            //         MsgArgType::Uint32,
            //         MsgArgType::Array,
            //     ])
            //     .parse(buffer)?;
            //
            //     let height = results.next().unwrap().into_u32();
            //     let width = results.next().unwrap().into_u32();
            //
            //     let event_states = results
            //         .next()
            //         .unwrap()
            //         .into_array()
            //         .into_iter()
            //         .map(TopLevelState::try_from);
            //
            //     let mut states = Vec::new();
            //     for state in event_states {
            //         states.push(state?);
            //     }
            //
            //     Ok(WaylandEvent::XdgTopLevelConfigure {
            //         width,
            //         height,
            //         states,
            //     })
            //     // unmarshall_event!(buffer, XdgTopLevelConfigure {
            //     //     width  => Uint32,
            //     //     height => Uint32,
            //     //     states => Array,
            //     // })
            // }
            _ => {
                Err(eyre!("Event {event_id} for {self:?} doesn't have handler"))
            }
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
    SufaceAttach {
        buffer_id: ObjectId,
        x: u32,
        y: u32,
    },
    SufaceCommit,
    XdgSurfaceGetTopLevel(ObjectId),
    XdgTopLevelSetTitle(String),
    XdgSurfaceAckConfigure(u32),
}

impl WaylandRequest {
    pub fn request_id(&self) -> u16 {
        match self {
            Self::DisplaySync(_) => 0,
            Self::DisplayGetRegistry(_) => 1,
            Self::CompositorCreateSurface(_) => 0,
            Self::XdgWmGetSurface { .. } => 2,
            Self::RegistryBind { .. } => 0,
            Self::ShmCreatePool { .. } => 0,
            Self::ShmPoolCreateBuffer { .. } => 0,
            Self::SufaceAttach { .. } => 1,
            Self::SufaceCommit => 6,
            Self::XdgSurfaceGetTopLevel(_) => 1,
            Self::XdgTopLevelSetTitle(_) => 1,
            Self::XdgSurfaceAckConfigure(_) => 4,
        }
    }

    pub fn write_data(self, buffer: &mut impl Write) -> std::io::Result<()> {
        match self {
            Self::DisplaySync(value)
            | Self::DisplayGetRegistry(value)
            | Self::CompositorCreateSurface(value)
            | Self::XdgSurfaceGetTopLevel(value)
            | Self::XdgSurfaceAckConfigure(value) => {
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
            Self::ShmCreatePool {
                pool_id,
                fd: _,
                size,
            } => {
                // u32 as i32 doesn't make any difference for marshalling
                marshall_values!(
                    buffer,
                    Uint32(pool_id),
                    // Int32(fd), // TODO: think about this ...
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
                    Int32(offset),
                    Int32(width),
                    Int32(height),
                    Int32(stride),
                    Uint32(pixel_format as u32)
                )
            }
            Self::SufaceAttach { buffer_id, x, y } => {
                marshall_values!(
                    buffer,
                    Uint32(buffer_id),
                    Uint32(x),
                    Uint32(y)
                )
            }
            Self::SufaceCommit => Ok(()),
            Self::XdgTopLevelSetTitle(title) => {
                marshall_values!(buffer, String(title))
            }
        }
    }
}
