use super::*;
use super::base::WlSurface;
use log::debug;

declare_interface!(XdgWmBase);
declare_interface!(XdgSurface);
declare_interface!(XdgTopLevel);

impl XdgWmBase {
    pub fn get_xdg_surface(
        &self,
        new_surface: &XdgSurface,
        created_surface: &WlSurface,
    ) -> Result<usize> {
        debug!(
            "{} @ {:?} -> get_xdg_surface( {}, {} )",
            self.get_object_id(),
            Self::get_interface_id(),
            new_surface.get_object_id(),
            created_surface.get_object_id()
        );

        self.0.stream.send(WireMessage {
            object_id: self.get_object_id(),
            request_id: 2,
            values: &[
                Uint32(new_surface.get_object_id()),
                Uint32(created_surface.get_object_id()),
            ],
        })
    }
}

impl XdgSurface {
    pub fn get_top_level(&self, new_role : &XdgTopLevel) -> Result<usize> {

        debug!(
            "{} @ {:?} -> get_top_level ( {} )",
            self.get_object_id(),
            Self::get_interface_id(),
            new_role.get_object_id(),
        );

        self.0.stream.send(WireMessage {
            object_id: self.get_object_id(),
            request_id: 1,
            values: &[ Uint32(new_role.get_object_id()) ],
        })

    }

}


impl XdgTopLevel {
    pub fn set_title(&self, title : &str) -> Result<usize> {
        debug!(
            "{} @ {:?} -> set_title( {title:?} )",
            self.get_object_id(),
            Self::get_interface_id(),
        );

        self.0.stream.send(WireMessage {
            object_id: self.get_object_id(),
            request_id: 2,
            values: &[ Str(title.to_string()) ],
        })
    }
}
