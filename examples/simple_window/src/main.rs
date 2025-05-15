use std::time::{Duration, Instant};
use egui_sdl2_renderer::Painter;
use sdl2::{event::Event, image::{self, Sdl2ImageContext}, mixer::{self, Sdl2MixerContext, AUDIO_S16LSB, DEFAULT_CHANNELS}, pixels::Color, render::{Canvas, TextureCreator}, ttf::Sdl2TtfContext, video::{Window, WindowContext}, IntegerOrSdlError, Sdl, VideoSubsystem};
use winapi::{shared::windef::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, um::winuser::SetProcessDpiAwarenessContext};

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
windowb.allow_highdpi().position_centered();

let window = windowb.build().unwrap();

// The main object to render textures on (<=> SDL_CreateRenderer)
let canvas: Canvas<Window> = window
  .into_canvas()
  .build()
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
  _ttf_context: Sdl2TtfContext,
  _video_subsystem: VideoSubsystem,
  _mixer_context: Sdl2MixerContext,
  canvas: Canvas<Window>,
  texture_creator: TextureCreator<WindowContext>,
  window_dim: (u32, u32),
}

impl MySdl2 {
  pub fn new(title: &str, win_width: u32, win_heigt: u32) -> Self {
    let (sdl_context, _image_context, _ttf_context, _video_subsystem, _mixer_context, canvas) 
    = init_sdl2(title, win_width, win_heigt);

    let texture_creator = canvas.texture_creator();
    Self {
      sdl_context, _image_context, _ttf_context, _video_subsystem, _mixer_context, canvas, 
      window_dim: (win_width, win_heigt), texture_creator
    }
  }
}

fn main() {
  // 1. Init SDL2 
  let screen_size = (800, 500); // w, h
  let mut mysdl2 = MySdl2::new("my app", screen_size.0, screen_size.1);
  let mut platform = egui_sdl2_platform::Platform::new(mysdl2.window_dim).unwrap();
  let mut painter = Painter::new();
  let mut event_pump = mysdl2.sdl_context.event_pump().unwrap();
  let target_frame_duration = Duration::from_secs_f32(1.0 / 60.0); // Targeting 60 FPS

  // Used in the egui window :
  let mut color = [0.0, 0.0, 0.0, 1.0];
  let mut text = String::new();

  let start_time = Instant::now();
  'myloop: loop {
    platform.update_time(start_time.elapsed().as_secs_f64());
    let now = Instant::now();

    // Handle events 
    for event in event_pump.poll_iter() {
      match event {
        Event::Quit {..} => { break 'myloop }, 
        _ => { /* Nothing for now */ } // TODO test when window resizes
      }
      platform.handle_event(&event, &mysdl2.sdl_context, &mysdl2._video_subsystem);
    }
    let ctx = platform.context();
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

    let output = platform.end_frame(&mut mysdl2._video_subsystem).unwrap();
    let v_primitives = platform.tessellate(&output);

    // Convert textures_delta (image data) to SDL2 textures, and draw
    mysdl2.canvas.set_draw_color(Color::RGB(0, 0, 0));
    mysdl2.canvas.clear();

    painter.paint_and_update_textures(
      ctx.pixels_per_point(),
      &output.textures_delta, &mysdl2.texture_creator, 
      &v_primitives, &mut mysdl2.canvas);
    
    // Render
    mysdl2.canvas.present();

    // Maintain a consistent frame rate
    let frame_duration = now.elapsed();
    if frame_duration < target_frame_duration {
      std::thread::sleep(target_frame_duration - frame_duration);
    } 
  } // Main loop 
}
