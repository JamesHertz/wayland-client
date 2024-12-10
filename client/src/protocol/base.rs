use super::declare_interfaces;

declare_interfaces !{
    FirstId = 0,
    WlRegion,

    @interface(WlCallBack) { @events { done(data : u32); } },

    @interface(WlDisplay) {
        @requests {
            sync(callback : &WlCallBack) => [Uint32(callback.get_object_id())];
            get_registry(registry : &WlRegistry) => [Uint32(registry.get_object_id())];
        }

        @events {
            error(object_id: u32, code: u32, message: String);
            delete_id(id: u32);
        }
    },

    @interface(WlRegistry) {
        @requests {
            bind(name: u32, interface: String, version: u32, new_id: WaylandId) => [
                Uint32(name), Str(interface), Uint32(version), Uint32(new_id),
            ];
        }

        @events { global(name: u32, interface: String, version: u32); }
    },

    @interface(WlCompositor) {
        @requests {
            create_surface(surface: &WlSurface) => [ Uint32(surface.get_object_id()) ];
        }
    },

    @interface(WlSurface){
        @requests {
            destroy();
            attach(buffer : &WlBuffer, x : i32, y : i32)    => [ Uint32(buffer.get_object_id()), Int32(x), Int32(y) ];
            damage(x: i32, y: i32, width: i32, height: i32) => [ Int32(x), Int32(y), Int32(width), Int32(height) ];
            frame(callback: &WlCallBack)         => [ Uint32(callback.get_object_id()) ];
            set_opaque_region(region: &WlRegion) => [ Uint32(region.get_object_id()) ];
            set_input_region(region: &WlRegion)  => [ Uint32(region.get_object_id()) ];
            commit();
            set_buffer_transform(transformation : i32) => [ Int32(transformation) ]; // TODO: add enum for transformation
            set_buffer_scale(scale : i32) => [ Int32(scale) ];
            damage_buffer(x: i32, y: i32, width: i32, height: i32) => [ Int32(x), Int32(y), Int32(width), Int32(height) ];
        }

        @events {
            enter(output_id : u32);
            leave(output_id : u32);
            preferred_buffer_scale(factor : i32);
            preferred_buffer_transform(output_transform : u32);
        }
    },

    // shared memory stuffs
    @interface(WlShm) {
        @requests {
            create_pool(pool: &WlShmPool, file_descriptor: i32, size: i32) => [
                Uint32(pool.get_object_id()), FileDesc(file_descriptor), Int32(size),
            ];
        }
        @events { format( format_value : u32 ); } 
    },

    @interface(WlShmPool) {
        @requests {
            create_buffer(buffer: &WlBuffer, offset: i32, width: i32, height: i32, stride: i32, format : WlShmFormat) => [
                Uint32(buffer.get_object_id()), Int32(offset), Int32(width),
                Int32(height), Int32(stride), Uint32(format as u32),
            ];
            destroy();
            resize(size: i32) => [Int32(size)];
        }
    },

    @interface(WlBuffer) { 
        @requests { destroy(); } 
        @events   { release(); } 
    },
}


#[repr(u32)]
pub enum WlShmFormat {
    Argb8888 = 0,
    Xrgb8888 = 1	
}

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
