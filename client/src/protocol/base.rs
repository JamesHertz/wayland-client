use super::*;
use crate::Result;
use log::info;



macro_rules! declare_interface {
    ($name : ident, $( $parse_event : tt)* ) => {
        pub struct $name(WlObjectMetaData);

        impl WaylandInterface for $name {
            fn get_interface_id() -> WlInterfaceId {
                WlInterfaceId::$name
            }

            fn build(object_id: u32, stream: Rc<dyn WaylandStream>) -> Self {
                Self(WlObjectMetaData { object_id, stream })
            }

            fn get_object_id(&self) -> WaylandId {
                self.0.object_id
            }

            $($parse_event)*
        }

    }
}

declare_interface!(
    WlDisplay, 
    fn parse_event(
        object_id : WaylandId,
        event_id: WaylandId,
        _payload: &[u8],
    ) -> Option<WlEvent> {
        info!("Received message {event_id} for object {object_id}@{:?}", Self::get_interface_id());
        None
    }
);

impl WlDisplay {
    pub fn sync(&self, new_id: WaylandId) -> Result<usize> {
        self.0.stream.send(WireMessage {
            object_id: self.0.object_id,
            request_id: 0,
            values: &[Uint32(new_id)],
        })
    }

    pub fn get_registry(&self, new_id : WaylandId) -> Result<usize> {
        self.0.stream.send(WireMessage {
            object_id: self.0.object_id,
            request_id: 1,
            values: &[Uint32(new_id)],
        })
    }
}

declare_interface!(
    WlRegistry, 
    fn parse_event(
        object_id : WaylandId,
        event_id: WaylandId,
        _payload: &[u8],
    ) -> Option<WlEvent> {
        info!("Received message {event_id} for object {object_id}@{:?}", Self::get_interface_id());
        None
    }
);
