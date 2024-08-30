macro_rules! declare_interface {

    (@base_skeleton $name : ident, { $($parse_event_body : tt)* } ) => {
        // TODO: think about this 'pub' thing ...
        pub struct $name(crate::protocol::WlObjectMetaData);

        impl crate::protocol::WaylandInterface for $name {
            fn get_interface_id() -> crate::protocol::WlInterfaceId {
                crate::protocol::WlInterfaceId::$name
            }

            fn build(
                object_id: u32,
                stream: std::rc::Rc<dyn crate::protocol::WaylandStream>
            ) -> Self {
                Self(crate::protocol::WlObjectMetaData { object_id, stream })
            }

            fn get_object_id(&self) -> WaylandId {
                self.0.object_id
            }

            $($parse_event_body)*
        }

    };

    ($name : ident) => {
        declare_interface!(@base_skeleton $name, {
            fn parse_event(
                _object_id: WaylandId,
                event_id: WaylandId,
                _payload: &[u8],
            ) -> crate::protocol::EventParseResult<WlEvent> {
                Err(crate::protocol::EventParseError::NoEvent(event_id))
            }
        });
    };

    (
        @name($name : ident), 
        @events($object_id : ident, $iter_name : ident) {
            $( $parse_event : tt)+
        }
    ) => {

        declare_interface!(@base_skeleton $name, {
            fn parse_event(
                $object_id: WaylandId,
                event_id: WaylandId,
                payload: &[u8],
            ) -> crate::protocol::EventParseResult<WlEvent> {
                let mut $iter_name = payload.iter().cloned();
                // debug!(
                //     "Received message {event_id} for object {}@{:?}", 
                //     $object_id, Self::get_interface_id()
                // );

                let event = match event_id {
                    $($parse_event)+,
                    id => { return Err(crate::protocol::EventParseError::NoEvent(id)) }
                };

                let remaining = $iter_name.count();
                if remaining != 0 {
                    return Err(crate::protocol::EventParseError::ParsingError(
                        crate::error::fallback_error!(
                        "Found {remaining} extra bytes while parsing {event:?}."
                    )));
                }

                Ok(event)
            }
        });

    }
}

pub(super) use declare_interface;
