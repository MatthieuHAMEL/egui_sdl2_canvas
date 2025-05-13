use std::collections::HashMap;
use sdl2::render::Texture;

struct Painter<'a> {
    texture_map: HashMap<egui::TextureId, Texture<'a>>
}

pub fn run() {
    println!("Hello, world!");
}
