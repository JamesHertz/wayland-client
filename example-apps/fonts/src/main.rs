#![allow(unused)]
use mini_ui::{ 
    Color, MiniUI, Result, Screen, UIEvent, UIEventLoopHandler
};

mod math;
mod parser;

use math::{Vec2, lerp};
use parser::TrueTypeFont;

struct Blank;
impl Blank {
    fn draw(screen : &mut Screen, Vec2(x,y) : Vec2, size : u32, color : Color) {
        let x = (x * screen.get_width() as f32) as u32;
        let y = (y * screen.get_height() as f32) as u32;
        screen.fill_rect(x - size / 2, y - size / 2, size, size, color);
    }

    fn draw_curve(screen : &mut Screen, from : Vec2, to : Vec2, curvature : Vec2, size : u32, color : Color){
        for i in (0..=100) {
            let s = 0.01 * i as f32;
            let v1 = lerp(from, curvature, s);
            let v2 = lerp(curvature, to, s);
            let target = lerp(v1, v2, s);
            Self::draw(screen, target, size, color);
        }
    }
}

impl UIEventLoopHandler for Blank {
    fn dispatch(&mut self, mut screen: Screen, event: UIEvent) {
        match event {
            UIEvent::Initialize | UIEvent::Resize => {
                screen.fill(Color::White);

                let a = Vec2(0.1, 0.1);
                let b = Vec2(0.8, 0.1);
                let c = Vec2(0.5, 0.8);

                Self::draw_curve(&mut screen, a, b, c, 5, Color::Blue);
                Self::draw_curve(&mut screen, a, b, c + Vec2(0.0, 0.5), 5, Color::Blue);
            }
            _ => ()
        }
    }
}

fn main() -> Result<()> {
    let mut font = TrueTypeFont::from_file("example.ttf")?;
    font.iter_table_entries().for_each(|entry| println!("{entry:?}"));
    font.load_glyphs();

    //let ui = MiniUI::build("Font Redering App", "font-app", Blank)?;
    //ui.event_loop()?;
    Ok(())
}
