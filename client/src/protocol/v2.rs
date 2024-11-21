use super::WaylandStream;
use std::rc::Rc;
pub type WaylandId = u32;
pub type WlInterfaceId = u32;

#[derive(Debug)]
pub enum WlEventParseError {
    NoSuchEvent,
    ParsingError(String),
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

pub trait WlInterface<T> {
    fn get_interface_id() -> WlInterfaceId;
    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self;
    //fn parse_event(msg: RawMessage<'_>) -> Result<T, WlEventParseError>;
    fn parse_event(msg: RawMessage) -> Result<T, WlEventParseError>;
    fn get_object_id(&self) -> WaylandId;
}

//trait WlClient {
//    fn connect(socket_addr : &str) -> Result<Self>;
//    fn new_object<T : WlInterface<_>>(&mut self) -> T;
//    fn new_global<T : WlInterface<_>>(&mut self) -> T;
//
//    fn get_reference<T : WlInterface> (&self, object_id : u32) -> Option<T>;
//    fn add_event_handler<T, E, F>(&mut self, object : &T, handler)
//        where
//            T : WlInterface<E>
//            F : FnMut(&mut Self, event : WlMsg<E>) -> Option<WlMsg<E>>
//    ;
//}
