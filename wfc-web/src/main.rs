use glam::UVec2;
use image::Rgba;
use pixels::Pixels;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wfc_lib::{preprocessor::Pattern, wfc::Model};
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

// TODO: come up with a better name for this
#[wasm_bindgen]
pub struct WfcData {
    model: Model,
    patterns: Vec<Pattern>,
    tile_size: usize,
    output_dimensions: UVec2,
}

#[wasm_bindgen]
pub struct WfcWindow {
    window: Window,
    // Option because it is moved once it is started
    event_loop: Option<winit::event_loop::EventLoop<WfcEvent>>,
    pixels: Pixels,
}

// TODO: merge this with WfcWebBuilder
#[wasm_bindgen]
impl WfcWindow {
    // TODO: move pixels setup to run function and remove output_dimensions and tile_size params
    pub async fn new() -> Self {
        // FIXME: set clippy target arch to wasm32 to avoid wasm
        // target errors
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
        let pixels = create_pixels(&window, UVec2::splat(100)).await;

        return Self {
            window,
            event_loop: Some(event_loop),
            pixels,
        };
    }

    pub fn start_event_loop(mut self) {
        let mut cur_model_data: Option<WfcData> = None;

        // TODO: create done method in model
        let mut done = |m: &Model| m.remaining_uncollapsed == 0;
        let mut playing = true;

        let event_loop = self.event_loop.take().unwrap();

        event_loop.run(move |event, _, control_flow| {
            if let winit::event::Event::UserEvent(WfcEvent::StartWfc(data)) = event {
                // load initial state of model
                let (out_x, out_y) = data.output_dimensions.into();
                self.pixels.resize_buffer(out_x, out_y);
                // TODO: come up with a nicer way to set the initial state
                let updated_cells = data.model.iter_cells().map(|c| c.loc).collect();
                update_frame_buffer(&mut self.pixels, &data, updated_cells);
                self.window.request_redraw();

                cur_model_data.replace(data);
                return;
            }
            // TODO: handle window resizing by resizing pixels surface_texture
            if cur_model_data.is_none() {
                return;
            }
            if let Some(data) = &mut cur_model_data {
                match event {
                    winit::event::Event::UserEvent(WfcEvent::TogglePlaying) => {
                        log::warn!("toggle playing");
                        playing = !playing;
                    }
                    winit::event::Event::RedrawRequested(_window_id) => {
                        let mut exit = false;

                        if !done(&data.model) {
                            let updated_cells = data.model.step();

                            update_frame_buffer(&mut self.pixels, &data, updated_cells);
                            if let Err(err) = self.pixels.render() {
                                log::error!("pixels.render() failed: {err}");
                                exit = true;
                            }
                        } else {
                            exit = true;
                        }
                        if exit {
                            *control_flow = winit::event_loop::ControlFlow::Exit;
                            return;
                        }
                    }
                    _ => {}
                }
                if playing && !done(&data.model) {
                    self.window.request_redraw();
                }
            }
        });
    }
}

fn update_frame_buffer(pixels: &mut Pixels, data: &WfcData, mut updated_cells: Vec<UVec2>) {
    // FIXME: figure out a better way of keeping track of updated cells other than
    // in models state


    let WfcData {
        model,
        patterns,
        tile_size,
        output_dimensions,
    } = data;
    let tile_size = *tile_size;

    let frame = pixels.get_frame_mut();

    while let Some(cell_loc) = updated_cells.pop() {
        let cell = model.get_cell(cell_loc).unwrap();
        // TODO: inline cell.render here
        let cell_pattern = cell.render(patterns, tile_size);

        let frame_coord = cell_loc * tile_size as u32;
        for x in 0..tile_size {
            for y in 0..tile_size {
                // TODO: simplify this logic
                let frame_idx = UVec2 {
                    x: x as u32,
                    y: y as u32,
                } + frame_coord;
                let idx = 4 * ((frame_idx.y * output_dimensions.x) + frame_idx.x) as usize;
                // let cell_pixel = pattern[y * self.tile_size + x].0;
                let frame_pixel = frame
                    .get_mut(idx..idx + 4)
                    .unwrap_or_else(|| panic!("pixel at {:?} should be in bounds but loc {cell_loc:?} and frame cell {frame_idx:?} aren't in bounds", frame_idx));

                let cell_pixel: [u8; 4] = cell_pattern[y * tile_size + x];

                frame_pixel.copy_from_slice(&cell_pixel);
            }
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
        // TODO: consider whether decoding here is really necessary
        //
        // Assuming it is so that Image figures out how to give me the vec of
        // pixels I want
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

    pub fn wrap(mut self) -> Self {
        self.processor_config.as_mut().unwrap().wrap = true;
        return self;
    }

    pub fn wang(mut self) -> Self {
        self.processor_config.as_mut().unwrap().wang = true;
        return self;
    }

    pub fn wang_flip(mut self) -> Self {
        // TODO: test whether all wangs are wang-flips and delete this method
        // if they are
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

    pub fn build(&mut self) -> WfcData {
        // TODO: render preprocessing
        self.process_image();

        let output_dimensions = self.output_dims.unwrap();
        let model = Model::new(
            // TODO: move actual values from wfc_data
            self.get_adjacency_rules().clone(),
            self.get_tile_frequencies().clone(),
            // TODO: pass tile size and output dims to model
            // and let it figure out the rest
            output_dimensions / self.tile_size as u32,
        );
        // TODO: remove clone of patterns
        // may require making all &mut self into mut self
        let patterns = self.wfc_data.as_ref().unwrap().patterns.clone();
        let tile_size = self.tile_size;
        return WfcData {
            model,
            patterns,
            tile_size,
            output_dimensions,
        };
    }
}

enum WfcEvent {
    TogglePlaying,
    StartWfc(WfcData),
}

#[wasm_bindgen]
pub struct WfcController {
    event_loop_proxy: winit::event_loop::EventLoopProxy<WfcEvent>,
}

#[wasm_bindgen]
impl WfcController {
    pub fn init(display: &WfcWindow) -> Self {
        let event_loop_proxy = display
            .event_loop
            .as_ref()
            .expect("event loop was created")
            .create_proxy();
        return Self { event_loop_proxy };
    }

    pub fn toggle_playing(&self) {
        // Ignore result.
        // throws if event loop is not running, in which case do nothing
        let _ = self.event_loop_proxy.send_event(WfcEvent::TogglePlaying);
    }

    pub fn start_wfc(&self, data: WfcData) {
        let _ = self.event_loop_proxy.send_event(WfcEvent::StartWfc(data));
    }
}

async fn create_pixels(window: &winit::window::Window, output_dimensions: UVec2) -> Pixels {
    let size = window.inner_size();
    let surface_texture = pixels::SurfaceTexture::new(size.width, size.height, &window);
    log::info!("surface texture built");

    let pixels =
        pixels::PixelsBuilder::new(output_dimensions.x, output_dimensions.y, surface_texture)
            .blend_state(pixels::wgpu::BlendState::REPLACE)
            .build_async()
            .await
            .unwrap();
    log::info!("pixels built");
    return pixels;
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
