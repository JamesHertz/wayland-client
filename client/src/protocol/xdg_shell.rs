//use super::base::WlSurface;

use super::macros::declare_interfaces;
use super::base::WlSurface;

declare_interfaces! {
    FirstId = 100,
    XdgPositioner,
    XdgPopUp,

    @interface(XdgSurface) {
        @requests {
            destroy();
            get_toplevel(top_level : &XdgTopLevel) => [ Uint32(top_level.get_object_id()) ];
            get_popup(popup : &XdgPopUp, parent : &XdgSurface, positioner : &XdgPositioner) => [ 
                Uint32(popup.get_object_id()), Uint32(parent.get_object_id()), Uint32(positioner.get_object_id()),
            ];

            set_window_geometry(x: i32, y: i32, width: i32, height: i32) => [ 
                Int32(x), Int32(y), Int32(width), Int32(height)
            ];
            ack_configure(serial: u32) => [ Uint32( serial ) ];
        }
        @events { configure( serial_nr : u32); }
    },

    @interface(XdgTopLevel) {
        @requests {
            destroy();
            set_parent(parent : &XdgTopLevel) => [ Uint32(parent.get_object_id()) ];
            set_title(title: &str)   => [ Str(title.to_string()) ];
            set_app_id(app_id: &str) => [ Str(app_id.to_string()) ];
        }

        @events {
            configure(height : i32, width: i32, states : Array);
            close();
            configure_bounds(width: i32, height: i32);
            wm_capabilities(capabilities: Array);
        }

    },

    @interface(XdgWmBase) {
        @requests {
            destroy();
            create_positioner(positioner : &XdgPositioner) => [ Uint32(positioner.get_object_id()) ];
            get_xdg_surface( xdg_surface: &XdgSurface, surface: &WlSurface) => [
                Uint32(xdg_surface.get_object_id()), Uint32(surface.get_object_id())
            ];
            pong(serial : u32) => [ Uint32(serial) ];
        }

        @events { ping(serial : u32); }
    }

}

//#[derive(Debug)]
//pub enum XdgWmCapabilities {
//    WindowMenu = 1,
//    Maximize = 2,
//    Fullscreen = 3,
//    Minimize = 4,
//}

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
