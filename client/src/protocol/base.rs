use super::*;
use crate::{error::{self, fallback_error}, wire_format::parsing as parser};
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
            WlDisplayDeleteId ( object )
        }
    }
);

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

declare_interface!(
    WlCallBack,
    @iterator  = iter,
    @object_id = obj_id,
    @branches  = {
        0 =>  {
            let cb_data = parser::parse_u32(&mut iter)?;
            debug!( 
                "{obj_id} @ {:?} <- done ( {cb_data} )",
                WlCallBack::get_interface_id()
            );

            WlCallBackDone( cb_data )
        }
    }
);

declare_interface!(WlCompositor);
declare_interface!(XdgWmBase);

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum WlShmFormatValue { 
    Argb8888 = 0, Xrgb8888 = 1, Other(u32)
    // TODO: add all the remaining...
    
    // C8, Rgb332, Bgr233, Xrgb4444, Xbgr4444, Rgbx4444,
    // Bgrx4444, Argb4444, Abgr4444, Rgba4444, Bgra4444, Xrgb1555, Xbgr1555,
    // Rgbx5551, Bgrx5551, Argb1555, Abgr1555, Rgba5551, Bgra5551, Rgb565,
    // Bgr565,   Rgb888,   Bgr888,   Xbgr8888, Rgbx8888, Bgrx8888, Abgr8888,
    // Rgba8888, Bgra8888, Xrgb2101010, Xbgr2101010, Rgbx1010102, Bgrx1010102,
    // Argb2101010, Abgr2101010, Rgba1010102, Bgra1010102, Yuyv, Yvyu, Uyvy,
    // Vyuy, Ayuv, Nv12, Nv21, Nv16, Nv61, Yuv410, Yvu410, Yuv411, Yvu411,
    // Yuv420, Yvu420, Yuv422, Yvu422, Yuv444, Yvu444, R8, R16, Rg88, Gr88,
    // Rg1616, Gr1616, Xrgb16161616f, Xbgr16161616f, Argb16161616f, Abgr16161616f,
    // Xyuv8888, Vuy888, Vuy101010, Y210, Y212, Y216, Y410, Y412, Y416,
    // Xvyu2101010, Xvyu12_16161616, Xvyu16161616, Y0l0, X0l0, Y0l2, X0l2,
    // Yuv420_8bit, Yuv420_10bit, Xrgb8888_a8, Xbgr8888_a8, Rgbx8888_a8,
    // Bgrx8888_a8, Rgb888_a8, Bgr888_a8, Rgb565_a8, Bgr565_a8, Nv24, Nv42, P210,
    // P010, P012, P016, Axbxgxrx106106106106, Nv15, Q410, Q401, Xrgb16161616,
    // Xbgr16161616, Argb16161616, Abgr16161616, C1, C2, C4, D1, D2, D4, D8,
    // R1, R2, R4, R10, R12, Avuy8888, Xvuy8888, P030 
} 

declare_interface!(
    WlShm,
    @iterator  = iter,
    @object_id = obj_id,
    @branches  = {
        0 => {
            let format = match parser::parse_u32(&mut iter)? {
                0 => WlShmFormatValue::Argb8888,
                1 => WlShmFormatValue::Xrgb8888,
                value => WlShmFormatValue::Other(value)
            };

            debug!( 
                "{obj_id} @ {:?} <- format ( {format:?} )",
                WlCallBack::get_interface_id()
            );
            WlShmFormat( format )
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

impl WlRegistry {

    pub fn bind( &self, 
        name: u32,
        interface: String,
        version: u32,
        new_id: WaylandId,
    ) -> Result<usize> {
        debug!(
            "{} @ {:?} -> bind( {name}, {interface:?}, {version}, {new_id} )",
            self.get_object_id(),
            Self::get_interface_id()
        );

        self.0.stream.send(WireMessage {
            object_id: self.get_object_id(),
            request_id: 0,
            values: &[
                Uint32(name), Str(interface), 
                Uint32(version), Uint32(new_id)
            ],
        })
    }

}
