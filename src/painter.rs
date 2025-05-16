use std::collections::HashMap;
use egui::epaint::{ImageDelta, Primitive};
use egui::{ClippedPrimitive, TexturesDelta};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::pixels::PixelFormatEnum;
use sdl2::video::{Window, WindowContext};
use sdl2_sys::{SDL_RenderGeometry, SDL_Renderer, SDL_Texture};
use sdl2::sys::{SDL_Vertex, SDL_FPoint, SDL_Color};

use std::os::raw::c_int;

// R, G, B, A is passed in order to the SDL, hence the format :
#[cfg(target_endian = "little")]
const SDL_EGUI_FORMAT: PixelFormatEnum = PixelFormatEnum::ABGR8888; // bytes = RGBA
#[cfg(target_endian = "big")]
const SDL_EGUI_FORMAT: PixelFormatEnum = PixelFormatEnum::RGBA8888; // bytes = RGBA

#[inline]
fn egui_vertex_to_sdl(v: &egui::epaint::Vertex, ppp: f32) -> SDL_Vertex {
  let [r, g, b, a] = v.color.to_array();
  SDL_Vertex {
      position: SDL_FPoint { x: v.pos.x * ppp, y: v.pos.y * ppp },
      color: SDL_Color   { r, g, b, a },
      tex_coord: SDL_FPoint { x: v.uv.x, y: v.uv.y },
  }
}

pub fn update_egui_texture<'a>(id: &egui::TextureId, delta: &ImageDelta, 
  textures: &mut HashMap<egui::TextureId, Texture<'a>>,
  tc: &'a TextureCreator<WindowContext>) -> Result<(), String> 
{
  // 1. Flatten (TODO , some allocations may be avoided here, see :
  // https://github.com/emilk/egui/blob/81b7e7f05a6b03fa2cd5bdc6d4ce5f598e16c628/crates/egui_glow/src/painter.rs#L470)
  // Allocate buffer outside the match to ensure it lives long enough
  let mut _buf: Option<Vec<u8>> = None;

  let (bytes, w, h): (&[u8], u32, u32) = match &delta.image {
    egui::ImageData::Color(img) => {
      let bytes : &[u8] = bytemuck::cast_slice(img.pixels.as_ref());
      (bytes, img.width() as u32, img.height() as u32)
    }
    egui::ImageData::Font(img) => {
      // srgba_pixels(None) returns an iterator, so I still need a buffer
      _buf = Some(img.srgba_pixels(None).flat_map(|c| c.to_array()).collect());
      (_buf.as_ref().unwrap().as_slice(), img.width() as u32, img.height() as u32)
    }
  };

  let pitch = (w * 4) as usize;

  // 2. Create the SDL texture, if needed
  let tex = textures.entry(*id).or_insert_with(|| {
    let mut t = tc.create_texture_streaming(SDL_EGUI_FORMAT, w, h) // ABGR8888 on Little-Endian               
      .expect("Failed to create egui/sdl texture");
    t.set_blend_mode(BlendMode::Blend);
    t
  });

  // Patch upload (or full upload)
  if let Some([x, y]) = delta.pos {
      let rect = Rect::new(x as i32, y as i32, w, h);
      tex.update(rect, &bytes, pitch).unwrap();
  } else {
      tex.update(None, &bytes, pitch).unwrap();
  }
  Ok(())
}

pub struct Painter<'a> {
  texture_map: HashMap<egui::TextureId, Texture<'a>>,
}

impl<'a> Painter<'a> {
  pub fn new() -> Self {
    Self {
      texture_map: HashMap::new(),
    }
  }

  pub fn paint_and_update_textures(
    &mut self, 
    pixels_per_point: f32,
    textures_delta: &TexturesDelta, texture_creator: &'a TextureCreator<WindowContext>, 
    paint_jobs: &[ClippedPrimitive], canvas: &mut Canvas<Window>) -> Result<(), String> {
    for (id, delta) in &textures_delta.set {
      update_egui_texture(id, &delta, &mut self.texture_map, &texture_creator).unwrap();
    }

    // Now render every "ClippedPrimitive" from paint_jobs
    for ClippedPrimitive { clip_rect, primitive } in paint_jobs {
      // 1) Skip Primitive::PaintCallback (which is advanced stuff), focus on Mesh
      let Primitive::Mesh(mesh) = primitive 
        else {
          return Err("WARNING / TODO: PaintCallbacks are not supported, for now.".to_owned());
        };

      // 2) Get the sdl texture
      let texture_ptr = self.texture_map.get(&mesh.texture_id)
        .map(|t| t.raw() as *mut SDL_Texture)
        .unwrap_or(std::ptr::null_mut()); // egui may draw untextured shape (nullptr in SDL_RenderGeometry)

      // 3) Clip rect (egui units -> pixels)
      let clip = sdl2::rect::Rect::new(
        (clip_rect.min.x * pixels_per_point) as i32,
        (clip_rect.min.y * pixels_per_point) as i32,
        ((clip_rect.max.x - clip_rect.min.x) * pixels_per_point) as u32,
        ((clip_rect.max.y - clip_rect.min.y) * pixels_per_point) as u32,
      );
      canvas.set_clip_rect(clip);

      // 4) Convert egui vertices to SDL_Vertex (go unsafe, No vertex type in sdl2 crate)
      let sdl_vertices: Vec<SDL_Vertex> = mesh.vertices
        .iter()
        .map(|v| egui_vertex_to_sdl(v, pixels_per_point))
        .collect();
      let verts_len = sdl_vertices.len() as c_int;
      let verts_ptr = if verts_len == 0 {
        std::ptr::null()
      } else {
        sdl_vertices.as_ptr()
      };

      // 5) Indices: egui uses u32, SDL wants c_int
      let idxs_len = mesh.indices.len() as c_int;
      let idxs_ptr = if idxs_len == 0 {
        std::ptr::null() 
      } else { 
        mesh.indices.as_ptr() as *const c_int // array of u32 -> array of c_int (<=> i32)
      };

      // 6) Draw!
      let rv = unsafe {
        SDL_RenderGeometry(
          canvas.raw() as *mut SDL_Renderer,
          texture_ptr,
          verts_ptr, verts_len,
          idxs_ptr, idxs_len,
        )
      };
      if rv != 0 {
        return Err(format!("SDL_RenderGeometry failed with error {}", rv));
      }
    }
    canvas.set_clip_rect(None); 

    for &id in &textures_delta.free {
      self.texture_map.remove(&id);
    }

    Ok(())
  }
}
