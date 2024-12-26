#![allow(unused)]
use wlclient::{
    client::{memory::SharedBuffer, WaylandClient},
    error::Result,
    protocol::{base::*, xdg_shell::*, WlEventMsg},
};

use core::mem;
use std::{convert::Into, process};

struct Window {
    // surface info
    surface: WlSurface,
    wm_surface: XdgSurface,
    top_level: XdgTopLevel,
    //buffer: Option<WlBuffer>,

    canvas: WindowBuffer,
}

const MAX_WIDTH: i32 = 1920;
const MAX_HEIGHT: i32 = 1080;
const MAX_STRIDE: i32 = calc_stride(MAX_WIDTH);
const MAX_WINDOW_SIZE: i32 = MAX_STRIDE * MAX_HEIGHT;

#[inline(always)]
const fn calc_stride(width: i32) -> i32 {
    width * 4
}

#[derive(Clone, Copy)]
enum Color {
    Red,
    Green,
    Blue,
    White,
    Black,
    // TODO: add others
    Rgb(u8, u8, u8),
    Argb(u8, u8, u8, u8),
}

impl From<Color> for u32 {
    fn from(value: Color) -> Self {
        let (a, r, g, b) = match value {
            Color::Red => (0, 255, 0, 0),
            Color::Green => (0, 0, 255, 0),
            Color::Blue => (0, 0, 0, 255),
            Color::White => (0, 255, 255, 255),
            Color::Black => (0, 0, 0, 0),
            Color::Rgb(r, g, b) => (0, r as u32, g as u32, b as u32),
            Color::Argb(a, r, g, b) => (a as u32, r as u32, g as u32, b as u32),
        };
        a << 24 | r << 16 | g << 8 | b
    }
}

struct WindowBuffer {
    width: u32,
    height: u32,
    buffer: SharedBuffer,
}

impl WindowBuffer {
    fn new(width: i32, height: i32, buffer: SharedBuffer) -> Self {
        let mut buffer = Self {
            width: 0,
            height: 0,
            buffer,
        };
        buffer.resize(width, height);
        buffer
    }

    fn resize(&mut self, width: i32, height: i32) {
        assert!(width >= 0 && height >= 0);
        assert!(
            self.buffer.len() >= width as usize * height as usize * mem::size_of::<u32>(),
            "Underline buffer overflow ( allocated-size: {}, new-size: {width} x {height} )",
            self.buffer.len()
        );

        self.width = width as u32;
        self.height = height as u32;
    }

    fn fill(&mut self, color: Color) {
        let color = u32::from(color);
        let buffer = self.as_mut();
        buffer.fill(color);
    }

    fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: Color) {
        let window_height = self.height;
        let window_width = self.width;

        let start_x = u32::min(window_width, x);
        let end_x = u32::min(window_width, x + width);

        let start_y = u32::min(window_height, y);
        let end_y = u32::min(window_height, y + height);

        let buffer = self.as_mut();
        let color = u32::from(color);

        for y in start_y..end_y {
            for x in start_x..end_x {
                let idx = y * window_width + x;
                buffer[idx as usize] = color;
            }
        }
    }

    fn set(&mut self, x: u32, y: u32, color: Color) {
        assert!(
            x < self.width && y < self.height,
            "Invalid coordinates ({x}, {y}) for window with size {} x {}",
            self.width,
            self.height
        );

        let idx = y * self.width + x;
        self.as_mut()[idx as usize] = u32::from(color);
    }

    fn get_height(&self) -> u32 {
        self.height
    }
    fn get_width(&self) -> u32 {
        self.width
    }

    fn as_mut(&mut self) -> &mut [u32] {
        let limit = self.width as usize * self.height as usize * mem::size_of::<u32>();
        let buffer = &mut self.buffer.as_mut()[0..limit];

        let (pref, buffer, suf) = unsafe { buffer.align_to_mut::<u32>() };
        assert!(pref.is_empty() && suf.is_empty());

        buffer
    }
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

    let (pool, shared_buffer) = client.create_pool(MAX_WINDOW_SIZE)?;
    client.upgrade_to_global(&pool)?;

    let mut canvas = WindowBuffer::new(0, 0, shared_buffer);
    canvas.fill(Color::White);

    Ok(Window {
        surface,
        top_level,
        wm_surface: xdg_surface,
        canvas,
    })
}

#[allow(clippy::too_many_arguments)]
fn render_char(
    canvas: &mut WindowBuffer,
    letter: char,
    line_width : u32,
    start_x: u32,
    start_y: u32,
    square_width: u32,
    square_height: u32,
    color: Color,
) {
    match letter.to_ascii_lowercase() {
        'h' => {
            canvas.fill_rect(start_x, start_y, line_width, square_height, color);
            let middle_y = start_y +  square_height / 2 - line_width / 2;
            canvas.fill_rect(start_x, middle_y, square_width, line_width,  color);
            canvas.fill_rect(start_x + square_width - line_width, start_y, line_width, square_height, color);
        }

        'e' => {
            canvas.fill_rect(start_x, start_y, square_width, line_width, color);
            let middle_y = start_y +  square_height / 2 - line_width / 2;
            canvas.fill_rect(start_x, middle_y, square_width - line_width, line_width,  color);
            canvas.fill_rect(start_x, start_y, line_width, square_height, color);
            canvas.fill_rect(start_x, start_y + square_height - line_width, square_width, line_width, color);
        }

        'l' => {
            canvas.fill_rect(start_x + line_width / 2, start_y, line_width, square_height, color);
            canvas.fill_rect(start_x + line_width / 2, start_y + square_height - line_width, square_width - line_width / 2, line_width, color);
        }

        'o' => {
            canvas.fill_rect(start_x, start_y, line_width, square_height, color);
            canvas.fill_rect(start_x, start_y, square_width, line_width, color);
            canvas.fill_rect(start_x + square_width - line_width, start_y, line_width, square_height, color);
            canvas.fill_rect(start_x, start_y + square_height - line_width, square_width, line_width, color);
        }

        'w' => {
            canvas.fill_rect(start_x, start_y, line_width, square_height, color);
            let middle_x = start_x + square_width / 2 - line_width / 2;
            canvas.fill_rect(middle_x, start_y + 2 * line_width, line_width, square_height - 2 * line_width, color);
            canvas.fill_rect(start_x, start_y + square_height - line_width, square_width, line_width, color);
            canvas.fill_rect(start_x + square_width - line_width, start_y, line_width, square_height, color);
        }

        'r' => {
            canvas.fill_rect(start_x, start_y, line_width, square_height, color);
            canvas.fill_rect(start_x, start_y, square_width, line_width, color);
            canvas.fill_rect(start_x + square_width - line_width, start_y, line_width, square_height, color);
        }

        'd' => {
            canvas.fill_rect(start_x + line_width, start_y, line_width, square_height, color);
            canvas.fill_rect(start_x, start_y, square_width, line_width, color);
            canvas.fill_rect(start_x + square_width - line_width, start_y, line_width, square_height, color);
            canvas.fill_rect(start_x, start_y + square_height - line_width, square_width, line_width, color);
        }

        '!' => {
            canvas.fill_rect(start_x, start_y, line_width, square_height - 2 * line_width, color);
            canvas.fill_rect(start_x, start_y + square_height - line_width, line_width, line_width, color);
        }
        ',' => {
            let head_size = line_width + line_width / 2;
            canvas.fill_rect(start_x, start_y + square_height - head_size, head_size, head_size, color);
            canvas.fill_rect(start_x + line_width / 2, start_y + square_height, line_width,  line_width, color);
        }
        ' ' => (),

        letter => {
            unimplemented!("Render for letter: '{letter}'")
        }
    }
}

fn update(client: &mut WaylandClient<'_, Window>, new_width: i32, new_height: i32) -> Result<()> {
    //let window = client.get_custom_state().unwrap();

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
    });

    let window = client.get_custom_state().unwrap();
    {
        let canvas = &mut window.canvas;
        canvas.resize(new_width, new_height);
        canvas.fill(Color::White);

        let square_height = 90;
        let square_width  = (2 * square_height) / 3;

        let letter_gap = square_width / 8;
        let start_y = if canvas.get_height() > square_height {
            canvas.get_height() / 2 - square_height / 2
        } else {
            0
        };

        let line_width = 2 * letter_gap;

        let text = "hello, world!";
        let estimated_width = text.chars().fold(letter_gap, |acc, letter| {
            acc + match letter {
                '!' => line_width + letter_gap,
                letter if letter.is_whitespace() => 0,
                _ => letter_gap + square_width
            }
        });

        let mut start_x = if estimated_width < canvas.get_width() {
            canvas.get_width() / 2 - estimated_width / 2
        } else {
            letter_gap
        };

        for (i, letter) in text.chars().enumerate() {
            render_char(canvas, letter, line_width, start_x, start_y, square_width, square_height, Color::Black);

            if !letter.is_whitespace() {
                start_x += letter_gap + square_width;
            }
        }
    }

    let surface = &window.surface;
    surface.damage_buffer(0, 0, i32::MAX, i32::MAX)?;
    surface.attach(&buffer, 0, 0)?;
    surface.commit()?;

    window.canvas.resize(new_width, new_height);
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
            let buffer = &window.canvas;

            if buffer.get_width() != width as u32 || buffer.get_height() != height as u32 {
                let Err(error) = update(client, width, height) else {
                    return;
                };
                log::error!("Updating application state: {error:?}")
            }
        }
        _ => (),
    })?;

    client.add_event_handler(&window.wm_surface, |client, msg| {
        let XdgSurfaceEvent::Configure { serial_nr } = msg.event;
        let xdg_surface: XdgSurface = client.get_reference(msg.object_id).unwrap();
        xdg_surface.ack_configure(serial_nr).unwrap();

        let window = client.get_custom_state().unwrap();
        let buffer = &window.canvas;
        if buffer.get_width() == 0 && buffer.get_height() == 0 {
            update(client, MAX_WIDTH, MAX_HEIGHT).unwrap();
        }
    });

    client.set_custom_state(window);
    log::info!("State initialization completed...");

    client.event_loop();
    Ok(())
}
