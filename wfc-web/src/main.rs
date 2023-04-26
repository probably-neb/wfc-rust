#![deny(unused_crate_dependencies)]
use glam::UVec2;
use thiserror::Error;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wfc_lib::preprocessor::PreProcessor;
use wfc_lib::tile::TileId;
use wfc_lib::{preprocessor::Pattern, wfc::Model};
use wgpu;
use winit::platform::web::WindowBuilderExtWebSys;
use winit::window::Window;

const TILE_SIZE_DEFAULT: usize = 2;

fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Warn).expect("error initializing logger");
    // TODO: specifying seed for random weighting of tiles
    // TODO: render preprocessor steps
}

// Wrapped in a stuct for wasm-bindgen purposes
#[wasm_bindgen]
pub struct WfcInstance {
    model: Model,
    data: WfcData,
}

#[wasm_bindgen]
pub struct WfcData {
    patterns: Vec<Pattern>,
    tile_size: usize,
    output_dimensions: UVec2,
    tile_frequencies: Vec<usize>,
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

    // TODO: wrap_input and wrap_output
    pub fn wrap(mut self) -> Self {
        self.processor_config.as_mut().unwrap().wrap = true;
        return self;
    }

    pub fn wang(mut self, val: Option<bool>) -> Self {
        self.processor_config.as_mut().unwrap().wang = val.unwrap_or(true);
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
        // FIXME: solution for creating different forms of preprocessor
        // intutively make processor builder that wfc builder either creates
        // behind the scenes or is passed
        let mut processor = wfc_lib::preprocessor::WangPreprocessor::new(self.tile_size);
        self.wfc_data =
            Some(processor.process(self.image.as_ref().expect("Image is set").to_owned()));
    }

    pub fn build(&mut self) -> WfcInstance {
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
        let tile_frequencies = self.wfc_data.as_ref().unwrap().tile_frequencies.clone();
        let tile_size = self.tile_size;
        return WfcInstance {
            model,
            data: WfcData {
                patterns,
                tile_size,
                output_dimensions,
                tile_frequencies,
            },
        };
    }
}

enum WfcEvent {
    TogglePlaying,
    LoadWfc(WfcInstance),
    StartWfc,
    CanvasResize(winit::dpi::PhysicalSize<u32>),
}

/// The public interface between the javascript frontend and the winit
/// event loop controlling the canvas displaying wfc for the current model
///
/// * `event_loop_proxy`: the link to the event_loop that we can send messages too
#[wasm_bindgen]
pub struct WfcController {
    event_loop_proxy: winit::event_loop::EventLoopProxy<WfcEvent>,
}

#[wasm_bindgen]
impl WfcController {
    pub fn init(display: &WfcWindow) -> Self {
        let event_loop_proxy = display.event_loop.as_ref().unwrap().create_proxy();
        return Self { event_loop_proxy };
    }

    pub fn toggle_playing(&self) {
        // Ignore result.
        // throws if event loop is not running, in which case do nothing
        let _ = self.event_loop_proxy.send_event(WfcEvent::TogglePlaying);
    }

    pub fn load_wfc(&self, data: WfcInstance) {
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
}

#[wasm_bindgen]
pub struct WfcWindow {
    window: Window,
    // Option because it is moved once it is started
    event_loop: Option<winit::event_loop::EventLoop<WfcEvent>>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
}

// TODO: merge this with WfcWebBuilder
#[wasm_bindgen]
impl WfcWindow {
    pub async fn new() -> Self {
        // panic on Error
        Self::new_impl().await.unwrap()
    }

    // TODO: move pixels setup to run function and remove output_dimensions and tile_size params
    async fn new_impl() -> Result<WfcWindow, RenderError> {
        // FIXME: set clippy target arch to wasm32 to avoid wasm
        // target errors
        let event_loop = winit::event_loop::EventLoopBuilder::<WfcEvent>::with_user_event().build();
        // TODO: take canvas ref as param
        let canvas = web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.get_element_by_id("wfc"))
            .and_then(|canvas| canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("couldn't find canvas element with id=\"wfc\"");
        let window = winit::window::WindowBuilder::new()
            .with_title("WFC")
            .with_canvas(Some(canvas.clone()))
            .build(&event_loop)
            .expect("WindowBuilder error");
        let size: winit::dpi::PhysicalSize<u32> =
            get_canvas_container_size().to_physical(window.scale_factor());
        window.set_inner_size(size);

        // TODO: use this size for surface texture
        // let size = get_canvas_container_size().to_physical(window.scale_factor());
        let backend = wgpu::Backends::BROWSER_WEBGPU;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface_from_canvas(&canvas)?;
        let adapter = match wgpu::util::initialize_adapter_from_env(&instance, backend) {
            Some(adapter) => Some(adapter),
            None => {
                instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        compatible_surface: Some(&surface),
                        force_fallback_adapter: false,
                        power_preference: wgpu::util::power_preference_from_env()
                            .unwrap_or_default(),
                    })
                    .await
            }
        }
        .ok_or(RenderError::RequestAdapter)?;

        let device_descriptor = wgpu::DeviceDescriptor {
            limits: adapter.limits(),
            ..wgpu::DeviceDescriptor::default()
        };
        let (device, queue) = adapter.request_device(&device_descriptor, None).await?;
        let surface_capabilities = surface.get_capabilities(&adapter);
        let texture_format = *surface_capabilities
            .formats
            .iter()
            .find(|format| format.describe().srgb)
            .unwrap_or(&wgpu::TextureFormat::Bgra8UnormSrgb);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::COPY_DST,
            format: texture_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        log::info!("window created and attached to canvas");

        Ok(Self {
            window,
            queue,
            surface,
            device,
            config,
            event_loop: Some(event_loop),
        })
    }

    pub fn start_event_loop(mut self) {
        let mut cur_model: Option<Model> = None;
        let mut render_ctx: Option<RenderContext> = None;

        // TODO: create done method in model
        let done = |m: &Model| m.remaining_uncollapsed == 0;
        let mut playing = true;

        // move event_loop out of self
        let event_loop = self.event_loop.take().unwrap();

        event_loop.run(move |event, _, control_flow| {
            match event {
                winit::event::Event::UserEvent(WfcEvent::LoadWfc(WfcInstance { model, data })) => {
                    // load initial state of model
                    let (out_x, out_y) = data.output_dimensions.into();
                    // TODO: come up with a nicer way to set the initial state
                    // that doesn't rerender the completely merged pattern
                    // for each cell
                    render_ctx.replace(RenderContext::new(data, &mut self));
                    // let new_renderer =
                    // pollster::block_on(RenderContext::build(data, &self.window)).unwrap();

                    // renderer.replace(new_renderer);
                    // let updated_cells = data.model.iter_cells().map(|c| c.loc).collect();
                    // update_frame_buffer(&mut self.pixels, &data, updated_cells);
                    self.window.request_redraw();
                    cur_model.replace(model);
                    playing = false;
                }
                winit::event::Event::UserEvent(WfcEvent::StartWfc) => {
                    assert!(cur_model.is_some());
                    playing = true;
                    self.window.request_redraw();
                }
                winit::event::Event::UserEvent(WfcEvent::CanvasResize(size)) => {
                    // TODO: use exisiting resize event
                    let size: winit::dpi::PhysicalSize<u32> =
                        get_canvas_container_size().to_physical(self.window.scale_factor());
                    self.window.set_inner_size(size);
                    // TODO: catch this error
                    // let _ = self.pixels.resize_surface(size.width, size.height);
                }
                winit::event::Event::UserEvent(WfcEvent::TogglePlaying) => {
                    if cur_model.is_some() {
                        playing = !playing;
                    }
                }
                winit::event::Event::RedrawRequested(_window_id) => {
                    if let Some(model) = &mut cur_model {
                        if playing && !done(&model) {
                            let updated_cells = model.step();
                            // update_frame_buffer(&mut self.pixels, &data, updated_cells);
                        }
                        // let err = self.pixels.render();
                        // if let Err(err) = err {
                        //     log::error!("pixels.render() failed: {err}");
                        //     *control_flow = winit::event_loop::ControlFlow::Exit;
                        // }
                        self.window.request_redraw();
                    }
                }
                _ => {}
            }
        });
    }
}

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("Failed to create surface")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("Request for adapter failed")]
    RequestAdapter,
    #[error("Request for device failed")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}

// The required data for displaying WfcData
struct RenderContext {
    data: WfcData,
}

impl RenderContext {
    fn new(data: WfcData, win: &mut WfcWindow) -> Self {
        // resize surface texture
        let size = winit::dpi::PhysicalSize { width: data.output_dimensions.x, height:  data.output_dimensions.y};
        win.config.width = size.width;
        win.config.height = size.height;
        win.surface.configure(&win.device, &win.config);
        let mut encoder = win.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("initial render encoder")
        });
        let frame = win.surface.get_current_texture().unwrap();
        win.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &frame.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0},
                aspect: wgpu::TextureAspect::All
            },
            &create_intial_image(&data),
            wgpu::ImageDataLayout {
                offset: 0,
                // TODO: track dependency between output dimensions and tile size (they must always
                // be aligned)
                bytes_per_row: std::num::NonZeroU32::new(4 * size.width),
                rows_per_image: std::num::NonZeroU32::new(size.height)
            },
            wgpu::Extent3d { width: size.width, height: size.height, depth_or_array_layers: 1}
        );
        win.queue.submit(Some(encoder.finish()));
        frame.present();

        Self {
            data
        }
    }
    fn render(&self, model: &Model) -> Result<(), wgpu::SurfaceError> {
        todo!();
    }
}

// blends all the patterns together and copies the merged pattern into every cell
fn create_intial_image(
    WfcData { output_dimensions, tile_size, patterns, tile_frequencies }: &WfcData
) -> Vec<u8> {
    let tile_size = tile_size.to_owned();
    let output_dimensions = output_dimensions.to_owned();

    let merged_pattern = {
        let mut counts = vec![[0; 4]; tile_size * tile_size];

        for pattern_id in 0..patterns.len() - 1 {
            let weight = tile_frequencies[pattern_id];
            for (i, px) in patterns[pattern_id].iter().enumerate() {
                counts[i][0] += px[0] as usize * weight;
                counts[i][1] += px[1] as usize * weight;
                counts[i][2] += px[2] as usize * weight;
                // new_pattern[i][3] += px[3] as usize* weight;
            }
        }

        let mut new_pattern: Pattern = vec![[0; 4]; tile_size * tile_size];
        let total_weight: usize = tile_frequencies.iter().sum();

        for (i, c) in counts.iter().enumerate() {
            new_pattern[i][0] = (c[0] / total_weight) as u8;
            new_pattern[i][1] = (c[1] / total_weight) as u8;
            new_pattern[i][2] = (c[2] / total_weight) as u8;
            // new_pattern[i][3] = (px[3] / total_weight;
            new_pattern[i][3] = 255;
        }
        new_pattern
    };
    // TODO: handle case where output_dimensions % tile_size != 0
    let grid_dims = output_dimensions / tile_size as u32;

    let buf: Vec<u8> = std::iter::repeat(
        merged_pattern
            .chunks(tile_size)
            .flat_map(|row| std::iter::repeat(row).take(grid_dims.x as usize))
            .flatten()
            .flatten()
            .cloned()
            .collect::<Vec<u8>>(),
    )
    .take(grid_dims.y as usize)
    .flatten()
    .collect();
    return buf;
}

// fn update_frame_buffer(pixels: &mut Pixels, data: &WfcData, mut updated_cells: Vec<UVec2>) {
//     let WfcData {
//         model,
//         patterns,
//         tile_size,
//         output_dimensions,
//     } = data;
//     let tile_size = *tile_size;
//
//     let frame = pixels.get_frame_mut();
//
//     while let Some(cell_loc) = updated_cells.pop() {
//         let cell = model.get_cell(cell_loc).unwrap();
//         // TODO: find more efficient way of determining whether cell
//         // has a single allowed pattern left
//         // this may require some sort of restructure of cell.domain
//         // as currently figuring out if it is the last remaining pattern
//         // requires iterating through all the patterns and cell.collapsed is
//         // not set
//
//         // per-pixel weighted average of the allowed patterns for this cell
//         let cell_pattern = if let Some(final_pattern) = cell.collapsed_to {
//             patterns[final_pattern].to_owned()
//         } else {
//             let mut counts = vec![[0; 4]; tile_size * tile_size];
//             let allowed_tile_ids = cell.domain.allowed_tile_ids();
//
//             for pattern_id in cell.domain.allowed_tile_ids() {
//                 let weight = cell.probability_dict.counts[pattern_id];
//                 for (i, px) in patterns[pattern_id].iter().enumerate() {
//                     counts[i][0] += px[0] as usize * weight;
//                     counts[i][1] += px[1] as usize * weight;
//                     counts[i][2] += px[2] as usize * weight;
//                     // new_pattern[i][3] += px[3] as usize* weight;
//                 }
//             }
//
//             let mut new_pattern: Pattern = vec![[0; 4]; tile_size * tile_size];
//             let total_weight = cell.probability_dict.total_count;
//
//             for (i, c) in counts.iter().enumerate() {
//                 new_pattern[i][0] = (c[0] / total_weight) as u8;
//                 new_pattern[i][1] = (c[1] / total_weight) as u8;
//                 new_pattern[i][2] = (c[2] / total_weight) as u8;
//                 // new_pattern[i][3] = (px[3] / total_weight;
//                 new_pattern[i][3] = 255;
//             }
//             new_pattern
//         };
//
//         // TODO: refactor to copy_from_slice rows at a time instead of pixels
// we know the necessary information to recalculate the average

//         let frame_coord = cell_loc * tile_size as u32;
//         for x in 0..tile_size {
//             for y in 0..tile_size {
//                 // TODO: simplify this logic
//                 let frame_idx = UVec2 {
//                     x: x as u32,
//                     y: y as u32,
//                 } + frame_coord;
//                 let idx = 4 * ((frame_idx.y * output_dimensions.x) + frame_idx.x) as usize;
//
//                 let frame_pixel = frame
//                     .get_mut(idx..idx + 4)
//                     .unwrap_or_else(|| panic!("pixel at {:?} should be in bounds but loc {cell_loc:?} and frame cell {frame_idx:?} aren't in bounds", frame_idx));
//
//                 let cell_pixel: [u8; 4] = cell_pattern[y * tile_size + x].map(|c| c as u8);
//
//                 frame_pixel.copy_from_slice(&cell_pixel);
//             }
//         }
//     }
// }

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

// FIXME: get simple patterns working again
// #[allow(unused)]
// #[wasm_bindgen]
// pub async fn run_simple_patterns() {
//     WfcWindow::new(glam::UVec2::splat(40), 32).await.play(
//         construct_simple_patterns()
//             .with_tile_size(4)
//             .with_output_dimensions(40, 40),
//     )
//}
// async fn create_pixels(window: &winit::window::Window, output_dimensions: UVec2) -> Pixels {
//     let surface_texture = pixels::SurfaceTexture::new(size.width, size.height, &window);
//     log::info!("surface texture built");
//
//     let pixels =
//         pixels::PixelsBuilder::new(output_dimensions.x, output_dimensions.y, surface_texture)
//             .blend_state(pixels::wgpu::BlendState::REPLACE)
//             .build_async()
//             .await
//             .unwrap();
//     log::info!("pixels built");
//     return pixels;
// }
//
