use glam::UVec2;
use image::Rgba;
use pixels::Pixels;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wfc_lib::{
    preprocessor::Pattern,
    simple_patterns::construct_simple_patterns,
    wfc::{Cell, Model},
    Wfc,
};
use winit::platform::web::WindowBuilderExtWebSys;
use winit::window::Window;

// const TILE_SIZE_DEFAULT: usize = 2;
// const PIXEL_SCALE_DEFAULT: u32 = 2;

fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Trace).expect("error initializing logger");
    // TODO: specifying seed for random weighting of tiles
    // TODO: render preprocessor steps
}

#[wasm_bindgen]
pub async fn run_wang_tile(bytes: &[u8]) {
    log::info!("run_wang_tile");
    WfcWindow::new(256, 256, 32).await.play(
        WfcWebBuilder::new_from_image_bytes(&bytes)
            .with_tile_size(32)
            .with_output_dimensions(256, 256)
            .wang(),
    );
}

// FIXME: get simple patterns working again
// #[allow(unused)]
// #[wasm_bindgen]
// pub async fn run_simple_patterns() {
//     WfcWindow::new(glam::UVec2::splat(40), 32).await.play(
//         construct_simple_patterns()
//             .with_tile_size(4)
//             .with_output_dimensions(40, 40),
//     )
// }

#[wasm_bindgen]
pub struct WfcWindow {
    window: Window,
    pixels: Pixels,
    tile_size: usize, // TODO: remove tile_size from this struct and use the variable used in self.wfc
    window_dimensions: UVec2,
    event_loop: Option<winit::event_loop::EventLoop<WfcEvent>>,
}

// TODO: merge this with WfcWebBuilder
#[wasm_bindgen]
impl WfcWindow {
    // TODO: move pixels setup to run function and remove output_dimensions and tile_size params
    pub async fn new(
        output_dimensions_x: u32,
        output_dimensions_y: u32,
        tile_size_var: usize,
    ) -> Self {
        let event_loop = winit::event_loop::EventLoopBuilder::<WfcEvent>::with_user_event().build();
        let canvas = web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.get_element_by_id("wfc"))
            .and_then(|canvas| canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("couldn't find canvas element with id=\"wfc\"");
        let window = {
            winit::window::WindowBuilder::new()
                .with_title("WFC")
                .with_canvas(Some(canvas))
                .build(&event_loop)
                .expect("WindowBuilder error")
        };
        log::info!("window created and attached to canvas");

        // let window = Rc::new(window);

        // FIXME: set clippy target arch to wasm32 to avoid
        // warnings and having to use this block to avoid them
        // #[cfg(target_arch = "wasm32")]
        // {
        // Retrieve current width and height dimensions of browser client window
        // let get_window_size = || {
        //     let client_window = web_sys::window().unwrap();
        //     winit::dpi::LogicalSize::new(
        //         client_window.inner_width().unwrap().as_f64().unwrap(),
        //         client_window.inner_height().unwrap().as_f64().unwrap(),
        //     )
        // };

        // let window = Rc::clone(&window);

        // Initialize winit window with current dimensions of browser client
        // window.set_inner_size(get_window_size());

        // let client_window = web_sys::window().unwrap();

        // Listen for resize event on browser client. Adjust winit window dimensions
        // on event trigger
        // let closure =
        //     wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
        //         let size = get_window_size();
        //         window.set_inner_size(size)
        //     }) as Box<dyn FnMut(_)>);
        // client_window
        //     .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
        //     .unwrap();
        // closure.forget();
        // }

        let size = window.inner_size();
        let surface_texture = pixels::SurfaceTexture::new(size.width, size.height, &window);
        log::info!("surface texture built");

        let pixels =
            pixels::PixelsBuilder::new(output_dimensions_x, output_dimensions_y, surface_texture)
                .blend_state(pixels::wgpu::BlendState::REPLACE)
                .build_async()
                .await
                .unwrap();
        log::info!("pixels built");
        return Self {
            window,
            pixels,
            tile_size: tile_size_var,
            event_loop: Some(event_loop),
            window_dimensions: UVec2 {
                x: output_dimensions_x,
                y: output_dimensions_y,
            },
        };
    }

    fn update_cell_in_frame_buffer(&mut self, cell: &Cell, patterns: &Vec<Pattern>) {
        self.render_cell(cell.loc, cell.render(patterns, self.tile_size));
    }

    fn update_frame_buffer(&mut self, model: &mut Model, patterns: &Vec<Pattern>) {
        while let Some(cell_loc) = model.updated_cells.pop() {
            // TODO: move cell render here
            self.update_cell_in_frame_buffer(model.get_cell(cell_loc).unwrap(), patterns);
        }
    }

    pub fn get_controller(&self) -> WfcController {
        let proxy = self.event_loop.as_ref().unwrap().create_proxy();
        return WfcController { event_loop_proxy: proxy };
    }

    pub fn play(mut self, mut wfc_builder: WfcWebBuilder) {
        // TODO: create surface texture and pixels buffer here
        // in order to allow running again with new model

        // TODO: consider moving WFCBuilder into web crate and exposing
        // it via wasm_bindgen so that it becomes the js api

        let (mut model, mut patterns) = wfc_builder.build();

        // load initial state of model
        for cell in model.iter_cells() {
            self.update_cell_in_frame_buffer(cell, &patterns);
        }
        let event_loop = self.event_loop.take().unwrap();
        let mut done = |m: &Model| m.remaining_uncollapsed == 0;
        let mut playing = true;
        event_loop.run(move |event, _, control_flow| {
            // TODO: handle window resizing

            if let winit::event::Event::UserEvent(WfcEvent::TogglePlaying) = event {
                log::warn!("toggle playing");
                playing = !playing;
            }
            if let winit::event::Event::RedrawRequested(_window_id) = event {
                let mut exit = false;

                if !done(&model) {
                    model.step();
                    self.update_frame_buffer(&mut model, &patterns);
                    if let Err(err) = self.pixels.render() {
                        log::error!("pixels.render() failed: {err}");
                        exit = true;
                    }
                }
                if exit {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                    return;
                }
            }
            if playing && !done(&model) {
                self.window.request_redraw();
            }

        });
    }

    fn render_cell(&mut self, cell_coord: UVec2, pattern: Pattern) {
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

    fn render(&self) {
        self.pixels.render().unwrap();
    }

    fn update<'a>(&mut self, image: impl Iterator<Item = &'a Rgba<u8>>) {
        let frame = self.pixels.get_frame_mut();
        for (cell_pixel, frame_pixel) in image.zip(frame.chunks_exact_mut(4)) {
            frame_pixel.copy_from_slice(&cell_pixel.0);
        }
    }
}

#[derive(Default)]
#[wasm_bindgen]
pub struct WfcWebBuilder {
    image: Option<image::RgbaImage>,
    processor_config: Option<wfc_lib::preprocessor::ProcessorConfig>,
    wfc_data: Option<wfc_lib::preprocessor::WfcData>,
    output_dims: Option<UVec2>,
    // TODO: remove importance of order: option 1: everything including (pixel scale and tile_size) are options, set with defaults when ran
    tile_size: usize,
    output_image: Option<image::RgbaImage>,
}

const TILE_SIZE_DEFAULT: usize = 2;
// TODO: proc macro / derive macro to generate these builder functions and
// set mutually exclusive fields
// also maybe assert functions?
#[wasm_bindgen]
impl WfcWebBuilder {
    fn setup() -> Self {
        return Self {
            tile_size: TILE_SIZE_DEFAULT,
            ..Default::default()
        };
    }
    fn new_from_image(image: image::RgbaImage) -> Self {
        let mut this = Self::setup();
        this.image = Some(image);
        this.processor_config = Some(wfc_lib::preprocessor::ProcessorConfig::default());
        return this;
    }
    fn load_image_from_bytes(raw_data: &[u8]) -> image::RgbaImage {
        let reader = image::io::Reader::new(std::io::Cursor::new(raw_data))
            .with_guessed_format()
            .expect("Cursor io never fails");
        let image = reader.decode().unwrap().to_rgba8();
        return image;
    }

    pub fn new_from_image_bytes(raw_data: &[u8]) -> Self {
        let image = Self::load_image_from_bytes(raw_data);
        return Self::new_from_image(image);
    }

    pub fn with_output_dimensions(mut self, x: u32, y: u32) -> Self {
        self.output_dims = Some(UVec2 { x, y });
        return self;
    }
    pub fn with_tile_size(mut self, tile_size: usize) -> Self {
        assert!(tile_size != 0, "tile size cannot be zero");
        // if from patterns
        if let Some(wfcdata) = &self.wfc_data {
            for pattern in &wfcdata.patterns {
                // .expect("wfc_data size should be set before patterns")
                assert!(
                    pattern.len() == tile_size.pow(2),
                    "pattern size: {} should match tile size squared: {}",
                    pattern.len(),
                    tile_size.pow(2),
                );
            }
        }
        self.tile_size = tile_size;
        return self;
    }
    // TODO: make PatternsBuilder that has FromImage and FromPatterns variants?
    pub fn wrap(mut self) -> Self {
        self.processor_config.as_mut().unwrap().wrap = true;
        return self;
    }

    pub fn wang(mut self) -> Self {
        self.processor_config.as_mut().unwrap().wang = true;
        return self;
    }

    pub fn wang_flip(mut self) -> Self {
        self.processor_config.as_mut().unwrap().wang_flip = true;
        return self;
    }

    fn get_patterns(&self) -> &Vec<Pattern> {
        return &self.wfc_data.as_ref().unwrap().patterns;
    }
    fn get_adjacency_rules(&self) -> &wfc_lib::adjacency_rules::AdjacencyRules {
        return &self.wfc_data.as_ref().unwrap().adjacency_rules;
    }
    fn get_tile_frequencies(&self) -> &Vec<usize> {
        return &self.wfc_data.as_ref().unwrap().tile_frequencies;
    }
    pub fn process_image(&mut self) {
        let mut processor = wfc_lib::preprocessor::PreProcessor::new(
            self.image.as_ref().expect("Image is set"),
            self.tile_size,
            self.processor_config
                .as_ref()
                .expect("ProcessorConfig is set")
                .clone(),
        );
        self.wfc_data = Some(processor.process());
    }

    fn build(&mut self) -> (Model, Vec<Pattern>) {
        self.process_image();
        let model = Model::new(
            // TODO: move actual values from wfc_data
            self.get_adjacency_rules().clone(),
            self.get_tile_frequencies().clone(),
            // TODO: pass tile size and output dims to model
            // and let it figure out the rest
            self.output_dims.unwrap() / self.tile_size as u32,
        );
        // TODO: remove clone of patterns
        // may require making all &mut self into mut self
        return (model, self.wfc_data.as_ref().unwrap().patterns.clone());
    }
}

enum WfcEvent {
    TogglePlaying,
}

#[wasm_bindgen]
pub struct WfcController {
    event_loop_proxy: winit::event_loop::EventLoopProxy<WfcEvent> ,
}

#[wasm_bindgen]
impl WfcController {
    pub fn toggle_playing(&self) {
        // Ignore result.
        // throws if event loop is not running, in which case do nothing
        let _ = self.event_loop_proxy.send_event(WfcEvent::TogglePlaying);
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
// #[allow(unused)]
// #[wasm_bindgen]
// pub async fn run_celtic() {
//     let image = include_bytes!("../../inputs/celtic.png");
//     WfcWindow::new(glam::UVec2::splat(256), 32).await.play(
//         Wfc::new_from_image_bytes(image)
//             .with_tile_size(32)
//             .with_output_dimensions(256, 256)
//             .wang()
//             .wang_flip(),
//     );
// }
//
// #[allow(unused)]
// #[wasm_bindgen]
// pub async fn run_dual() {
//     WfcWindow::new(glam::UVec2::splat(256), 32).await.play(
//         Wfc::new_from_image_path("./inputs/dual.png")
//             .with_tile_size(32)
//             .with_output_dimensions(256, 256)
//             .wang(),
//     );
// }
//
// #[allow(unused)]
// async fn render_celtic_patterns() {
//     let mut win = WfcWindow::new(glam::UVec2::splat(128), 64).await;
//     let image = image::io::Reader::open("./inputs/celtic.png")
//         .unwrap()
//         .decode()
//         .unwrap()
//         .to_rgba8();
//     let mut processor = wfc_lib::preprocessor::PreProcessor::new(
//         &image,
//         64,
//         wfc_lib::preprocessor::ProcessorConfig::default(),
//     );
//     let data = processor.process();
//     for (id, &loc) in processor.tiles.iter().enumerate() {
//         win.render_cell(loc / 64, data.patterns[id].clone());
//     }
//     loop {
//         win.render();
//     }
// }
//
// #[allow(unused)]
// async fn render_celtic() {
//     let mut win = WfcWindow::new(glam::UVec2::splat(128), 64).await;
//     let image_bytes = include_bytes!("../../inputs/celtic.png");
//     let image = Wfc::load_image_from_bytes(image_bytes);
//     win.update(image.pixels());
//     loop {
//         win.render();
//     }
// }
