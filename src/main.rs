use glam::UVec2;
use image::{io::Reader as ImageReader, Rgba};

use wfc_rust::{
    preprocessor::{Pattern, PreProcessor, WfcData},
    wfc::Model, IdMap,
};
// use wfc_rust::IdMap;

use pixels::Pixels;
// use winit;

const TILE_SIZE: usize = 64;
// const MAX_OUTPUT_DIMS: UVec2 = UVec2 { x: 400, y: 400 };
// const OUTPUT_DIMS: UVec2 = UVec2 { x: 200, y: 200 };

fn main() {
    let image = ImageReader::open("./inputs/celtic.png")
        .expect("image loadable")
        .decode()
        .expect("image decodable");
    let mut processor = PreProcessor::new(image.fliph().flipv().to_rgba8(), TILE_SIZE);
    let WfcData {
        patterns,
        adjacency_rules,
        tile_frequencies,
    } = processor.process();
    let image_dims: UVec2 = processor.image.dimensions().into();
    let mut window = Window::new(image_dims, 2, TILE_SIZE);
    // let clocs = Grid(UVec2::splat(2)).iter_locs();
    // for (loc, pattern) in zip(clocs,patterns) {
    //     window.update_grid_cell(loc, pattern);
    // }
    let mut wfc = Model::new(
        adjacency_rules,
        tile_frequencies,
        image_dims / TILE_SIZE as u32,
    );
    for _i in 0..4 {
        wfc.collapse_cell();
    }

    for cell in wfc.iter_cells() {
        window.update_grid_cell(
            cell.loc,
            cell.domain
                .filter_allowed(&patterns)
                .next()
                .expect("no contradictions"),
        );
    }

    loop {
        window.render();
    }
}

pub struct Window {
    _window: winit::window::Window,
    pixels: Pixels,
    grid_dims: UVec2,
    tile_size: usize,
    patterns: IdMap<Pattern>,
    output_dimensions: UVec2,
}

impl Window {
    pub fn new(output_dimensions: UVec2, pixel_size: u32, tile_size_var: usize) -> Self {
        let event_loop = winit::event_loop::EventLoop::new();
        let size = winit::dpi::LogicalSize::new(
            output_dimensions.x * pixel_size,
            output_dimensions.y * pixel_size,
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
            pixels::Pixels::new(output_dimensions.x, output_dimensions.y, surface_texture).unwrap();
        let grid_dims = output_dimensions / tile_size_var as u32;
        Self {
            _window: window,
            pixels,
            grid_dims,
            tile_size: tile_size_var,
            patterns: IdMap::default(),
            output_dimensions,
        }
    }

    fn update_grid_cell(&mut self, cell_coord: UVec2, pattern: Pattern) {
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
                let cell_pixel = pattern[y * self.tile_size + x].0;
                let frame_pixel = frame
                    .get_mut(idx..idx + 4)
                    .unwrap_or_else(|| panic!("pixel at {:?} should be in bounds but loc {cell_coord:?} and frame cell {frame_idx:?} aren't in bounds", frame_idx));
                frame_pixel.copy_from_slice(&cell_pixel);
                // frame[idx*4+0] = r;
                // frame[idx*4+1] = g;
                // frame[idx*4+2] = b;
                // frame[idx*4+3] = a;
            }
        }
    }

    pub fn render(&self) {
        self.pixels.render().unwrap();
    }

    pub fn update<'a>(&mut self, image: impl Iterator<Item = &'a Rgba<u8>>) {
        let frame = self.pixels.get_frame_mut();
        for (cell_pixel, frame_pixel) in image.zip(frame.chunks_exact_mut(4)) {
            // let [r, g, b, a] = image_patterns.weighted_average_colour(&cell).0;
            // let [r, g, b, a] = cell_pixel.0;
            // frame_pixel[0] = r;
            // frame_pixel[1] = g;
            // frame_pixel[2] = b;
            // frame_pixel[3] = a;
            frame_pixel.copy_from_slice(&cell_pixel.0);
        }
    }
}

// #[macroquad::main("WAVE FUNCTION COLLAPSE")]
// async fn main() {
//     let mut zoom: f32 = 0.01;
//     let mut target: (f32, f32) = (0., 0.);
//     let mut prev_mouse_pos: (f32, f32) = (0., 0.);

//     let mut play = false;
//     let tile_size = 4;
//     let image = ImageReader::open("./inputs/celtic.png")
//         .expect("image loadable")
//         .decode()
//         .expect("image decodable");
//     let mut processor = PreProcessor::new(image.flipv().to_rgba8(), tile_size);
//     let (tile_freqs, adjacency_rules) = processor.process();
//     let output_dims: UVec2 = dbg!(processor.image.dimensions().into());
//     let texture_atlas = Texture2D::from_rgba8(
//         output_dims.x as u16,
//         output_dims.y as u16,
//         processor.image.to_vec().as_slice(),
//     );
//     texture_atlas.set_filter(FilterMode::Nearest);
//     let ts_u16 = tile_size as u16;
//     let tile_textures: IdMap<Texture2D> = processor
//         .tiles
//         .iter()
//         .map(|&loc| {
//             let pattern = processor.pattern_at(loc);
//             return Texture2D::from_rgba8(ts_u16, ts_u16, &pattern);
//         })
//         .collect();
//     let tile_texture_names: IdMap<String> = processor.tile_ids().map(|id| id.to_string()).collect();
//     // let material = load_material

//     let rects: IdMap<Rect> = processor
//         .tiles
//         .iter()
//         .map(|&v| {
//             let v = v.as_vec2();
//             let ts = tile_size as f32;
//             Rect {
//                 x: v.x,
//                 y: v.y,
//                 w: ts,
//                 h: ts,
//             }
//         })
//         .collect();

//     loop {
//         if is_mouse_button_down(MouseButton::Right) {
//             let mouse_pos = mouse_position();
//             target.0 += -1.0 * (mouse_pos.0 - prev_mouse_pos.0) * zoom;
//             target.1 += (mouse_pos.1 - prev_mouse_pos.1) * zoom;
//         }
//         if is_key_down(KeyCode::W) {
//             target.1 += 1.0;
//         }
//         if is_key_down(KeyCode::S) {
//             target.1 -= 1.0;
//         }
//         if is_key_down(KeyCode::A) {
//             target.0 -= 1.0;
//         }
//         if is_key_down(KeyCode::D) {
//             target.0 += 1.0;
//         }
//         match mouse_wheel() {
//             (_x, y) if y != 0.0 => {
//                 // Normalize mouse wheel values is browser (chromium: 53, firefox: 3)
//                 #[cfg(target_arch = "wasm32")]
//                 let y = if y < 0.0 {
//                     -1.0
//                 } else if y > 0.0 {
//                     1.0
//                 } else {
//                     0.0
//                 };
//                 zoom *= 1.1f32.powf(y);
//             }
//             _ => (),
//         }

//         set_camera(&Camera2D {
//             zoom: vec2(zoom, zoom * screen_width() / screen_height()),
//             // zoom: vec2(0.01,0.01),
//             // target: vec2(15.0, 15.0),
//             target: vec2(target.0, target.1),
//             // offset: vec2(offset.0, offset.1),
//             ..Default::default()
//         });

//         for (&loc, &id) in &processor.tile_loc_map {
//             let rect = rects[id];
//             draw_texture_ex(
//                 texture_atlas,
//                 loc.x as f32,
//                 loc.y as f32,
//                 WHITE,
//                 DrawTextureParams {
//                     source: Some(rect),
//                     ..Default::default()
//                 },
//             )
//         }

//         if play {
//             thread::sleep(time::Duration::from_millis(10));
//             // model.step();
//         }
//         if root_ui().button(None, "PLAY") {
//             play = !play;
//         }
//         // if root_ui().button(None, "Collapse") {
//         //     model.collapse();
//         //     // model.tile_at_mut(Loc {x:0,y:0}).dom =
//         // }
//         prev_mouse_pos = mouse_position();
//         next_frame().await
//     }
// }
