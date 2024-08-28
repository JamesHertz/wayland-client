#![feature(unix_socket_ancillary_data)]
#![allow(dead_code, unused_imports)]
use std::{
    env,
    io::{IoSlice, IoSliceMut},
    os::{
        fd::{AsFd, AsRawFd},
        unix::net::{AncillaryData, SocketAncillary, UnixListener, UnixStream},
    },
    thread,
    time::Duration,
};


use client::{Result, client::WaylandClient};

use log::info;
// use memmap::MmapOptions;
// // use utils::join_shared_memory;
// use client::protocol::{
//     api::{ShmPixelFormat, WaylandObject, WaylandRequest, TopLevelState},
//     WaylandClient,
// };

fn main() -> Result<()> {
    utils::init_log();

    let client = WaylandClient::connect(&utils::wayland_sockpath())?;
    // client.load_interfaces()?;
    loop {}

    // info!("Waiting to see if any error will arrive!");
    // thread::sleep(Duration::from_secs(30));

    // let messages = client.pull_messages()?;
    // info!("Gotten messages {messages:?}");
    //
    // // 1920x1080
    // let width  : i32 = 1920; 
    // let height : i32 = 1080;
    // // let width  : i32 = 960; 
    // // let height : i32 = 1030;
    // let stride : i32 = 4 * width;
    // let window_size : i32 = 4 * height * stride;
    // let pool_size   : i32 = 2 * window_size;
    // info!("Pool size : {}", pool_size);
    //
    // let (pool_id, mem_buffer_id) = client.create_pool(pool_size as usize)?;
    // {
    //     let mem = client.get_shared_buffer(mem_buffer_id).unwrap();
    //     mem.data.as_mut().fill(0);
    //     info!("Mem size {}", mem.data.len())
    // }
    //
    // info!("Created pool {pool_id} ...");
    // thread::sleep(Duration::from_secs(1));
    //
    // let buffer_id = client.new_id(WaylandObject::Buffer);
    // client.send_request(
    //     pool_id, WaylandRequest::ShmPoolCreateBuffer { 
    //         buffer_id,  offset: 0, width, height, stride, pixel_format: ShmPixelFormat::Xrgb
    //     }
    // )?;
    //
    // info!("Created Buffer {buffer_id} ...");
    //
    // let surface_id = {
    //     let compositor_id = client.get_global_mapping(
    //         WaylandObject::Compositor
    //     ).unwrap();
    //     let surface_id= client.new_id(WaylandObject::Surface);
    //     client.send_request(
    //         compositor_id,
    //         WaylandRequest::CompositorCreateSurface(surface_id)
    //     )?;
    //     info!("Created surface {surface_id} ... ");
    //     thread::sleep(Duration::from_secs(1));
    //
    //
    //     // client.send_request(
    //     //     surfarce_id, 
    //     //     WaylandRequest::SufaceCommit
    //     // )?;
    //     // info!("Commited Surface");
    //     surface_id
    // };
    //
    // let window_xdg_surface_id = {
    //     let xdg_wm_base_id = client.get_global_mapping(
    //         WaylandObject::XdgWmBase
    //     ).unwrap();
    //
    //     let xdg_suface_id = client.new_id(WaylandObject::XdgSurface);
    //     client.send_request(
    //         xdg_wm_base_id, 
    //         WaylandRequest::XdgWmGetSurface { 
    //             new_id: xdg_suface_id, surface: surface_id
    //         }
    //     )?;
    //
    //     info!("Got an xdg_suface {xdg_suface_id} ...");
    //     thread::sleep(Duration::from_secs(1));
    //
    //     let top_level_id = client.new_id(WaylandObject::XdgTopLevel);
    //     client.send_request(
    //         xdg_suface_id,
    //         WaylandRequest::XdgSurfaceGetTopLevel(top_level_id)
    //     )?;
    //
    //     info!("Got an xdg_top_level {top_level_id} ...");
    //     thread::sleep(Duration::from_secs(1));
    //
    //     top_level_id
    // };
    //
    // client.send_request(
    //     window_xdg_surface_id,
    //     WaylandRequest::XdgTopLevelSetTitle("Example app".to_string())
    // )?;
    //
    // info!("Set xdg_top_level {window_xdg_surface_id} to 'Example app' ...");
    // thread::sleep(Duration::from_secs(1));
    //
    // client.send_request(
    //     surface_id, 
    //     WaylandRequest::SufaceCommit
    // )?;
    //
    // info!("Committed surface");
    // thread::sleep(Duration::from_secs(1));
    //
    // client.send_request(
    //     surface_id, 
    //     WaylandRequest::SufaceAttach { 
    //         buffer_id, x: 0, y : 0 
    //     }
    // )?;
    //
    // info!("Attached surface {surface_id} to buffer {buffer_id} ...");
    // thread::sleep(Duration::from_secs(1));
    //
    // let mem = client.get_shared_buffer(mem_buffer_id).unwrap();
    // for i in (1..window_size).step_by(4) {
    //     mem.data.as_mut()[i as usize] = 255;
    // }
    //
    // client.send_request(
    //     surface_id, 
    //     WaylandRequest::SufaceCommit
    // )?;
    //
    // info!("Committed surface");
    // info!("Waiting for errors");
    //
    //
    //
    // loop{}
    //
    // let shm_id = client.get_global_mapping(WaylandObject::Shm).unwrap();
    // let shared_buffer = client.create_buffer(1024 * 1024 * 4)?;

    // shared_buffer.data.len();


    // let compositor   = client.get_global(WaylandObject::Compositor).unwrap();
    // let sufurface_id = client.new_id(WaylandObject::Surface);
    // client.send_request(
    //     compositor, WaylandRequest::CompositorCreateSurface(sufurface_id)
    // )?;
    //
    // let wm = client.get_global(WaylandObject::XdgWmBase).unwrap();
    // let xdg_surface_id = client.new_id(WaylandObject::XdgSurface);
    // client.send_request(
    //     wm, WaylandRequest::XdgWmGetSurface { new_id: xdg_surface_id, surface: sufurface_id }
    // )?;
    //
    // let messages = client.pull_msgs();
    // info!("You've got {:?} messages", messages);
    Ok(())
}

mod utils {

    use color_eyre::eyre::eyre;
    use log::info;
    use memmap::{MmapMut, MmapOptions};
    use std::{env, fs::File, os::fd::FromRawFd, thread, time::Duration};

    pub fn wayland_sockpath() -> String {
        format!(
            "{}/{}",
            env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR var "),
            env::var("WAYLAND_DISPLAY").expect("WAYLAND_DISPLAY var "),
        )
    }

    pub fn init_log() {
        // TODO: ADD TIME TO LOG LINES
        if env::var_os("RUST_LOG").is_none() {
            unsafe {
                env::set_var("RUST_LOG", "info");
            }
        }
        pretty_env_logger::init();
    }

}
