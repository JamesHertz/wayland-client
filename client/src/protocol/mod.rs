#![allow(unused)]
use crate::error;
use std::{any::Any, fmt, rc::Rc};

pub mod base;
mod macros;
pub mod xdg_shell;

#[allow(unused_imports)]
pub use self::WireValue::*;

#[allow(unused_imports)]
use macros::declare_interface;

pub type WlInterfaceId = u32;
pub type WaylandId = u32;
pub type WlEventId = u16;
//pub type EventParseResult<T> = Result<T, EventParseError>;

#[derive(Debug)]
pub enum WireValue {
    Uint32(u32),
    Int32(i32),
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
    fn send(&self, msg: WireMessage) -> Result<usize, error::Error>;
}

//#[derive(Debug)]
//pub enum WlEventParseError {
//    NoSuchEvent,
//    ParsingError(String),
//}

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
    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self;
    fn parse_event(
        object_id: WaylandId,
        event_id: WlEventId,
        iter: &mut impl Iterator<Item = u8>,
    ) -> Result<Self::Event, WlEventParseError>;

    fn parse_msg(msg: RawMessage) -> Result<WlEventMsg<Self::Event>, WlEventParseError> {
        let object_id = msg.object_id;
        let event_id = msg.event_id;
        let mut iter = msg.payload.iter().copied();
        let event = Self::parse_event(object_id, event_id, &mut iter)?;

        let remaining = iter.count();
        if remaining != 0 {
            return Err(WlEventParseError::ParsingError(error::fallback_error!(
                "Found {remaining} extra bytes while parsing {event:?}."
            )));
        }

        Ok(WlEventMsg { object_id, event })
    }
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

#[derive(Debug)]
pub enum WlEventParseError {
    NoEvent(WlEventId),
    ParsingError(error::Error),
}

impl From<error::Error> for WlEventParseError {
    fn from(value: error::Error) -> Self {
        Self::ParsingError(value)
    }
}

impl fmt::Display for WlEventParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        // TODO: implement Display for error::Error so that this can be more beautiful
        // TODO: implment display for this
        write!(f, "{self:?}")
    }
}

//pub trait WaylandInterface {
//    fn get_interface_id() -> WlInterfaceId;
//    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self;
//    fn parse_event(
//        object_id: WaylandId,
//        event_id: WaylandId,
//        payload: &[u8],
//    ) -> EventParseResult<WlEvent>;
//    fn get_object_id(&self) -> WaylandId;
//}

//pub trait WlInterface<T> {
//    fn get_interface_id() -> WlNewInterfaceId;
//    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self;
//    fn parse_event(msg: RawMessage<'_>) -> Option<T>;
//    fn get_object_id(&self) -> WlObjectId;
//}

// #[derive(Debug, PartialEq, Eq)]
// #[derive(Debug)]
//pub enum WlEvent {
//    WlDisplayError {
//        object: WaylandId,
//        code: u32,
//        message: String,
//    },
//    WlDisplayDeleteId(WaylandId),
//    WlRegistryGlobal {
//        name: u32,
//        interface: String,
//        version: u32,
//    },
//    WlCallBackDone(u32),
//    WlShmFormat(base::WlShmFormatValue),
//    XdgSurfaceConfigure(u32),
//    XdgTopLevelConfigure {
//        width: i32,
//        height: i32,
//        states: Vec<xdg_shell::XdgTopLevelState>,
//    },
//    XdgTopLevelWmCapabilities(Vec<xdg_shell::XdgWmCapabilities>),
//    WlSurfacePreferredBufferScale(u32),
//    WlSurfacePreferredBufferTransform(base::WlOutputTransform),
//    WlBufferRelease,
//}

//#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
//pub enum WlInterfaceId {
//    WlDisplay,
//    WlRegistry,
//    WlCallBack,
//    WlCompositor,
//    XdgWmBase,
//    WlShm,
//    WlSurface,
//    WlShmPool,
//    WlBuffer,
//    XdgSurface,
//    XdgTopLevel,
//}
