use pixels::Pixels;
use std::rc::Rc;
use wfc_lib::{
    preprocessor::Pattern,
    simple_patterns::construct_simple_patterns,
    wfc::{Cell, Model},
    Wfc,
};
use wasm_bindgen::prelude::*;
// use winit::dpi::LogicalSize;
// use winit::event::{Event, VirtualKeyCode};
// use winit::event_loop::{ControlFlow, EventLoop};
// use winit::window::WindowBuilder;

// #[wasm_bindgen]
fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Warn).expect("error initializing logger");
    // run_simple_patterns();
    // wasm_bindgen_futures::spawn_local(run_celtic());
    // render_celtic();
    // render_celtic_patterns();
}

#[wasm_bindgen]
pub async fn run_wang_tile(bytes: &[u8]) {
    WfcWindow::new(glam::UVec2::splat(256), 2, 32).await.play(
        Wfc::new_from_image_bytes(&bytes)
            .with_tile_size(32)
            .with_output_dimensions(256, 256)
            .wang()
    );
}

#[allow(unused)]
#[wasm_bindgen]
pub async fn run_celtic() {
    let image = include_bytes!("../../inputs/celtic.png");
    WfcWindow::new(glam::UVec2::splat(256), 2, 32).await.play(
        Wfc::new_from_image_bytes(image)
            .with_tile_size(32)
            .with_output_dimensions(256, 256)
            .wang()
            .wang_flip(),
    );
}

#[allow(unused)]
#[wasm_bindgen]
pub async fn run_dual() {
    WfcWindow::new(glam::UVec2::splat(256), 2, 32).await.play(
        Wfc::new_from_image_path("./inputs/dual.png")
            .with_tile_size(32)
            .with_output_dimensions(256, 256)
            .wang(),
    );
}

#[allow(unused)]
async fn render_celtic_patterns() {
    let mut win = WfcWindow::new(glam::UVec2::splat(128), 4, 64).await;
    let image = image::io::Reader::open("./inputs/celtic.png")
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();
    let mut processor = wfc_lib::preprocessor::PreProcessor::new(
        &image,
        64,
        wfc_lib::preprocessor::ProcessorConfig::default(),
    );
    let data = processor.process();
    for (id, &loc) in processor.tiles.iter().enumerate() {
        win.render_cell(loc / 64, data.patterns[id].clone());
    }
    loop {
        win.render();
    }
}

#[allow(unused)]
async fn render_celtic() {
    let mut win = WfcWindow::new(glam::UVec2::splat(128), 4, 64).await;
    let image_bytes = include_bytes!("../../inputs/celtic.png");
    let image = Wfc::load_image_from_bytes(image_bytes);
    win.update(image.pixels());
    loop {
        win.render();
    }
}

#[allow(unused)]
#[wasm_bindgen]
pub async fn run_simple_patterns() {
    WfcWindow::new(glam::UVec2::splat(40), 12, 32).await.play(
        construct_simple_patterns()
            .with_tile_size(4)
            .with_output_dimensions(40, 40),
    )
}

use glam::UVec2;
use image::Rgba;
use wasm_bindgen::JsCast;
use winit::platform::web::WindowBuilderExtWebSys;
use winit::window::Window;

// const TILE_SIZE_DEFAULT: usize = 2;
// const PIXEL_SCALE_DEFAULT: u32 = 2;

pub struct WfcWindow {
    window: Rc<Window>,
    pixels: Pixels,
    tile_size: usize,
    window_dimensions: UVec2,
    event_loop: Option<winit::event_loop::EventLoop<()>>,
    wfc: Option<Wfc>,
}

impl WfcWindow {
    pub async fn new(window_dimensions: UVec2, pixel_size: u32, tile_size_var: usize) -> Self {
        let event_loop = winit::event_loop::EventLoop::new();
        let size = winit::dpi::LogicalSize::new(
            window_dimensions.x * pixel_size,
            window_dimensions.y * pixel_size,
        );
        let canvas = web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.get_element_by_id("wfc"))
            .and_then(|canvas| canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("couldn't find canvas element with id=\"wfc\"");
        let window = {
            let size = winit::dpi::LogicalSize::new(200 as f64, 200 as f64);
            winit::window::WindowBuilder::new()
                .with_title("Hello Pixels + Web")
                .with_inner_size(size)
                .with_min_inner_size(size)
                .with_canvas(Some(canvas))
                .build(&event_loop)
                .expect("WindowBuilder error")
        };

        let window = Rc::new(window);

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowExtWebSys;

            // Retrieve current width and height dimensions of browser client window
            let get_window_size = || {
                let client_window = web_sys::window().unwrap();
                winit::dpi::LogicalSize::new(
                    client_window.inner_width().unwrap().as_f64().unwrap(),
                    client_window.inner_height().unwrap().as_f64().unwrap(),
                )
            };

            let window = Rc::clone(&window);

            // Initialize winit window with current dimensions of browser client
            window.set_inner_size(get_window_size());

            let client_window = web_sys::window().unwrap();

            // Listen for resize event on browser client. Adjust winit window dimensions
            // on event trigger
            let closure =
                wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
                    let size = get_window_size();
                    window.set_inner_size(size)
                }) as Box<dyn FnMut(_)>);
            client_window
                .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }

        let size = window.inner_size();
        // let hidpi_factor = window.scale_factor();
        // let p_size = size.to_physical::<f64>(hidpi_factor);
        let surface_texture = pixels::SurfaceTexture::new(
            // p_size.width.round() as u32,
            // p_size.height.round() as u32,
            size.width,
            size.height,
            window.as_ref(),
        );

        let pixels =
            pixels::PixelsBuilder::new(window_dimensions.x, window_dimensions.y, surface_texture)
                .blend_state(pixels::wgpu::BlendState::REPLACE)
                .build_async()
                .await
                .unwrap();
        return Self {
            window,
            pixels,
            tile_size: tile_size_var,
            wfc: None,
            event_loop: Some(event_loop),
            window_dimensions,
        };
    }

    fn update_cell_in_frame_buffer(&mut self, cell: &Cell) {
        self.render_cell(
            cell.loc,
            cell.render(self.wfc.as_ref().unwrap().get_patterns(), self.tile_size),
        );
    }
    fn update_frame_buffer(&mut self, model: &mut Model) {
        while let Some(cell_loc) = model.updated_cells.pop() {
            // TODO: move cell render here
            self.update_cell_in_frame_buffer(model.get_cell(cell_loc).unwrap());
        }
    }

    pub fn play(mut self, wfc: Wfc) {
        self.wfc = Some(wfc);
        // TODO: call setup window func here
        let mut model = self.wfc.as_mut().unwrap().get_model();

        for cell in model.iter_cells() {
            self.update_cell_in_frame_buffer(cell);
        }
        let event_loop = self.event_loop.take().unwrap();
        event_loop.run(move |event, _, control_flow| {
            // update frame
            if let winit::event::Event::RedrawRequested(_window_id) = event {
                self.update_frame_buffer(&mut model);
                let mut exit = false;
                if let Err(err) = self.pixels.render() {
                    log::error!("pixels.render() failed: {err}");
                    exit = true;
                }
                if model.remaining_uncollapsed == 0 {
                    log::info!("Wfc completed");
                    // exit = true;
                }
                if exit {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                    return;
                }
            }
            model.step();
            self.window.request_redraw();
        });
    }

    pub fn render_cell(&mut self, cell_coord: UVec2, pattern: Pattern) {
        let frame = self.pixels.get_frame_mut();
        // let pattern = domain.filter_allowed(&self.patterns).next().unwrap();
        let frame_coord = cell_coord * self.tile_size as u32;
        for x in 0..self.tile_size {
            for y in 0..self.tile_size {
                let frame_idx = UVec2 {
                    x: x as u32,
                    y: y as u32,
                } + frame_coord;
                let idx = 4 * ((frame_idx.y * self.window_dimensions.x) + frame_idx.x) as usize;
                // let cell_pixel = pattern[y * self.tile_size + x].0;
                let cell_pixel: [u8; 4] = pattern[y * self.tile_size + x];
                let frame_pixel = frame
                    .get_mut(idx..idx + 4)
                    .unwrap_or_else(|| panic!("pixel at {:?} should be in bounds but loc {cell_coord:?} and frame cell {frame_idx:?} aren't in bounds", frame_idx));
                frame_pixel.copy_from_slice(&cell_pixel);
            }
        }
    }

    pub fn render(&self) {
        self.pixels.render().unwrap();
    }

    pub fn update<'a>(&mut self, image: impl Iterator<Item = &'a Rgba<u8>>) {
        let frame = self.pixels.get_frame_mut();
        for (cell_pixel, frame_pixel) in image.zip(frame.chunks_exact_mut(4)) {
            frame_pixel.copy_from_slice(&cell_pixel.0);
        }
    }
}

fn rgba_f32_to_u8(a: f32) -> u8 {
    return (a * 255.0) as u8;
}
pub fn blend_rgb(a: f32, b: f32, t: f32) -> f32 {
    return (((1.0 - t) * a.powi(2)) + (t * b.powi(2))).sqrt();
}

pub fn blend_alpha(a: f32, b: f32, t: f32) -> f32 {
    (1.0 - t) * a + t * b
}

pub fn blend_rgba(a: [u8; 4], b: [u8; 4], factor: f32) -> [u8; 4] {
    let conv_to_f32 = |c| (c as f32) * 255.0;
    let [ar, ag, ab, aa] = a.map(conv_to_f32);
    let [br, bg, bb, ba] = b.map(conv_to_f32);
    let t = factor;
    return [
        blend_rgb(ar, br, t),
        blend_rgb(ag, bg, t),
        blend_rgb(ab, bb, t),
        blend_alpha(aa, ba, t),
    ]
    .map(rgba_f32_to_u8);
}
