use std::process;
use wlclient::{client::{memory::SharedBuffer, WaylandClient}, error::Result, protocol::{base::*, xdg_shell::*}};

use log::info;

struct State {
    surface   : WlSurface,
    pixels    : SharedBuffer,
    buffer    : WlBuffer,
    turn      : u32,
    last_time : u32,
    released  : bool,
    window_size : i32
}

fn update(client : &mut WaylandClient<'_, State>, current_time : u32) {
    let state = client.get_custom_state().unwrap();

    if current_time - state.last_time >= 500 && state.released {
        let window_size = state.window_size;
        let turn = state.turn;
        let data = state.pixels.as_mut();

        for i in (0..window_size as usize).step_by(4) {
            data[i + 0] = 0;
            data[i + 1] = 0;
            data[i + 2] = 0;

            data[i + turn as usize] = 255;
        }

        let surface = &state.surface;
        let buffer  = &state.buffer;

        surface.damage_buffer(0, 0, i32::MAX, i32::MAX).unwrap();
        surface.attach(buffer, 0, 0).unwrap();
        surface.commit().unwrap();

        state.last_time = current_time;
        state.turn      = (turn + 1) % 3;
        state.released  = false;
    }

    let cb : WlCallBack  = client.new_object();
    let state = client.get_custom_state().unwrap();
    state.surface.frame(&cb).unwrap();

    client.add_event_handler(&cb, |client, msg| {
        let WlCallBackEvent::Done { data } = msg.event;
        update(client, data);
    }).unwrap();
}

fn main() -> Result<()> {
    wlclient::init_log();

    //let mut client = WaylandClient::<'_, State>::connect_to(&wayland_sockpath())?;
    let mut client = wlclient::connect::<State>()?;
//let mut client = WaylandClient::<State>::connect()?;
    info!("Initialization completed!");

    let width = 1920;
    let height = 1080;
    let stride = 4 * width; // size of a line
    let window_size = width * height * 4;
    let (pool, mut pixels) = client.create_pool(window_size)?;
    info!("BufferPool created!");

    pixels.as_mut().fill(0);

    let buffer: WlBuffer = client.new_object();
    pool.create_buffer(
        &buffer,
        0,
        width,
        height,
        stride,
        WlShmFormat::Xrgb8888,
    )?;

    let compositor: WlCompositor =
        client.get_global().expect("Failed to get WlCompositor");

    let surface: WlSurface = client.new_object();
    compositor.create_surface(&surface)?;

    let wm_base: XdgWmBase =
        client.get_global().expect("Failed to get XdgWmBase");

    let xdg_surface: XdgSurface = client.new_object();
    wm_base.get_xdg_surface(&xdg_surface, &surface)?;

    let xdg_top_level: XdgTopLevel = client.new_object();
    xdg_surface.get_toplevel(&xdg_top_level)?;
    xdg_top_level.set_app_id("example-app")?;
    xdg_top_level.set_title("Black Space")?;

    surface.commit()?;

    surface.attach(&buffer, 0, 0)?;
    surface.commit()?;

    client.add_event_handler(&buffer, |client, _| {
        client.get_custom_state().unwrap().released = true;
    })?;

    client.set_custom_state(State {
        surface, buffer, pixels, window_size, 
        turn : 0, last_time: 0, released: false
    });


    client.add_event_handler(&xdg_top_level, move |client, msg| {
        match msg.event {
            XdgTopLevelEvent::Close => {
                info!("Closing window");
                process::exit(0);
            }

            XdgTopLevelEvent::Configure { width, height, .. } => {
                let state = client.get_custom_state().unwrap();
                let new_window_size = 4 * height * width;

                if new_window_size != state.window_size {
                    state.window_size = new_window_size;

                    let buffer : WlBuffer = client.new_object();
                    pool.create_buffer(
                        &buffer,
                        0,
                        width,
                        height,
                        4 * width,
                        WlShmFormat::Xrgb8888,
                    ).unwrap();

                    client.add_event_handler(&buffer, |client, _| {
                        client.get_custom_state().unwrap().released = true;
                    }).unwrap();

                    let state = client.get_custom_state().unwrap();
                    state.buffer.destroy().unwrap();
                    state.buffer   = buffer;
                    state.released = true;
                }
            },
            _ => ()
        }
    })?;

    client.add_event_handler(&xdg_surface, |client, msg| {
        let XdgSurfaceEvent::Configure { serial_nr } = msg.event;
        let xdg_surface : XdgSurface = client.get_reference(msg.object_id).unwrap();
        xdg_surface.ack_configure(serial_nr).unwrap();
    })?;

    update(&mut client, 0);
    client.event_loop();
    Ok(())

    // TODO:
    //  - keep reading the WaylandBook c:
    //  - Switch between red and green (when the user clicks somewhere on the screen)
}

