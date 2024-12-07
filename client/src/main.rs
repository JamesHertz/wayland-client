#![feature(unix_socket_ancillary_data)]
#![allow(dead_code, unused)]
use std::{
    env,
    io::{IoSlice, IoSliceMut, Read},
    os::{
        fd::{AsFd, AsRawFd},
        unix::net::{AncillaryData, SocketAncillary, UnixListener, UnixStream},
    },
    thread,
    time::Duration,
};

use client::{
    client::WaylandClient,
    protocol::{base::*, xdg_shell::*},
    Result,
};

use log::info;

fn main() -> Result<()> {
    utils::init_log();

    let client = WaylandClient::connect(&utils::wayland_sockpath())?;
    info!("Initialization completed!");

    client.event_loop();

    Ok(())
    //let width = 1920;
    //let height = 1080;
    //let stride = 4 * width; // size of a line
    //let window_size = width * height * 4;
    //let (pool, mut pixel_buffer) = client.create_pool(window_size * 2)?;
    //info!("BufferPool created!");
    //
    //{
    //    // set buffer content to be dark
    //    let data = pixel_buffer.as_mut();
    //    for i in (1..window_size as usize).step_by(4) {
    //        data[i] = 255;
    //    }
    //}
    //
    //let buffer: WlBuffer = client.new_object();
    //pool.create_buffer(
    //    &buffer,
    //    0,
    //    width,
    //    height,
    //    stride,
    //    WlShmFormatValue::Xrgb8888,
    //)?;
    //
    //let compositor: WlCompositor =
    //    client.get_global().expect("Failed to get WlCompositor");
    //
    //let surface: WlSurface = client.new_object();
    //compositor.create_surface(&surface)?;
    //
    //let wm_base: XdgWmBase =
    //    client.get_global().expect("Failed to get XdgWmBase");
    //
    //let xdg_surface: XdgSurface = client.new_object();
    //wm_base.get_xdg_surface(&xdg_surface, &surface)?;
    //
    //let xdg_top_level: XdgTopLevel = client.new_object();
    //xdg_surface.get_top_level(&xdg_top_level)?;
    //xdg_top_level.set_app_id("example-app")?;
    //xdg_top_level.set_title("Black Space")?;
    //surface.commit()?;
    //
    //surface.attach(&buffer, 0, 0)?;
    //surface.commit()?;

    //client.event_loop();


    // TODO:
    //  - get a window on the screen
    //  - do some clean up
    //  - think about a way to have event handlers
    //  - keep reading the WaylandBook c:
}

mod utils {

    use log::info;
    use memmap::{MmapMut, MmapOptions};
    use std::{env, fs::File, os::fd::FromRawFd, thread, time::Duration};

    pub fn wayland_sockpath() -> String {
        format!(
            "{}/{}",
            env::var("XDG_RUNTIME_DIR").expect("Failed to get XDG_RUNTIME_DIR var"),
            env::var("WAYLAND_DISPLAY").expect("Failed to get WAYLAND_DISPLAY var"),
        )
    }

    pub fn init_log() {
        if env::var_os("RUST_LOG").is_none() {
            unsafe {
                env::set_var("RUST_LOG", "info");
            }
        }
        pretty_env_logger::init();
    }
}
