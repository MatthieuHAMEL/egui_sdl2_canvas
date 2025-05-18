use std::time::{Duration, Instant};
use egui::{Color32, Context};
use egui_plot::{Line, Plot, PlotPoints};
use egui_sdl2_canvas::Painter;
use sdl2::{event::Event, pixels::Color, render::{Canvas, TextureCreator}, video::{Window, WindowContext}, IntegerOrSdlError, Sdl, VideoSubsystem};

#[cfg(target_os = "windows")]
use winapi::{shared::windef::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, um::winuser::SetProcessDpiAwarenessContext};

// Code using EGUI
pub struct HelloworldApp {
// Used in the egui window :
  color: [f32;4],
  text: String
}

impl HelloworldApp {
  pub fn new() -> Self {
    Self { color: [0.0, 0.0, 0.0, 1.0], text: String::new() }
  }

  fn show(&mut self, ctx: &Context) {
    egui::Window::new("Hello, world!").show(&ctx, |ui| {
      ui.label("Hello, world!");
      if ui.button("Greet").clicked() {
        println!("Hello, world!");
      }
      ui.horizontal(|ui| {
        ui.label("Color: ");
        ui.color_edit_button_rgba_premultiplied(&mut self.color);
      });
      ui.code_editor(&mut self.text);
    });
  }
}

struct MoreComplexApp {
  freq: f64,
  running: bool,
  another_thing: bool,
  zoom: f32,
  tint: Color32,
  t0: Instant,
}

impl MoreComplexApp {
  pub fn new() -> Self {
    Self {
      freq: 0.0,
      running: false,
      another_thing: false,
      zoom: 0.0,
      tint: Color32::default(),
      t0: Instant::now(),
    }
  }
  pub fn show(&mut self, ctx: &Context) {
    // Left panel
    egui::SidePanel::left("controls").show(ctx, |ui| {
      ui.heading("Controls");
      ui.separator();

      ui.checkbox(&mut self.running, "Animate sine");
      ui.add(egui::Slider::new(&mut self.freq, 0.1..=10.0).text("frequency"));

      ui.separator();
      ui.checkbox(&mut self.another_thing, "Another checkbox");
      ui.add(egui::Slider::new(&mut self.zoom, 0.1..=4.0).text("zoom"));
      ui.color_edit_button_srgba(&mut self.tint);
    });

    // Central area
    egui::CentralPanel::default().show(ctx, |ui| {
      ui.set_min_width(600.0);

      //  animated sine plot 
      let dt = self.t0.elapsed().as_secs_f64();
      let phase = if self.running { dt } else { 0.0 };
      let points: PlotPoints = (0..1000)
        .map(|i| {
          let x = i as f64 * 0.01;
          [x, (phase + x * self.freq).sin()]
        }).collect();

      Plot::new("sine")
        .height(150.0)
        .include_y(-1.2)
        .include_y(1.2)
        .show(ui, |plot_ui| {
            plot_ui.line(Line::new(points).color(egui::Color32::LIGHT_GREEN));
        });

      ui.separator();

      // scrollable log
      egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("log").striped(true).show(ui, |ui| {
          for i in 0..200 {
            ui.label(format!("Row {i}"));
            ui.label(format!("Value {}", i * 42));
            ui.end_row();
          }
        });
      });
    });
  }
}

// Code initializing the SDL 
fn init_sdl2(win_title: &str, win_width: u32, win_height: u32) 
  -> (Sdl, VideoSubsystem, Canvas<Window>) 
{
  // For some reason the hint below was not enough and I had to do that
  #[cfg(target_os = "windows")]
  unsafe {
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

  // sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "1"); // for pixel linear interpolation. Needed ? 
  let video_subsystem = sdl_context.video().unwrap();

  // Window creation
  let mut windowb = video_subsystem.window(win_title, win_width, win_height);
  windowb.allow_highdpi().position_centered().resizable();
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

  (sdl_context, video_subsystem, canvas)
  // no need to return the window, it is held by the canvas
}

pub struct MySdl2 {
  sdl_context: Sdl,
  _video_subsystem: VideoSubsystem,
  canvas: Canvas<Window>,
  texture_creator: TextureCreator<WindowContext>,
  window_dim: (u32, u32),
}

impl MySdl2 {
  pub fn new(title: &str, win_width: u32, win_heigt: u32) -> Self {
    let (sdl_context, _video_subsystem, canvas) 
      = init_sdl2(title, win_width, win_heigt);

    let texture_creator = canvas.texture_creator();
    Self {
      sdl_context, _video_subsystem, canvas, 
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

  let mut my_app = MoreComplexApp::new(); // try : HelloworldApp::new();

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

    my_app.show(&ctx);

    let output = platform.end_frame(&mut mysdl2._video_subsystem).unwrap();
    let v_primitives = platform.tessellate(&output);

    // Convert textures_delta (image data) to SDL2 textures, and draw
    mysdl2.canvas.set_draw_color(Color::RGB(0, 0, 0));
    mysdl2.canvas.clear();

    if let Err(err) = painter.paint_and_update_textures(
      ctx.pixels_per_point(),
      &output.textures_delta, &mysdl2.texture_creator, 
      &v_primitives, &mut mysdl2.canvas) {
        println!("{}", err);
    }
    
    // Render
    mysdl2.canvas.present();

    // Maintain a consistent frame rate
    let frame_duration = now.elapsed();
    if frame_duration < target_frame_duration {
      std::thread::sleep(target_frame_duration - frame_duration);
    } 
  } // Main loop 
}
