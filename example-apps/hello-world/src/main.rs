#![allow(unused)]
use wlclient::{
    client::{memory::SharedBuffer, WaylandClient},
    error::Result,
    protocol::{base::*, xdg_shell::*, WlEventMsg},
};

use std::process;

struct Window {
    // surface info
    surface: WlSurface,
    wm_surface: XdgSurface,
    top_level: XdgTopLevel,
    buffer: Option<WlBuffer>,

    // relevant info c:
    height: i32,
    width: i32,
    pixels: SharedBuffer,
}

const MAX_WIDTH: i32 = 1920;
const MAX_HEIGHT: i32 = 1080;
const MAX_STRIDE: i32 = calc_stride(MAX_WIDTH);
const WINDOW_SIZE: i32 = MAX_WIDTH * MAX_HEIGHT * 4;

#[inline(always)]
const fn calc_stride(width: i32) -> i32 {
    width * 4
}

fn create_window<T>(client: &mut WaylandClient<'_, T>, app_id: &str, title: &str) -> Result<Window> {
    // create surfaces
    let (surface, xdg_surface, top_level) = {
        let compositor: WlCompositor = client.get_global().expect("Failed to get WlCompositor");
        let wm: XdgWmBase = client.get_global().expect("Failed to get XdbWmBase");

        let surface: WlSurface = client.new_object();
        compositor.create_surface(&surface)?;

        let xdg_surface: XdgSurface = client.new_object();
        wm.get_xdg_surface(&xdg_surface, &surface)?;

        let top_level: XdgTopLevel = client.new_object();
        xdg_surface.get_toplevel(&top_level)?;

        top_level.set_app_id(app_id)?;
        top_level.set_title(title)?;

        (surface, xdg_surface, top_level)
    };

    surface.commit()?; // initial empty commit

    let (pool, mut shared_buffer) = client.create_pool(WINDOW_SIZE)?;
    client.upgrade_to_global(&pool)?;

    // set initial background color to white
    shared_buffer.as_mut().fill(255);

    Ok(Window {
        surface,
        top_level,
        buffer: None,
        wm_surface: xdg_surface,

        height: 0,
        width: 0,
        pixels: shared_buffer,
    })
}

fn update(client: &mut WaylandClient<'_, Window>, new_width: i32, new_height: i32) -> Result<()> {
    let window = client.get_custom_state().unwrap();

    let pool: WlShmPool = client.get_global().expect("Failed to get pool");
    let buffer: WlBuffer = client.new_object();
    pool.create_buffer(
        &buffer,
        0,
        new_width,
        new_height,
        calc_stride(new_width),
        WlShmFormat::Xrgb8888,
    )?;

    client.add_event_handler(&buffer, |client, WlEventMsg { object_id, .. }| {
        let buffer: WlBuffer = client.get_reference(object_id).unwrap();
        buffer.destroy();
        client.get_custom_state().map(|window| {
            window.buffer = None;
        });
        assert!(client.get_custom_state().unwrap().buffer.is_none());
    });

    let window = client.get_custom_state().unwrap();

    // TODO: render hello world

    let surface = &window.surface;
    surface.damage_buffer(0, 0, i32::MAX, i32::MAX)?;
    surface.attach(&buffer, 0, 0)?;
    surface.commit()?;

    window.buffer = Some(buffer);
    window.width  = new_width;
    window.height = new_height;
    Ok(())
}

fn main() -> Result<()> {
    wlclient::init_log();
    let mut client = wlclient::connect::<Window>()?;
    let window = create_window(&mut client, "hello-world-app", "Hello World")?;

    // TODO: look which one comes first: xdg_surface @ ... <- configure( ... )
    // or xdg_top_level @ .. <- configure( ... )
    client.add_event_handler(&window.top_level, |client, msg| match msg.event {
        XdgTopLevelEvent::Close => {
            log::info!("Closing window");
            process::exit(0);
        }

        XdgTopLevelEvent::Configure { width, height, .. } => {
            let window = client.get_custom_state().unwrap();
            if window.width != width || window.height != height {
                let Err(error) =  update(client, width, height) else { return };
                log::error!("Updating application state: {error:?}")
            }
        }
        _ => (),
    })?;

    client.add_event_handler(&window.wm_surface, |client, msg| {
        let XdgSurfaceEvent::Configure { serial_nr } = msg.event;
        let xdg_surface: XdgSurface = client.get_reference(msg.object_id).unwrap();
        xdg_surface.ack_configure(serial_nr).unwrap();
    });

    client.set_custom_state(window);
    log::info!("State initialization completed...");

    client.event_loop();
    Ok(())
}
