pub mod adjacency_rules;
pub mod preprocessor;
pub mod tile;
pub mod wfc;

use adjacency_rules::AdjacencyRules;
use derive_more::IsVariant;
use derive_more::{Deref, DerefMut, From};
use glam::UVec2;
use image::{io::Reader as ImageReader, Rgba, RgbaImage};
use pixels::Pixels;
use preprocessor::{Pattern, PreProcessor, ProcessorConfig, WfcData};
use simplelog::*;
use log::error;
use std::{fmt::Debug, fs::File, path::Path};
use tile::IdMap;
use wfc::Model;

const TILE_SIZE_DEFAULT: usize = 2;
const PIXEL_SCALE_DEFAULT: u32 = 2;

#[derive(Default)]
pub struct Wfc {
    creation_mode: CreationMode,
    image: Option<RgbaImage>,
    processor_config: Option<ProcessorConfig>,
    pub wfc_data: Option<WfcData>,
    output_dims: Option<UVec2>,
    // TODO: remove importance of order: option 1: everything including (pixel scale and tile_size) are options, set with defaults when ran
    tile_size: usize,
    pixel_scale: u32,
    output_image: Option<RgbaImage>,
}

impl Debug for Wfc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wfc")
            .field("creation_mode", &self.creation_mode)
            // .field("image", &self.image)
            .field("processor_config", &self.processor_config)
            .field("wfc_data", &self.wfc_data)
            .field("output_dims", &self.output_dims)
            .field("tile_size", &self.tile_size)
            .field("pixel_scale", &self.pixel_scale)
            // .field("output_image", &self.output_image)
            .finish()
    }
}

// TODO: proc macro / derive macro to generate these builder functions and
// set mutually exclusive fields
// also maybe assert functions?
impl Wfc {
    fn setup() -> Self {
        return Self {
            tile_size: TILE_SIZE_DEFAULT,
            pixel_scale: PIXEL_SCALE_DEFAULT,
            ..Default::default()
        };
    }
    pub fn new_from_image(image: RgbaImage) -> Self {
        let mut this = Self::setup();
        this.image = Some(image);
        this.creation_mode = CreationMode::FromImage;
        this.processor_config = Some(ProcessorConfig::default());
        return this;
    }
    fn load_image<P>(path: P) -> RgbaImage
    where
        P: AsRef<Path>,
    {
        return ImageReader::open(path)
            .expect("image loadable")
            .decode()
            .expect("image decodable")
            // .fliph()
            // .flipv()
            .to_rgba8();
    }
    pub fn new_from_image_path(path: &str) -> Self {
        let image = Self::load_image(path);
        return Self::new_from_image(image);
    }
    pub fn new_from_pattern_paths(
        paths: IdMap<String>,
        adjacency_rules: AdjacencyRules,
        tile_frequencies: IdMap<usize>,
    ) -> Self {
        let patterns: IdMap<Pattern> = paths
            .iter()
            // load image
            .map(Self::load_image)
            // map Rgba<u8> to [u8; 4]
            .map(|img| img.pixels().map(|rgba| rgba.0).collect())
            .collect();
        return Self::new_from_patterns(WfcData {
            tile_frequencies,
            adjacency_rules,
            patterns,
        });
    }

    pub fn new_from_patterns(wfcdata: WfcData) -> Self {
        let mut this = Self::setup();
        // assert!(t.image.is_none(), "wfc from source image and from patterns are mutually exclusive");
        this.wfc_data = Some(wfcdata);
        this.creation_mode = CreationMode::FromPatterns;
        return this;
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
    pub fn log(self) -> Self {
        CombinedLogger::init(vec![
            TermLogger::new(
                LevelFilter::Info,
                Config::default(),
                TerminalMode::Mixed,
                ColorChoice::Auto,
            ),
            WriteLogger::new(
                LevelFilter::Info,
                Config::default(),
                File::create("log").unwrap(),
            ),
        ])
        .unwrap();
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

    pub fn with_pixel_scale(mut self, pixel_scale: u32) -> Self {
        self.pixel_scale = pixel_scale;
        return self;
    }
    fn get_patterns(&self) -> &Vec<Pattern> {
        return &self.wfc_data.as_ref().unwrap().patterns;
    }
    fn get_adjacency_rules(&self) -> &AdjacencyRules {
        return &self.wfc_data.as_ref().unwrap().adjacency_rules;
    }
    fn get_tile_frequencies(&self) -> &Vec<usize> {
        return &self.wfc_data.as_ref().unwrap().tile_frequencies;
    }
    pub fn process_image(&mut self) {
        assert!(self.creation_mode.is_from_image());
        let mut processor = PreProcessor::new(
            self.image.as_ref().expect("Image is set"),
            self.tile_size,
            self.processor_config
                .as_ref()
                .expect("ProcessorConfig is set")
                .clone(),
        );
        self.wfc_data = Some(processor.process());
    }

    fn get_model(&mut self) -> Model {
        if self.creation_mode.is_from_image() {
            self.process_image();
        }
        let model = Model::new(
            self.get_adjacency_rules().clone(),
            self.get_tile_frequencies().clone(),
            self.output_dims.unwrap() / self.tile_size as u32,
        );
        return model;
    }

    fn run(mut self) {
        // thread::sleep(Duration::from_millis(250));
        let mut model = self.get_model();
        while model.remaining_uncollapsed > 0 {
            model.step();
        }
        // TODO: save final image
    }
}

#[derive(IsVariant)]
pub enum CompletionBehavior {
    KeepRunning,
    StopWhenCompleted,
}

#[derive(Clone, Copy, Debug, Default, IsVariant)]
enum CreationMode {
    FromImage,
    FromPatterns,
    #[default]
    Unknown,
}

pub struct WfcWindow {
    window: winit::window::Window,
    pixels: Pixels,
    tile_size: usize,
    output_dimensions: UVec2,
    event_loop: Option<winit::event_loop::EventLoop<()>>,
    wfc: Option<Wfc>,
}

impl WfcWindow {
    pub fn new(window_dimensions: UVec2, pixel_size: u32, tile_size_var: usize) -> Self {
        let event_loop = winit::event_loop::EventLoop::new();
        let size = winit::dpi::LogicalSize::new(
            window_dimensions.x * pixel_size,
            window_dimensions.y * pixel_size,
        );
        let window = winit::window::WindowBuilder::new()
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
            .build(&event_loop)
            .unwrap();
        let hidpi_factor = window.scale_factor();
        let p_size = size.to_physical::<f64>(hidpi_factor);
        let surface_texture = pixels::SurfaceTexture::new(
            p_size.width.round() as u32,
            p_size.height.round() as u32,
            &window,
        );
        let pixels =
            pixels::PixelsBuilder::new(window_dimensions.x, window_dimensions.y, surface_texture)
                .blend_state(pixels::wgpu::BlendState::REPLACE)
                .build()
                .unwrap();
        Self {
            window,
            pixels,
            tile_size: tile_size_var,
            output_dimensions: window_dimensions,
            event_loop: Some(event_loop),
            wfc: None,
        }
    }

    fn update_cell_in_frame_buffer(&mut self, cell: &wfc::Cell) {
        self.render_cell(cell.loc, cell.render(self.wfc.as_ref().unwrap().get_patterns(), self.tile_size));
    }
    fn update_frame_buffer(&mut self, model: &mut Model) {
        while let Some(cell_loc) = model.updated_cells.pop() {
            // TODO: move cell render here
            self.update_cell_in_frame_buffer(model.get_cell(cell_loc).unwrap());
        }
    }

    pub fn play(mut self, close_behavior: CompletionBehavior, wfc: Wfc) {
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
                    error!("pixels.render() failed: {err}");
                    exit = true;
                }
                if model.remaining_uncollapsed == 0 && close_behavior.is_stop_when_completed() {
                    log::info!("Wfc completed");
                    exit = true;
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
                let idx = 4 * ((frame_idx.y * self.output_dimensions.x) + frame_idx.x) as usize;
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

pub trait Area {
    type Output;
    fn area(&self) -> Self::Output;
}

impl Area for Grid {
    type Output = u32;

    fn area(&self) -> Self::Output {
        return self.x * self.y;
    }
}

#[derive(Deref, DerefMut, From, Clone, Debug, Default)]
pub struct Grid(pub UVec2);

impl Grid {
    pub fn iter_locs(&self) -> impl Iterator<Item = UVec2> {
        return UVec2Iter::new(UVec2::ZERO, self.0);
    }
}

#[derive(Clone, Debug)]
pub struct UVec2Iter {
    pub cur: UVec2,
    pub end: UVec2,
}

impl UVec2Iter {
    pub fn new(start: UVec2, end: UVec2) -> Self {
        return Self { cur: start, end };
    }
}

impl Iterator for UVec2Iter {
    type Item = UVec2;

    fn next(&mut self) -> Option<Self::Item> {
        let mut ret = Some(self.cur);
        if self.cur.x == self.end.x {
            self.cur.x = 0;
            self.cur.y += 1;
            ret = Some(self.cur);
        }
        if self.cur.y == self.end.y {
            ret = None
        } else {
            self.cur.x += 1;
        }
        return ret;
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
pub mod simple_patterns {
    use super::*;
    use adjacency_rules::CardinalDirs;
    use CardinalDirs::*;
    pub const CHARS: [&str; 5] = ["' '", "┓", "┛", "┏", "┗"];

    const PRINT_CREATION: bool = false;

    fn allow_all(
        aaa: [usize; 2],
        bbb: [usize; 2],
        dir: CardinalDirs,
        adjacency_rules: &mut AdjacencyRules,
    ) {
        for a in aaa {
            for b in bbb {
                adjacency_rules.allow(a, b, dir);
                if PRINT_CREATION {
                    let ac = CHARS[a];
                    let bc = CHARS[b];
                    println!("Allowing:");
                    match dir {
                        Up => {
                            println!("{}", bc);
                            println!("{}", ac);
                        }
                        Down => {
                            println!("{}", ac);
                            println!("{}", bc);
                        }
                        Left => {
                            println!("{}{}", bc, ac)
                        }
                        Right => {
                            println!("{}{}", ac, bc)
                        }
                    }
                }
            }
        }
    }

    pub const BLANK: usize = 0; //' '
    pub const DL: usize = 1; // ┓
    pub const LU: usize = 2; // ┛
    pub const RD: usize = 3; // ┏
    pub const UR: usize = 4; // ┗

    // ┓ ┛
    pub const BLANK_RIGHT: [usize; 2] = [DL, LU];
    // ┏ ┗
    pub const BLANK_LEFT: [usize; 2] = [RD, UR];
    // ┏ ┓
    pub const BLANK_UP: [usize; 2] = [RD, DL];
    // ┗ ┛
    pub const BLANK_DOWN: [usize; 2] = [UR, LU];

    pub const B2: [usize; 2] = [BLANK, BLANK];

    pub fn construct_simple_patterns() -> Wfc {
        let mut adjacency_rules = AdjacencyRules::new();
        let paths: IdMap<String> = vec!["blank", "dl", "lu", "rd", "ur"]
            .iter()
            .map(|&name| format!("./inputs/simple/{}.png", name))
            .collect();
        let tile_frequencies: IdMap<usize> = vec![1, 2, 2, 2, 2];

        // matching blank top / bottom
        allow_all(BLANK_UP, BLANK_DOWN, Up, &mut adjacency_rules);

        // connecting arm top / bottom
        allow_all(BLANK_DOWN, BLANK_UP, Up, &mut adjacency_rules);

        // matching blank left / right
        allow_all(BLANK_RIGHT, BLANK_LEFT, Right, &mut adjacency_rules);

        // connecting arm left / right
        allow_all(BLANK_LEFT, BLANK_RIGHT, Right, &mut adjacency_rules);

        allow_all(B2, BLANK_LEFT, Right, &mut adjacency_rules);
        allow_all(B2, BLANK_RIGHT, Left, &mut adjacency_rules);
        allow_all(B2, BLANK_UP, Down, &mut adjacency_rules);
        allow_all(B2, BLANK_DOWN, Up, &mut adjacency_rules);
        for &dir in CardinalDirs::iter() {
            adjacency_rules.allow(BLANK, BLANK, dir);
        }

        return Wfc::new_from_pattern_paths(paths, adjacency_rules, tile_frequencies)
            .with_tile_size(4);
    }
}
