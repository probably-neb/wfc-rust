// use core::time;
// use std::thread;

use image::io::Reader as ImageReader;
use image::Rgba;
// use macroquad::prelude::*;
// use macroquad::texture::{self, Image, Texture2D};
// use macroquad::ui::root_ui;
use glam::UVec2;

use wfc_rust::preprocessor::PreProcessor;
// use wfc_rust::IdMap;

use pixels::Pixels;
use winit;

fn main() {
    let tile_size = 32;
    let image = ImageReader::open("./inputs/big-circuit.png")
        .expect("image loadable")
        .decode()
        .expect("image decodable");
    let mut processor = PreProcessor::new(image.flipv().to_rgba8(), tile_size);
    let (tile_freqs, adjacency_rules) = processor.process();
    let output_dims: UVec2 = processor.image.dimensions().into();
    let mut window = Window::new(output_dims, UVec2::splat(4));
    // let patterns = processor.tiles.iter().map(|&loc| processor.pattern_at(loc));
    window.update(image.to_rgba8().pixels());
    loop {
        window.render();
    }
}

pub struct Window {
    _window: winit::window::Window,
    pixels: Pixels,
    grid_size: UVec2,
}

impl Window {
    pub fn new(grid_size: UVec2, pixel_size: UVec2) -> Self {
        let event_loop = winit::event_loop::EventLoop::new();
        let size =
            winit::dpi::LogicalSize::new(grid_size.x * pixel_size.x, grid_size.y * pixel_size.y);
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
        let pixels = pixels::Pixels::new(grid_size.x, grid_size.y, surface_texture).unwrap();
        Self {
            _window: window,
            pixels,
            grid_size,
        }
    }

    pub fn render(&self) {
        self.pixels.render().unwrap();
    }

    pub fn update<'a>(
        &mut self,
        image: impl Iterator<Item = &'a Rgba<u8>>,
    ) {
        let frame = self.pixels.get_frame_mut();
        for (cell_pixel, pixel) in image.zip(frame.chunks_exact_mut(4)) {
            // let [r, g, b, a] = image_patterns.weighted_average_colour(&cell).0;
            let [r, g, b, a] = cell_pixel.0;
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
            pixel[3] = a;
        }
    }
}
