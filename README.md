# egui_sdl2_canvas

This is [egui](https://github.com/emilk/egui) over the [canvas](https://docs.rs/sdl2/latest/sdl2/render/struct.Canvas.html) object from the sdl2 crate (better known as [SDL_Renderer](https://wiki.libsdl.org/SDL2/CategoryRender) in C).

Work in progress, feedback / proposals appreciated! 

The examples folder shows two little applications, using my painter, and [egui_sdl2_platform](https://github.com/GetAGripGal/egui_sdl2_platform) for all the event handling. (egui_sdl2_platform is currently limited to egui 0.27, but I submitted a PR)

![it works](screenshots/it_works.png)

![more complex ui](screenshots/more_complex_ui.png)
