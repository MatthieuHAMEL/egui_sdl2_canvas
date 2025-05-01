use std::time::{Duration, Instant};
use std::collections::HashMap;

use egui::epaint::ImageDelta;
use egui::RawInput;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::Texture;
use sdl2::{event::Event, image::{self, Sdl2ImageContext}, keyboard::Keycode, mixer::{self, Sdl2MixerContext, AUDIO_S16LSB, DEFAULT_CHANNELS}, pixels::Color, render::{Canvas, TextureCreator}, ttf::Sdl2TtfContext, video::{Window, WindowContext}, IntegerOrSdlError, Sdl, VideoSubsystem};
use winapi::{shared::windef::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, um::winuser::SetProcessDpiAwarenessContext};

// R, G, B, A is passed in order to the SDL, hence the format :
#[cfg(target_endian = "little")]
const SDL_EGUI_FORMAT: PixelFormatEnum = PixelFormatEnum::ABGR8888; // bytes = RGBA
#[cfg(target_endian = "big")]
const SDL_EGUI_FORMAT: PixelFormatEnum = PixelFormatEnum::RGBA8888; // bytes = RGBA

fn init_sdl2(
  win_title: &str,
  win_width: u32,
  win_height: u32,
) -> (Sdl,
   Sdl2ImageContext,
   Sdl2TtfContext,
   VideoSubsystem,
   Sdl2MixerContext,
   Canvas<Window>) 
{
//let mut b = sdl2::hint::set_with_priority("SDL_HINT_VIDEO_HIGHDPI_DISABLED", "1", &sdl2::hint::Hint::Override);

// For some reason the hint below was not enough and I had to do that
unsafe {  // TODO this is only on windows ...
  SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
}

let mut _b = sdl2::hint::set_with_priority(
  "SDL_HINT_WINDOWS_DPI_AWARENESS ",
  "permonitorv2",
  &sdl2::hint::Hint::Override,
);

// I'm handling DPI scaling by myself ... For now !
_b = sdl2::hint::set_with_priority(
  "SDL_HINT_WINDOWS_DPI_SCALING",
  "0",
  &sdl2::hint::Hint::Override,
);

let sdl_context = sdl2::init().unwrap();

// sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "1"); // for pixel linear interpolation. TODO needed ?
let image_context = sdl2::image::init(image::InitFlag::PNG).unwrap();

let video_subsystem = sdl_context.video().unwrap();

let ttf_context = sdl2::ttf::init().unwrap();

// TODO flag "init music ... "
let mixer_subsystem = mixer::init(mixer::InitFlag::MP3 | mixer::InitFlag::OGG).unwrap();

sdl2::mixer::open_audio(44100, AUDIO_S16LSB, DEFAULT_CHANNELS, 1024).unwrap();

sdl2::mixer::allocate_channels(16);

// Window creation
let mut windowb = video_subsystem.window(win_title, win_width, win_height);
println!("windowb flags !!!!! 3- {}", windowb.window_flags()); // TODO simplify
windowb.allow_highdpi().position_centered();

let window = windowb.build().unwrap();

// The main object to render textures on (<=> SDL_CreateRenderer)
let canvas: Canvas<Window> = window
  .into_canvas()
  // .present_vsync()
  .build() // vsync : (TODO : VSYNC support vs no vsync support)
  .map_err(|e| match e {
    IntegerOrSdlError::IntegerOverflows(msg, val) => {
      format!("int overflow {}, val: {}", msg, val)
    }
    IntegerOrSdlError::SdlError(msg) => {
      format!("SDL error: {}", msg)
    }
  })
  .unwrap();

(sdl_context, image_context, ttf_context, video_subsystem, mixer_subsystem, canvas)
// no need to return the window, it is held by the canvas
}

pub struct MySdl2 {
  sdl_context: Sdl,
  _image_context: Sdl2ImageContext,
  ttf_context: Sdl2TtfContext,
  _video_subsystem: VideoSubsystem,
  _mixer_context: Sdl2MixerContext,
  canvas: Canvas<Window>,
  texture_creator: TextureCreator<WindowContext>,
  window_dim: (u32, u32),
}

impl MySdl2 {
  pub fn new(title: &str, win_width: u32, win_heigt: u32) -> Self {
    let (sdl_context, _image_context, ttf_context, _video_subsystem, _mixer_context, canvas) 
    = init_sdl2(title, win_width, win_heigt);

    let texture_creator = canvas.texture_creator();
    Self {
      sdl_context, _image_context, ttf_context, _video_subsystem, _mixer_context, canvas, 
      window_dim: (win_width, win_heigt), texture_creator
    }
  }
}

pub fn update_egui_texture(id: egui::TextureId, delta: &ImageDelta, textures: &mut HashMap<egui::TextureId, Texture>,
  tc: &TextureCreator<WindowContext>) -> Result<(), String> 
{
  // 1. Flatten 
  let (mut bytes, w, h) = match &delta.image {
    egui::ImageData::Color(img) => {
      let mut buf = Vec::with_capacity(img.pixels.len() * 4);
      buf.extend(img.pixels.iter().flat_map(|&c| c.to_array()));
      (buf, img.width() as u32, img.height() as u32)
    }
    egui::ImageData::Font(img) => {
      let mut buf = Vec::with_capacity(img.width() * img.height() * 4); // Todo: use pixels.len() and factorize 
      buf.extend(img.srgba_pixels(None).flat_map(|c| c.to_array()));
      (buf, img.width() as u32, img.height() as u32)
    }
  };

  let pitch = (w * 4) as usize;

  // 2. create / resize the SDL texture
  let tex = textures.entry(id).or_insert_with(|| {
    tc.create_texture_streaming(SDL_EGUI_FORMAT, w, h) // ABGR8888 on Little-Endian               
      .expect("failed to create atlas texture")
  });

  // If size changed, recreate the texture 
  let q = tex.query();
  if q.width != w || q.height != h {
      *tex = tc.create_texture_streaming(PixelFormatEnum::RGBA32, w, h).unwrap();
  }

  // Patch upload (or full upload)
  if let Some([x, y]) = delta.pos {
      let rect = Rect::new(x as i32, y as i32, w, h);
      tex.update(rect, &bytes, pitch);
  } else {
      tex.update(None, &bytes, pitch);
  }
  Ok(())
}

fn main() {
  // 1. Init SDL2 
  let screen_size = (800, 500); // w, h
  let mut mysdl2 = MySdl2::new("my app", screen_size.0, screen_size.1);

  let mut egui_tex_map: HashMap<egui::TextureId, Texture> = HashMap::new();

  let mut event_pump = mysdl2.sdl_context.event_pump().unwrap();

  let target_frame_duration = Duration::from_secs_f32(1.0 / 60.0); // Targeting 60 FPS
  let mut last_update = Instant::now(); 

  let ctx = egui::Context::default();
  let mut color = [0.0, 0.0, 0.0, 1.0];
  let mut text = String::new();


  'myloop: loop {
    // Handle events 
    for event in event_pump.poll_iter() {
      match event {
        Event::Quit {..} => { break 'myloop }, 
        _ => { continue; /* Nothing for now */ }
      }
    }

    // Update logic 
    let now = Instant::now();
   // let delta_time = now.duration_since(last_update).as_secs_f32();
    //last_update = now;

    let raw_input = RawInput {
      screen_rect: Some(egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::Vec2 {
          x: screen_size.0 as f32,
          y: screen_size.1 as f32,
        },
      )),
      ..Default::default()
    };
    ctx.begin_pass(raw_input);

    egui::Window::new("Hello, world!").show(&ctx, |ui| {
      ui.label("Hello, world!");
      if ui.button("Greet").clicked() {
      println!("Hello, world!");
      }
      ui.horizontal(|ui| {
      ui.label("Color: ");
      ui.color_edit_button_rgba_premultiplied(&mut color);
      });
      ui.code_editor(&mut text);
    });

    let output = ctx.end_pass();
    let paint_job = ctx.tessellate(output.shapes, ctx.pixels_per_point());

    // Now I try to convert that vector of ClippedPrimitive into something I can render
    for (id, delta) in output.textures_delta.set {
      update_egui_texture(id, &delta, &mut egui_tex_map, &mysdl2.texture_creator)?;
    }

    // Render (draw, update screen)
    mysdl2.canvas.set_draw_color(Color::RGB(0, 0, 0));
    mysdl2.canvas.clear();
    mysdl2.canvas.present();

    // Maintain a consistent frame rate
    let frame_duration = now.elapsed();
    if frame_duration < target_frame_duration { // TODO not needed with VSYNC ?
      std::thread::sleep(target_frame_duration - frame_duration);
    } // else application is quite overwhelmed! ... 
  }


}

