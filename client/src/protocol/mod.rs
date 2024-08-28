// use self::WireValue::*;
pub mod base;

use std::rc::Rc;
use std::fmt;
use crate::Result;

#[allow(unused_imports)]
pub use self::WireValue::*;

pub type WaylandId = u32;
// pub type WlEventId  = WaylandId;
// pub type EventParser = for<'a> fn(WaylandId, WlEventId, &'a [u8]) -> Result<WlEvent>;

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
    fn parse_event(object_id : WaylandId, event_id: WaylandId, payload: &[u8]) -> Option<WlEvent>;
    fn get_object_id(&self) -> WaylandId;
}

pub struct WlObjectMetaData {
    object_id: WaylandId,
    stream: Rc<dyn WaylandStream>,
}

impl fmt::Debug for WlObjectMetaData  {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         f.debug_struct("WlObjectMetaData")
          .field("object_id", &self.object_id)
          .finish()
    }
}

// #[derive(Debug, PartialEq, Eq)]
pub enum WlEvent {
    WlDisplayError    { object : WaylandId, code : u32, message : String },
    WlDisplayDeleteId { object : WaylandId },

    WlRegistryGlobal { name: u32, interface: String, version: u32 }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub enum WlInterfaceId {
    WlDisplay,
    WlRegistry
}

// impl Into<EventParser> for WlInterfaceId {
//     fn into(self) -> EventParser {
//         match self {
//             Self::WlDisplay => base::WlDisplay::parse_event
//         }
//     }
// }






