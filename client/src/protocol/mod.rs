use crate::{error, wire_format::parsing, Result};
use std::{fmt, rc::Rc, result::Result as StdResult};

pub mod base;
mod macros;

#[allow(unused_imports)]
pub use self::WireValue::*;

#[allow(unused_imports)]
pub use self::WlEvent::*;

#[allow(unused_imports)]
use macros::declare_interface;

pub type WaylandId = u32;
pub type EventParseResult<T> = StdResult<T, EventParseError>;

#[derive(Debug)]
pub enum WireValue {
    Uint32(u32),
    Int32(u32),
    Str(String),
    Array(Vec<u8>),
    FileDesc(i32),
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

pub trait WaylandInterface {
    fn get_interface_id() -> WlInterfaceId;
    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self;
    fn parse_event(
        object_id: WaylandId,
        event_id: WaylandId,
        payload: &[u8],
    ) -> EventParseResult<WlEvent>;
    fn get_object_id(&self) -> WaylandId;
}

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

// #[derive(Debug, PartialEq, Eq)]
#[derive(Debug)]
pub enum WlEvent {
    WlDisplayError {
        object: WaylandId,
        code: u32,
        message: String,
    },
    WlDisplayDeleteId {
        object: WaylandId,
    },

    WlRegistryGlobal {
        name: u32,
        interface: String,
        version: u32,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub enum WlInterfaceId {
    WlDisplay,
    WlRegistry,
}

#[derive(Debug)]
pub enum EventParseError {
    NoEvent(WaylandId),
    ParsingError(error::Error),
}

impl From<error::Error> for EventParseError {
    fn from(value: error::Error) -> Self {
        EventParseError::ParsingError(value)
    }
}

impl fmt::Display for EventParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> StdResult<(), fmt::Error> {
        // TODO: implement Display for error::Error so that this can be more beautiful
        // TODO: implment display for this
        write!(f, "{self:?}")
    }
}


