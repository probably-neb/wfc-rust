use std::iter::zip;
use std::rc::Rc;

use glam::UVec2;
use image::Rgba;
use pixels::Pixels;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wfc_lib::preprocessor::PreProcessor;
use wfc_lib::tile::TileId;
use wfc_lib::{preprocessor::Pattern, wfc::Model};
use winit::platform::web::WindowBuilderExtWebSys;
use winit::window::Window;

const TILE_SIZE_DEFAULT: usize = 2;

fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Warn).expect("error initializing logger");
    // TODO: specifying seed for random weighting of tiles
    // TODO: render preprocessor steps
}

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
        let size: winit::dpi::PhysicalSize<u32> =
            get_canvas_container_size().to_physical(window.scale_factor());
        window.set_inner_size(size);
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
        let done = |m: &Model| m.remaining_uncollapsed == 0;
        let mut playing = true;

        let mut done_callback: Option<Box<dyn FnOnce()>> = None;

        let event_loop = self.event_loop.take().unwrap();

        event_loop.run(move |event, _, control_flow| {
            match event {
                // TODO: separate into loadwfc and startwfc events
                // and load preset on page load
                winit::event::Event::UserEvent(WfcEvent::LoadWfc(data)) => {
                    // load initial state of model
                    let (out_x, out_y) = data.output_dimensions.into();
                    let _ = self.pixels.resize_buffer(out_x, out_y);
                    // TODO: come up with a nicer way to set the initial state
                    // that doesn't rerender the completely merged pattern
                    // for each cell
                    let updated_cells = data.model.iter_cells().map(|c| c.loc).collect();
                    update_frame_buffer(&mut self.pixels, &data, updated_cells);
                    self.window.request_redraw();
                    cur_model_data.replace(data);
                    playing = false;
                }
                winit::event::Event::UserEvent(WfcEvent::StartWfc) => {
                    assert!(cur_model_data.is_some());
                    playing = true;
                    self.window.request_redraw();
                }
                winit::event::Event::UserEvent(WfcEvent::CanvasResize(size)) => {
                    // TODO: use exisiting resize event
                    let size = get_canvas_container_size().to_physical(self.window.scale_factor());
                    self.window.set_inner_size(size);
                    // TODO: catch this error
                    let _ = self.pixels.resize_surface(size.width, size.height);
                }
                winit::event::Event::UserEvent(WfcEvent::TogglePlaying) => {
                    if cur_model_data.is_some() {
                        playing = !playing;
                    }
                }
                winit::event::Event::UserEvent(WfcEvent::SetDoneCallback(cb)) => {
                    done_callback.replace(cb);
                }
                winit::event::Event::RedrawRequested(_window_id) => {
                    let data = match &mut cur_model_data {
                        Some(data) => data,
                        Nonde => return,
                    };

                    if playing && done(&data.model) && done_callback.is_some() {
                        // move cb out of option
                        let cb = done_callback.take().unwrap();
                        cb();
                    }
                    if playing && !done(&data.model) {
                        let updated_cells = data.model.step();
                        update_frame_buffer(&mut self.pixels, &data, updated_cells);
                    }
                    let err = self.pixels.render();
                    if let Err(err) = err {
                        log::error!("pixels.render() failed: {err}");
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                    // psuedo recursive call
                    self.window.request_redraw();
                }
                _ => {}
            }
        });
    }
}

fn update_frame_buffer(pixels: &mut Pixels, data: &WfcData, mut updated_cells: Vec<UVec2>) {
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
        // TODO: find more efficient way of determining whether cell
        // has a single allowed pattern left
        // this may require some sort of restructure of cell.domain
        // as currently figuring out if it is the last remaining pattern
        // requires iterating through all the patterns and cell.collapsed is
        // not set

        // per-pixel weighted average of the allowed patterns for this cell
        let cell_pattern = if let Some(final_pattern) = cell.collapsed_to {
            patterns[final_pattern].to_owned()
        } else {
            let mut counts = vec![[0; 4]; tile_size * tile_size];
            let allowed_tile_ids = cell.domain.allowed_tile_ids();

            for pattern_id in cell.domain.allowed_tile_ids() {
                let weight = cell.probability_dict.counts[pattern_id];
                for (i, px) in patterns[pattern_id].iter().enumerate() {
                    counts[i][0] += px[0] as usize * weight;
                    counts[i][1] += px[1] as usize * weight;
                    counts[i][2] += px[2] as usize * weight;
                    // new_pattern[i][3] += px[3] as usize* weight;
                }
            }

            let mut new_pattern: Pattern = vec![[0; 4]; tile_size * tile_size];
            let total_weight = cell.probability_dict.total_count;

            for (i, c) in counts.iter().enumerate() {
                new_pattern[i][0] = (c[0] / total_weight) as u8;
                new_pattern[i][1] = (c[1] / total_weight) as u8;
                new_pattern[i][2] = (c[2] / total_weight) as u8;
                // new_pattern[i][3] = (px[3] / total_weight;
                new_pattern[i][3] = 255;
            }
            new_pattern
        };

        // TODO: refactor to copy_from_slice rows at a time instead of pixels
        let frame_coord = cell_loc * tile_size as u32;
        for x in 0..tile_size {
            for y in 0..tile_size {
                // TODO: simplify this logic
                let frame_idx = UVec2 {
                    x: x as u32,
                    y: y as u32,
                } + frame_coord;
                let idx = 4 * ((frame_idx.y * output_dimensions.x) + frame_idx.x) as usize;

                let frame_pixel = frame
                    .get_mut(idx..idx + 4)
                    .unwrap_or_else(|| panic!("pixel at {:?} should be in bounds but loc {cell_loc:?} and frame cell {frame_idx:?} aren't in bounds", frame_idx));

                let cell_pixel: [u8; 4] = cell_pattern[y * tile_size + x].map(|c| c as u8);

                frame_pixel.copy_from_slice(&cell_pixel);
            }
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(remote = "UVec2")]
pub struct PlayerSettingsOutputDimensions {
    x: u32,
    y: u32,
}

#[derive(serde::Deserialize)]
pub struct PlayerSettings {
    tile_size: u32,
    #[serde(with = "PlayerSettingsOutputDimensions")]
    output_dimensions: UVec2,
    wang: bool,
    image: String,
    // TODO: wrap
    // TODO: wang_flip
    // TODO: reflect
    // TODO: rotate
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

// TODO: proc macro / derive macro to generate these builder functions and
// set mutually exclusive fields
// also maybe assert functions?
#[wasm_bindgen]
impl WfcWebBuilder {
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


    pub fn build_from_json_settings(image_bytes: &[u8], settings: JsValue) -> WfcData {
        let image = Self::load_image_from_bytes(image_bytes);
        let settings: PlayerSettings = serde_wasm_bindgen::from_value(settings).unwrap();

        let mut processor =
            wfc_lib::preprocessor::WangPreprocessor::new(settings.tile_size as usize);
        let pp_data = processor.process(image);
        let model = Model::new(
            pp_data.adjacency_rules,
            pp_data.tile_frequencies,
            settings.output_dimensions / settings.tile_size,
        );
        return WfcData {
            model,
            patterns: pp_data.patterns,
            tile_size: settings.tile_size as usize,
            output_dimensions: settings.output_dimensions,
        };
    }
}

enum WfcEvent {
    TogglePlaying,
    LoadWfc(WfcData),
    StartWfc,
    CanvasResize(winit::dpi::PhysicalSize<u32>),
    SetDoneCallback(Box<dyn FnOnce()>),
}

#[wasm_bindgen]
/// The public interface between the javascript frontend and the winit
/// event loop controlling the canvas displaying wfc for the current model
///
/// * `event_loop_proxy`: the link to the event_loop that we can send messages too
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

    pub fn load_wfc(&self, data: WfcData) {
        let _ = self.event_loop_proxy.send_event(WfcEvent::LoadWfc(data));
    }

    pub fn start_wfc(&self) {
        let _ = self.event_loop_proxy.send_event(WfcEvent::StartWfc);
    }

    pub fn resize_canvas(&self, w: u32, h: u32) {
        let size = winit::dpi::PhysicalSize::new(w, h);
        let _ = self
            .event_loop_proxy
            .send_event(WfcEvent::CanvasResize(size));
    }

    pub fn set_done_callback(&self, callback: js_sys::Function) {
        let _ = self
            .event_loop_proxy
            .send_event(WfcEvent::SetDoneCallback(Box::new(move || {
                callback.call0(&JsValue::NULL).unwrap();
            })));
    }
}

async fn create_pixels(window: &winit::window::Window, output_dimensions: UVec2) -> Pixels {
    let size = get_canvas_container_size().to_physical(window.scale_factor());
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

fn get_canvas_container_size() -> winit::dpi::LogicalSize<u32> {
    let canvas_container = web_sys::window()
        .expect("window exists")
        .document()
        .expect("document exists")
        .get_element_by_id("canvas-container")
        .expect("canvas-container exists")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("canvas container is html element");

    let canvas_width: u32 = canvas_container
        .client_width()
        .try_into()
        .expect("canvas width (i32) is within bounds of u32");
    let canvas_height: u32 = canvas_container
        .client_height()
        .try_into()
        .expect("canvas height (i32) is within bounds of u32");

    return winit::dpi::LogicalSize::new(canvas_width, canvas_height);
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
