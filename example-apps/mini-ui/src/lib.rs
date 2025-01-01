#![allow(unused)]
use std::{mem, process};

use wlclient::{
    client::memory::SharedBuffer,
    protocol::{
        self,
        base::{WlBuffer, WlCompositor, WlShmFormat, WlShmPool, WlSurface},
        xdg_shell::{XdgSurface, XdgSurfaceEvent, XdgTopLevel, XdgTopLevelEvent, XdgWmBase}, WlEventMsg,
    },
    WaylandClient,
};

pub use wlclient::error::Result;

#[derive(Clone, Copy)]
pub enum Color {
    Red,
    Green,
    Blue,
    White,
    Black,
    Cyan,
    Rgb(u8, u8, u8),
    Argb(u8, u8, u8, u8),
}

impl From<Color> for u32 {
    fn from(value: Color) -> Self {
        let (a, r, g, b) = match value {
            Color::Red   => (255, 255, 0, 0),
            Color::Cyan   => (255, 0, 255, 255),
            Color::Green => (255, 0, 255, 0),
            Color::Blue  => (255, 0, 0, 255),
            Color::White => (255, 255, 255, 255),
            Color::Black => (255, 0, 0, 0),
            Color::Rgb(r, g, b) => (255, r as u32, g as u32, b as u32),
            Color::Argb(a, r, g, b) => (a as u32, r as u32, g as u32, b as u32),
        };
        a << 24 | r << 16 | g << 8 | b
    }
}

pub enum UIEvent {
    Initialize,
    Resize,
    Exit
}

pub struct Screen<'a> {
    width: u32,
    height: u32,
    pixels: &'a mut [u32],
}

impl Screen <'_> {

    pub fn fill(&mut self, color: Color) {
        let color = u32::from(color);
        self.pixels.fill(color);
    }

    pub fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: Color) {
        let window_height = self.height;
        let window_width = self.width;

        let start_x = u32::min(window_width, x);
        let end_x = u32::min(window_width, x + width);

        let start_y = u32::min(window_height, y);
        let end_y = u32::min(window_height, y + height);

        let color = u32::from(color);

        for y in start_y..end_y {
            for x in start_x..end_x {
                let idx = y * window_width + x;
                self.pixels[idx as usize] = color;
            }
        }
    }

    pub fn set(&mut self, x: u32, y: u32, color: Color) {
        assert!(
            x < self.width && y < self.height,
            "Invalid coordinates ({x}, {y}) for window with size {} x {}",
            self.width,
            self.height
        );

        let idx = y * self.width + x;
        self.pixels[idx as usize] = u32::from(color);
    }

    #[inline(always)]
    pub fn get_height(&self) -> u32 {
        self.height
    }

    #[inline(always)]
    pub fn get_width(&self) -> u32 {
        self.width
    }

}

pub trait UIEventLoopHandler {
    fn dispatch(&mut self, screen: Screen, event: UIEvent);
}

pub struct MiniUI {
    client: Option<WaylandClient<Box<MiniUI>>>,
    buffer: SharedBuffer,
    handler: Box<dyn UIEventLoopHandler>,

    current_width  : i32,
    current_height : i32,
}

const MAX_WIDTH: i32 = 1920;
const MAX_HEIGHT: i32 = 1080;
const MAX_STRIDE: i32 = calc_stride(MAX_WIDTH);
const MAX_WINDOW_SIZE: i32 = MAX_STRIDE * MAX_HEIGHT;

#[inline(always)]
const fn calc_stride(width: i32) -> i32 {
    width * 4
}

impl MiniUI {
    pub fn build<T>(app_title: &str, app_id: &str, handler: T) -> Result<Self>
    where
        T: UIEventLoopHandler + 'static,
    {
        wlclient::init_log();
        let mut client = wlclient::connect()?;
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
            top_level.set_title(app_title)?;

            (surface, xdg_surface, top_level)
        };
        surface.commit()?; // initial empty commit
        let (pool, mut buffer) = client.create_pool(MAX_WINDOW_SIZE)?;
        buffer.fill(255); // fills the screen with white

        client.upgrade_to_global(&pool)?;

        // by now
        client.upgrade_to_global(&surface)?;
        client.upgrade_to_global(&xdg_surface)?;
        client.upgrade_to_global(&top_level)?;

        Ok(Self {
            buffer,
            client: Some(client),
            handler: Box::new(handler),
            current_height: 0,
            current_width: 0
        })
    }

    pub fn event_loop(mut self) -> Result<()> {
        let mut client = self.client.take().unwrap();

        let top_level : XdgTopLevel = client.get_global().unwrap();
        let wm_surface : XdgSurface = client.get_global().unwrap();
        let surface : WlSurface = client.get_global().unwrap();

        client.add_event_handler(&top_level, |client, msg| match msg.event {
            XdgTopLevelEvent::Close => {
                let ui = client.get_custom_state().unwrap();
                ui.dispatch(UIEvent::Exit);
                log::info!("Closing window");
                process::exit(0);
            }

            XdgTopLevelEvent::Configure { width, height, .. } => {

                let ui = client.get_custom_state().unwrap();
                if ui.current_width != width || ui.current_height != height  {
                    let Err(error) = Self::update(client, width, height) else {
                        return;
                    };
                    log::error!("Updating application state: {error:?}")
                }
            }
            _ => (),
        })?;

        // NOTE: Just to silence warnings, you want to do something else later
        client.add_event_handler(&surface, |_,_| {})?;
        client.add_event_handler(&wm_surface, |client, msg| {
            let XdgSurfaceEvent::Configure { serial_nr } = msg.event;
            let xdg_surface: XdgSurface = client.get_reference(msg.object_id).unwrap();
            xdg_surface.ack_configure(serial_nr).unwrap();

            let ui = client.get_custom_state().unwrap();
            if ui.current_width == 0 || ui.current_height == 0 {
                Self::update(client, MAX_WIDTH, MAX_HEIGHT).unwrap();
            }
        })?;

        client.set_custom_state(Box::new(self));
        client.event_loop();
        Ok(())
    }

    fn dispatch(&mut self, event : UIEvent) {
        assert!(self.current_height > 0 && self.current_width > 0);

        let screen = { 

            let limit = self.current_width as usize * self.current_height as usize * mem::size_of::<u32>();
            let buffer = &mut self.buffer.as_mut()[0..limit];

            // turn &[u8] iinto &[u32]
            let (pref, pixels, suf) = unsafe { buffer.align_to_mut::<u32>() };
            assert!(pref.is_empty() && suf.is_empty());

            Screen {
                width: self.current_width as u32,
                height: self.current_height as u32,
                pixels
            }
        };

        self.handler.dispatch(screen, event);
    }

    fn update(client : &mut WaylandClient<Box<Self>>, new_width: i32, new_height: i32) -> Result<()> {
        assert!(new_height > 0 && new_width > 0);

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
            // TODO: when you start support frames and everything please reuse buffers
            buffer.destroy(); 
        });

        let ui = client.get_custom_state().unwrap();
        let event = if ui.current_width == 0 && ui.current_height == 0 {
            UIEvent::Initialize
        } else {
            UIEvent::Resize
        };

        ui.current_width  = new_width;
        ui.current_height = new_height;
        ui.dispatch(event);

        let surface : WlSurface = client.get_global().unwrap();
        surface.damage_buffer(0, 0, i32::MAX, i32::MAX)?;
        surface.attach(&buffer, 0, 0)?;
        surface.commit()?;
        Ok(())
    }
}
