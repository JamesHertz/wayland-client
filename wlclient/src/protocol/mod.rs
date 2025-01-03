#![allow(unused)]
use std::{any::Any, fmt, rc::Rc};
use std::io::Error as IoError;
use wire_format::parsing;

#[allow(unused_imports)]
pub use self::WireValue::*;
#[allow(unused_imports)]
use macros::declare_interfaces;

pub use wire_format::{WireMsgHeader, ClientStream};

pub mod base;
pub mod wire_format;
pub mod xdg_shell;
mod macros;

pub type WlInterfaceId = u32;
pub type WaylandId = u32;
pub type WlEventId = u16;
pub type Array = Vec<u32>;
pub type EmptyEvent = ();
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum WireValue {
    Uint32(u32),
    Int32(i32),
    Str(String),
    Array(Vec<u8>),
    FileDesc(i32),
}

impl fmt::Display for WireValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Uint32(v)    => write!(f, "{v}"),
            Int32(v)     => write!(f, "{v}"),
            Str(v)    => write!(f, "{v:?}"),
            Array(v) => write!(f, "{v:?}"),
            FileDesc(v)  => write!(f, "{v}"),
        }
    }
}

#[derive(Debug)]
pub struct WireMessage<'a> {
    pub object_id: WaylandId,
    pub request_id: WaylandId,
    pub values: &'a [WireValue],
}

// Implementation of this trait are recommended to use interior mutability
// (https://doc.rust-lang.org/reference/interior-mutability.html)
pub trait WaylandStream {
    fn send(&self, msg: WireMessage) -> Result<usize>;
}

// TODO: fix this later
pub struct RawMessage {
    pub object_id: WaylandId,
    pub event_id: u16,
    pub payload: Box<[u8]>,
}

pub struct WlEventMsg<E> {
    pub object_id: WaylandId,
    pub event: E,
}

impl<E: 'static> WlEventMsg<E> {
    #[inline]
    pub fn from_any(value: Box<dyn Any>) -> Option<Self> {
        value.downcast().map_or(None, |value| Some(*value))
    }

    #[inline(always)]
    pub fn to_any(self) -> Box<dyn Any> {
        Box::new(self)
    }
}

pub trait WlInterface {
    type Event: std::fmt::Debug;

    fn get_object_id(&self) -> WaylandId;
    fn get_interface_id() -> WlInterfaceId;
    fn get_display_name() -> &'static str { "" }

    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self;

    fn parse_event(
        object_id: WaylandId,
        event_id: WlEventId,
        iter: &mut impl Iterator<Item = u8>,
    ) -> Result<Self::Event>;

    fn parse_msg(msg: RawMessage) -> Result<WlEventMsg<Self::Event>> {
        let object_id = msg.object_id;
        let event_id = msg.event_id;
        let mut iter = msg.payload.iter().copied();
        let event = Self::parse_event(object_id, event_id, &mut iter)?;

        let remaining = iter.count();
        if remaining != 0 {
            return Err(Error::UnexpectedExtraBytes { 
                object_id, event_id, extra_bytes: remaining 
            });
        }

        Ok(WlEventMsg { object_id, event })
    }
}

#[derive(Clone)]
pub struct WlObjectMetaData {
    object_id: WaylandId,
    stream: Rc<dyn WaylandStream>,
}

impl fmt::Debug for WlObjectMetaData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WlObjectMetaData")
            .field("object_id", &self.object_id)
            .finish()
    }
}

#[derive(Debug)]
pub enum Error {
    NoEvent(WlEventId),
    UnexpectedExtraBytes { object_id : u32, event_id : u16, extra_bytes : usize },
    ParsingError(parsing::Error),
    IoError(IoError),
}

impl From<IoError> for Error {
    fn from(value: IoError) -> Self {
        Error::IoError(value)
    }
}

impl From<parsing::Error> for Error {
    fn from(value: parsing::Error) -> Self {
        Self::ParsingError(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        // TODO: implement Display for error::Error so that this can be more beautiful
        // TODO: implment display for this
        write!(f, "{self:?}")
    }
}
