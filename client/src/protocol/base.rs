use super::*;
use crate::{
    error::{self, fallback_error},
    wire_format::parsing as parser,
};
use fmt::Display;
use log::debug;
use std::convert::TryFrom;

pub struct WlDisplay(WlObjectMetaData);

#[derive(Debug)]
pub enum WlDisplayEvent {
    Error { object: u32, code: u32, message: String },
    DeletedId(u32),
}

impl WlInterface for WlDisplay {
    type Event = WlDisplayEvent;

    fn get_interface_id() -> WlInterfaceId {
        0
    }

    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self {
        WlDisplay(WlObjectMetaData { object_id, stream })
    }

    fn get_object_id(&self) -> WaylandId {
        self.0.object_id
    }

    fn parse_event(
        object_id: WaylandId,
        event_id: u16,
        iter: &mut impl Iterator<Item = u8>,
    ) -> Result<Self::Event, WlEventParseError> {
        Ok(match event_id {
            0 => {
                let (object, code, message) = (
                    parser::parse_u32(iter)?,
                    parser::parse_u32(iter)?,
                    parser::parse_str(iter)?,
                );

                debug!("{object_id} @ WlDisplay <- display_error( {object}, {code}, {message:?}");
                Self::Event::Error {
                    object,
                    code,
                    message,
                }
            }
            1 => {
                let object = parser::parse_u32(iter)?;
                debug!("{object_id} @ WlDisplay <- delete_id ( {object} )");
                Self::Event::DeletedId(object)
            }
            id => return Err(WlEventParseError::NoEvent(id)),
        })
    }
}

pub struct WlRegistry(WlObjectMetaData);

#[derive(Debug)]
pub enum WlRegistryEvent {
    Global {
        name: u32,
        interface: String,
        version: u32,
    },
}

impl WlInterface for WlRegistry {
    type Event = WlRegistryEvent;

    fn get_object_id(&self) -> WaylandId {
        self.0.object_id
    }

    fn get_interface_id() -> WlInterfaceId {
        1
    }

    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self {
        WlRegistry(WlObjectMetaData { object_id, stream })
    }

    fn parse_event(
        object_id: WaylandId,
        event_id: WlEventId,
        iter: &mut impl Iterator<Item = u8>,
    ) -> Result<Self::Event, WlEventParseError> {
        if event_id == 0 {
            let (name, interface, version) = (
                parser::parse_u32(iter)?,
                parser::parse_str(iter)?,
                parser::parse_u32(iter)?,
            );

            debug!("{object_id} @ WlRegistry <- global ( {name}, {interface:?}, {version} )");
            Ok(WlRegistryEvent::Global {
                name,
                interface,
                version,
            })
        } else {
            Err(WlEventParseError::NoEvent(event_id))
        }
    }
}

impl WlDisplay {
    pub fn get_registry(&self, registry: &WlRegistry) -> error::Result<usize> {
        debug!(
            "{} @ WlDisplay -> get_registry( {} )",
            //self.get_object_id(),
            Self::get_interface_id(),
            registry.get_object_id()
        );

        self.0.stream.send(WireMessage {
            object_id: self.get_object_id(),
            request_id: 1,
            values: &[Uint32(registry.get_object_id())],
        })
    }

    pub fn sync(&self, callback: &WlCallBack) -> error::Result<usize> {
        debug!(
            "{} @ WlDisplay -> sync ( {} )",
            //self.get_object_id(),
            Self::get_interface_id(),
            callback.get_object_id()
        );

        self.0.stream.send(WireMessage {
            object_id: self.get_object_id(),
            request_id: 0,
            values: &[Uint32(callback.get_object_id())],
        })
    }


}

impl WlRegistry {
    pub fn bind(
        &self,
        name: u32,
        interface: String,
        version: u32,
        new_id: WaylandId,
    ) -> error::Result<usize> {
        debug!(
            "{} @ WlRegistry -> bind( {name}, {interface:?}, {version}, {new_id} )",
            //self.get_object_id(),
            Self::get_interface_id()
        );
        self.0.stream.send(WireMessage {
            object_id: self.get_object_id(),
            request_id: 0,
            values: &[Uint32(name), Str(interface), Uint32(version), Uint32(new_id)],
        })
    }
}

pub struct WlCallBack(WlObjectMetaData);

#[derive(Debug)]
pub struct WlCallBackDone (u32);

impl WlInterface for WlCallBack {
    type Event = WlCallBackDone;

    fn get_object_id(&self) -> WaylandId {
        self.0.object_id
    }

    fn get_interface_id() -> WlInterfaceId {
        2
    }

    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self {
        Self(WlObjectMetaData { object_id, stream })
    }

    fn parse_event(
        object_id: WaylandId,
        event_id: WlEventId,
        iter: &mut impl Iterator<Item = u8>,
    ) -> Result<Self::Event, WlEventParseError> {
        if event_id == 0 {
            let data = parser::parse_u32(iter)?;

            debug!("{object_id} @ WlCallBack <- done ( {data} )");
            Ok(WlCallBackDone(data))
        } else {
            Err(WlEventParseError::NoEvent(event_id))
        }
    }
}

pub struct WlCompositor(u32);
pub struct XdgWmBase(u32);
pub struct WlShm(u32);

impl WlInterface for WlCompositor {
    type Event = ();

    fn get_object_id(&self) -> WaylandId {
        self.0
    }

    fn get_interface_id() -> WlInterfaceId {
        3
    }

    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self {
        Self(object_id)
    }

    fn parse_event(
        object_id: WaylandId,
        event_id: WlEventId,
        iter: &mut impl Iterator<Item = u8>,
    ) -> Result<Self::Event, WlEventParseError> {
        todo!()
    }
}

impl WlInterface for XdgWmBase {
    type Event = ();

    fn get_object_id(&self) -> WaylandId {
        self.0
    }

    fn get_interface_id() -> WlInterfaceId {
        4
    }

    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self {
        Self(object_id)
    }

    fn parse_event(
        object_id: WaylandId,
        event_id: WlEventId,
        iter: &mut impl Iterator<Item = u8>,
    ) -> Result<Self::Event, WlEventParseError> {
        todo!()
    }
}

impl WlInterface for WlShm {
    type Event = ();

    fn get_object_id(&self) -> WaylandId {
        self.0
    }

    fn get_interface_id() -> WlInterfaceId {
        5
    }

    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self {
        Self(object_id)
    }

    fn parse_event(
        object_id: WaylandId,
        event_id: WlEventId,
        iter: &mut impl Iterator<Item = u8>,
    ) -> Result<Self::Event, WlEventParseError> {
        todo!()
    }
}



/*

declare_interfaces {
    interface {
        @name(WlDisplay),
        @requests {
            sync(callback : &WlCallBack) => [
                Uint32(callback.get_object_id())
            ],
            get_registry(registry : &WlRegistry) => [
                Uint32(registry.get_object_id())
            ],
        }
        @events(WlDisplayEvent, object_id, iter) {
            display_error(object : u32, code : u32, message : String),
            delete_id(object : u32)
        }
    }

}



*/

//declare_interface!(
//    @name(WlDisplay),
//    @events(obj_id, iter) {
//        0 => {
//            let (object, code, message) = (
//                parser::parse_u32(&mut iter)?,
//                parser::parse_u32(&mut iter)?,
//                parser::parse_str(&mut iter)?
//            );
//
//            debug!(
//                "{obj_id} @ {:?} <- display_error( {object}, {code}, {message:?}",
//                Self::get_interface_id()
//            );
//            WlDisplayError{ object, code, message }
//        },
//
//        1 => {
//            let object = parser::parse_u32(&mut iter)?;
//            debug!(
//                "{obj_id} @ {:?} <- delete_id ( {object} )",
//                Self::get_interface_id()
//            );
//            WlDisplayDeleteId ( object )
//        }
//    }
//);
//
//declare_interface!(
//    @name(WlRegistry),
//    @events(obj_id, iter) {
//        0 => {
//            let (name, interface, version) = (
//                parser::parse_u32(&mut iter)?,
//                parser::parse_str(&mut iter)?,
//                parser::parse_u32(&mut iter)?
//            );
//
//            debug!(
//                "{obj_id} @ {:?} <- global ( {name}, {interface:?}, {version} )",
//                Self::get_interface_id()
//            );
//
//            WlRegistryGlobal { name, interface, version }
//        }
//    }
//);
//
//declare_interface!(
//    @name(WlCallBack),
//    @events(obj_id, iter) {
//        0 =>  {
//            let cb_data = parser::parse_u32(&mut iter)?;
//            debug!(
//                "{obj_id} @ {:?} <- done ( {cb_data} )",
//                Self::get_interface_id()
//            );
//
//            WlCallBackDone( cb_data )
//        }
//    }
//);
//
//declare_interface!(
//    @name(WlShm),
//    @events(obj_id, iter) {
//        0 => {
//            let format = match parser::parse_u32(&mut iter)? {
//                0 => WlShmFormatValue::Argb8888,
//                1 => WlShmFormatValue::Xrgb8888,
//                value => WlShmFormatValue::Other(value)
//            };
//
//            debug!(
//                "{obj_id} @ {:?} <- format ( {format:?} )",
//                Self::get_interface_id()
//            );
//            WlShmFormat( format )
//        }
//    }
//);
//
//declare_interface!(WlCompositor);
//declare_interface!(WlShmPool);
//declare_interface!(
//    @name(WlBuffer),
//    @events(obj_id, _iter) {
//        0 => {
//            debug!("{obj_id} @ {:?} <- release( )", Self::get_interface_id());
//            WlBufferRelease
//        }
//    }
//);
//
//declare_interface!(
//    @name(WlSurface),
//    @events(obj_id, iter) {
//        2 => {
//            let factor = parser::parse_u32(&mut iter)?;
//            debug!(
//                "{obj_id} @ {:?} <- preferred_buffer_scale( {factor} )",
//                Self::get_interface_id()
//            );
//
//            WlSurfacePreferredBufferScale( factor )
//        },
//        3 => {
//            let transformation = parser::parse_u32(&mut iter)?.try_into()?;
//
//            debug!(
//                "{obj_id} @ {:?} preferred_buffer_transform( {transformation:?} )",
//                Self::get_interface_id()
//            );
//
//            WlSurfacePreferredBufferTransform( transformation )
//        }
//    }
//);
//
//#[derive(Debug, Clone, Copy)]
//#[repr(u8)]
//pub enum WlShmFormatValue {
//    Argb8888 = 0,
//    Xrgb8888 = 1,
//    Other(u32), // TODO: add all the remaining...
//
//                // C8, Rgb332, Bgr233, Xrgb4444, Xbgr4444, Rgbx4444,
//                // Bgrx4444, Argb4444, Abgr4444, Rgba4444, Bgra4444, Xrgb1555, Xbgr1555,
//                // Rgbx5551, Bgrx5551, Argb1555, Abgr1555, Rgba5551, Bgra5551, Rgb565,
//                // Bgr565,   Rgb888,   Bgr888,   Xbgr8888, Rgbx8888, Bgrx8888, Abgr8888,
//                // Rgba8888, Bgra8888, Xrgb2101010, Xbgr2101010, Rgbx1010102, Bgrx1010102,
//                // Argb2101010, Abgr2101010, Rgba1010102, Bgra1010102, Yuyv, Yvyu, Uyvy,
//                // Vyuy, Ayuv, Nv12, Nv21, Nv16, Nv61, Yuv410, Yvu410, Yuv411, Yvu411,
//                // Yuv420, Yvu420, Yuv422, Yvu422, Yuv444, Yvu444, R8, R16, Rg88, Gr88,
//                // Rg1616, Gr1616, Xrgb16161616f, Xbgr16161616f, Argb16161616f, Abgr16161616f,
//                // Xyuv8888, Vuy888, Vuy101010, Y210, Y212, Y216, Y410, Y412, Y416,
//                // Xvyu2101010, Xvyu12_16161616, Xvyu16161616, Y0l0, X0l0, Y0l2, X0l2,
//                // Yuv420_8bit, Yuv420_10bit, Xrgb8888_a8, Xbgr8888_a8, Rgbx8888_a8,
//                // Bgrx8888_a8, Rgb888_a8, Bgr888_a8, Rgb565_a8, Bgr565_a8, Nv24, Nv42, P210,
//                // P010, P012, P016, Axbxgxrx106106106106, Nv15, Q410, Q401, Xrgb16161616,
//                // Xbgr16161616, Argb16161616, Abgr16161616, C1, C2, C4, D1, D2, D4, D8,
//                // R1, R2, R4, R10, R12, Avuy8888, Xvuy8888, P030
//}
//
//
//// TODO: implement from for this c:
//impl WlShmFormatValue {
//    fn into_u32(self) -> u32 {
//        match self {
//            WlShmFormatValue::Argb8888 => 0,
//            WlShmFormatValue::Xrgb8888 => 1,
//            WlShmFormatValue::Other(value) => value,
//        }
//    }
//}
//
//
//#[derive(Debug, Clone, Copy)]
//pub enum WlOutputTransform {
//    Normal      = 0,
//    Turned90    = 1,
//    Turned180   = 2,
//    Turned270   = 3,
//    Flipped     = 4,
//    Flipped90   = 5,
//    Flipped180  = 6,
//    Flipped270  = 7
//}
//
//impl TryFrom<u32> for WlOutputTransform {
//
//    type Error = error::Error;
//    fn try_from(value : u32) -> Result<Self> {
//        Ok(
//            match value {
//                0 => Self::Normal,
//                1 => Self::Turned90,
//                2 => Self::Turned180,
//                3 => Self::Turned270,
//                4 => Self::Flipped,
//                5 => Self::Flipped90,
//                6 => Self::Flipped180,
//                7 => Self::Flipped270,
//                other => {
//                    return Err(fallback_error!("Invalid WlOutputTransfrom value '{other}' it should be in range 0..7"));
//                }
//            }
//        )
//
//    }
//}
//
//impl WlDisplay {
//    pub fn sync(&self, callback: &WlCallBack) -> Result<usize> {
//        debug!(
//            "{} @ {:?} -> sync( {} )",
//            self.get_object_id(),
//            Self::get_interface_id(),
//            callback.get_object_id()
//        );
//
//        self.0.stream.send(WireMessage {
//            object_id: self.get_object_id(),
//            request_id: 0,
//            values: &[Uint32(callback.get_object_id())],
//        })
//    }
//
//    pub fn get_registry(&self, registry: &WlRegistry) -> Result<usize> {
//        debug!(
//            "{} @ {:?} -> get_registry( {} )",
//            self.get_object_id(),
//            Self::get_interface_id(),
//            registry.get_object_id()
//        );
//
//        self.0.stream.send(WireMessage {
//            object_id: self.get_object_id(),
//            request_id: 1,
//            values: &[Uint32(registry.get_object_id())],
//        })
//    }
//}
//
//impl WlRegistry {
//    pub fn bind(
//        &self,
//        name: u32,
//        interface: String,
//        version: u32,
//        new_id: WaylandId,
//    ) -> Result<usize> {
//        debug!(
//            "{} @ {:?} -> bind( {name}, {interface:?}, {version}, {new_id} )",
//            self.get_object_id(),
//            Self::get_interface_id()
//        );
//
//        self.0.stream.send(WireMessage {
//            object_id: self.get_object_id(),
//            request_id: 0,
//            values: &[
//                Uint32(name),
//                Str(interface),
//                Uint32(version),
//                Uint32(new_id),
//            ],
//        })
//    }
//}
//
//
//
//impl WlShm {
//    pub fn create_pool(
//        &self,
//        pool: &WlShmPool,
//        file_descriptor: i32,
//        size: i32,
//    ) -> Result<usize> {
//        debug!(
//            "{} @ {:?} -> create_pool ( {}, {file_descriptor}, {size} )",
//            self.get_object_id(),
//            Self::get_interface_id(),
//            pool.get_object_id()
//        );
//
//        self.0.stream.send(WireMessage {
//            object_id: self.get_object_id(),
//            request_id: 0,
//            values: &[
//                Uint32(pool.get_object_id()),
//                FileDesc(file_descriptor),
//                Int32(size),
//            ],
//        })
//    }
//}
//
//impl WlCompositor {
//    pub fn create_surface(&self, surface: &WlSurface) -> Result<usize> {
//        debug!(
//            "{} @ {:?} -> create_surface ( {} )",
//            self.get_object_id(),
//            Self::get_interface_id(),
//            surface.get_object_id()
//        );
//
//        self.0.stream.send(WireMessage {
//            object_id: self.get_object_id(),
//            request_id: 0,
//            values: &[Uint32(surface.get_object_id())],
//        })
//    }
//}
//
//impl WlShmPool {
//    pub fn create_buffer(
//        &self,
//        buffer: &WlBuffer,
//        offset: i32,
//        width: i32,
//        height: i32,
//        stride: i32,
//        format: WlShmFormatValue,
//    ) -> Result<usize> {
//        let format = format.into_u32();
//
//        debug!(
//            "{} @ {:?} -> create_buffer ( {}, {offset}, {width}, {height}, {stride}, {format} )",
//            self.get_object_id(),
//            Self::get_interface_id(),
//            buffer.get_object_id()
//        );
//
//        self.0.stream.send(WireMessage {
//            object_id: self.get_object_id(),
//            request_id: 0,
//            values: &[
//                Uint32(buffer.get_object_id()),
//                Int32(offset),
//                Int32(width),
//                Int32(height),
//                Int32(stride),
//                Uint32(format),
//            ],
//        })
//    }
//}
//
//
//impl WlSurface {
//
//    pub fn attach(&self, buffer : &WlBuffer, x : i32, y : i32) -> Result<usize> {
//        debug!(
//            "{} @ {:?} -> attach( {}, {x}, {y})",
//            self.get_object_id(),
//            Self::get_interface_id(),
//            buffer.get_object_id()
//        );
//
//        self.0.stream.send(WireMessage {
//            object_id: self.get_object_id(),
//            request_id: 1,
//            values: &[
//                Uint32(buffer.get_object_id()),
//                Int32(x),
//                Int32(y),
//            ],
//        })
//
//    }
//
//    pub fn commit(&self) -> Result<usize> {
//        debug!(
//            "{} @ {:?} -> commit()",
//            self.get_object_id(),
//            Self::get_interface_id(),
//        );
//
//        self.0.stream.send(WireMessage {
//            object_id: self.get_object_id(),
//            request_id: 6,
//            values: &[],
//        })
//
//    }
//}
