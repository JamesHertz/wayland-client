//use super::base::WlSurface;
use super::*;
use crate::{
    error::{self, fallback_error},
    wire_format::parsing as parser,
};
use log::debug;
use std::{convert::TryFrom, result::Result as StdResult};

//declare_interface!(XdgWmBase);
//declare_interface!(
//    @name(XdgSurface),
//    @events(obj_id, iter) {
//        0 => {
//            let serial_nr = parser::parse_u32(&mut iter)?;
//
//            debug!(
//                "{obj_id} @ {:?} <- configure ( {serial_nr} )",
//                Self::get_interface_id()
//            );
//
//            XdgSurfaceConfigure( serial_nr )
//        }
//
//    }
//);
//
//declare_interface!(
//    @name(XdgTopLevel),
//    @events(obj_id, iter) {
//        0 => {
//            let (width, height, states) = (
//                parser::parse_i32(&mut iter)?,
//                parser::parse_i32(&mut iter)?,
//                parser::parse_u32_array(&mut iter)?
//                            .into_iter()
//                            .map(u32::try_into)
//                            .collect::<Result<_>>()?
//            );
//
//            debug!(
//                "{obj_id} @ {:?} <- configure ( {width}, {height}, {states:?} )",
//                Self::get_interface_id()
//            );
//
//            XdgTopLevelConfigure {
//                width, height, states
//            }
//        },
//
//        3 => {
//            let capabilities =  parser::parse_u32_array(&mut iter)?
//                        .into_iter()
//                        .map(u32::try_into)
//                        .collect::<Result<_>>()?;
//            debug!(
//                "{obj_id} @ {:?} <- wm_capabilities ( {capabilities:?} )",
//                Self::get_interface_id()
//            );
//
//            XdgTopLevelWmCapabilities( capabilities )
//        }
//    }
//);
//
//impl XdgWmBase {
//    pub fn get_xdg_surface(
//        &self,
//        new_surface: &XdgSurface,
//        created_surface: &WlSurface,
//    ) -> Result<usize> {
//        debug!(
//            "{} @ {:?} -> get_xdg_surface( {}, {} )",
//            self.get_object_id(),
//            Self::get_interface_id(),
//            new_surface.get_object_id(),
//            created_surface.get_object_id()
//        );
//
//        self.0.stream.send(WireMessage {
//            object_id: self.get_object_id(),
//            request_id: 2,
//            values: &[
//                Uint32(new_surface.get_object_id()),
//                Uint32(created_surface.get_object_id()),
//            ],
//        })
//    }
//}
//
//impl XdgSurface {
//    pub fn get_top_level(&self, new_role: &XdgTopLevel) -> Result<usize> {
//        debug!(
//            "{} @ {:?} -> get_top_level ( {} )",
//            self.get_object_id(),
//            Self::get_interface_id(),
//            new_role.get_object_id(),
//        );
//
//        self.0.stream.send(WireMessage {
//            object_id: self.get_object_id(),
//            request_id: 1,
//            values: &[Uint32(new_role.get_object_id())],
//        })
//    }
//}
//
//#[derive(Debug)]
//pub enum XdgWmCapabilities {
//    WindowMenu = 1,
//    Maximize = 2,
//    Fullscreen = 3,
//    Minimize = 4,
//}
//
//impl TryFrom<u32> for XdgWmCapabilities {
//    type Error = error::Error;
//
//    fn try_from(value: u32) -> StdResult<Self, Self::Error> {
//        Ok(match value {
//            1 => Self::WindowMenu,
//            2 => Self::Maximize,
//            3 => Self::Fullscreen,
//            4 => Self::Minimize,
//            value => {
//                return Err(fallback_error!("Invalid WmCapability value '{value}' it should be in range 1..4"));
//            }
//        })
//    }
//}
//
//#[derive(Debug)]
//pub enum XdgTopLevelState {
//    Maximized = 1,
//    Fullscreen = 2,
//    Resizing = 3,
//    Activated = 4,
//    TiledLeft = 5,
//    TiledRight = 6,
//    TiledTop = 7,
//    TiledBottom = 8,
//    Suspended = 9,
//}
//
//impl TryFrom<u32> for XdgTopLevelState {
//    type Error = error::Error;
//
//    fn try_from(value: u32) -> StdResult<Self, Self::Error> {
//        Ok(match value {
//            1 => Self::Maximized,
//            2 => Self::Fullscreen,
//            3 => Self::Resizing,
//            4 => Self::Activated,
//            5 => Self::TiledLeft,
//            6 => Self::TiledRight,
//            7 => Self::TiledTop,
//            8 => Self::TiledBottom,
//            9 => Self::Suspended,
//            value => {
//                return Err(fallback_error!("Invalid XdgTopLevelState value '{value}' it should be in range 1..9"));
//            }
//        })
//    }
//}
//
//
//impl XdgTopLevel {
//    pub fn set_title(&self, title: &str) -> Result<usize> {
//        debug!(
//            "{} @ {:?} -> set_title( {title:?} )",
//            self.get_object_id(),
//            Self::get_interface_id(),
//        );
//
//        self.0.stream.send(WireMessage {
//            object_id: self.get_object_id(),
//            request_id: 2,
//            values: &[Str(title.to_string())],
//        })
//    }
//
//    pub fn set_app_id(&self, app_id: &str) -> Result<usize> {
//        debug!(
//            "{} @ {:?} -> set_app_id( {app_id:?} )",
//            self.get_object_id(),
//            Self::get_interface_id(),
//        );
//
//        self.0.stream.send(WireMessage {
//            object_id: self.get_object_id(),
//            request_id: 3,
//            values: &[Str(app_id.to_string())],
//        })
//    }
//}
//
//
//impl XdgSurface {
//    pub fn ack_configure(&self, serial_nr : u32) -> Result<usize> {
//        debug!(
//            "{} @ {:?} -> ack_configure( {serial_nr} )",
//            self.get_object_id(),
//            Self::get_interface_id()
//        );
//
//        self.0.stream.send( WireMessage {
//            object_id  : self.get_object_id(),
//            request_id : 4,
//            values: &[ Uint32(serial_nr) ]
//        })
//    }
//
//}
