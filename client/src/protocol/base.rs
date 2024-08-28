use super::*;
// use crate::Result;
use crate::{error::fallback_error, wire_format::parsing as parser};
use log::{debug, info, trace};

declare_interface!(
    WlDisplay,
    @iterator  = iter,
    @object_id = obj_id,
    @branches  = {
        0 => {
            let (object, code, message) = (
                parser::parse_u32(&mut iter)?,
                parser::parse_u32(&mut iter)?,
                parser::parse_str(&mut iter)?
            );

            debug!(
                "{obj_id} @ {:?} <- display_error( {object}, {code}, {message:?}", 
                WlDisplay::get_interface_id()
            );
            WlDisplayError{ object, code, message }
        },

        1 => {
            let object = parser::parse_u32(&mut iter)?;
            debug!( 
                "{obj_id} @ {:?} <- delete_id ( {object} )",
                WlDisplay::get_interface_id()
            );
            WlDisplayDeleteId { object }
        }
    }
);

impl WlDisplay {
    pub fn sync(&self, new_id: WaylandId) -> Result<usize> {
        debug!(
            "{} @ {:?} -> sync( {new_id} )",
            self.get_object_id(),
            Self::get_interface_id()
        );

        self.0.stream.send(WireMessage {
            object_id: self.get_object_id(),
            request_id: 0,
            values: &[Uint32(new_id)],
        })
    }

    pub fn get_registry(&self, new_id: WaylandId) -> Result<usize> {
        debug!(
            "{} @ {:?} -> get_registry( {new_id} )",
            self.get_object_id(),
            Self::get_interface_id()
        );

        self.0.stream.send(WireMessage {
            object_id: self.get_object_id(),
            request_id: 1,
            values: &[Uint32(new_id)],
        })
    }
}

declare_interface!(
    WlRegistry,
    @iterator  = iter,
    @object_id = obj_id,
    @branches  = {
        0 => {
            let (name, interface, version) = (
                parser::parse_u32(&mut iter)?,
                parser::parse_str(&mut iter)?,
                parser::parse_u32(&mut iter)?
            );

            debug!( 
                "{obj_id} @ {:?} <- global ( {name}, {interface:?}, {version} )",
                WlDisplay::get_interface_id()
            );

            WlRegistryGlobal { name, interface, version }
        }
    }
);