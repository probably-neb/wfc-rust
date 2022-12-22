// use std::{iter::zip, thread, time::Duration};

use wfc_rust::{Wfc, simple_patterns::construct_simple_patterns, CompletionBehavior::*};
// use simplelog::*;
// use std::fs::File;
// use wfc_rust::IdMap;

// use pixels::Pixels;
// use winit;

// const TILE_SIZE: usize = 64;
// const MAX_OUTPUT_DIMS: UVec2 = UVec2 { x: 400, y: 400 };
// const OUTPUT_DIMS: UVec2 = UVec2 { x: 256, y: 256 };


fn main() {
    run_simple_patterns();
    // run_celtic();
    // render_celtic();
    // render_celtic_patterns();
}

#[allow(unused)]
fn run_celtic() {
     Wfc::new_from_image_path("./inputs/celtic.png")
        .with_tile_size(64)
        .with_output_dimensions(256, 256)
        .log()
        .run_render(KeepRunning);
}

fn render_celtic_patterns() {
    let mut win = wfc_rust::Window::new(glam::UVec2::splat(128), 4, 64);
    let image = image::io::Reader::open("./inputs/celtic.png")
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();
    let mut processor = wfc_rust::preprocessor::PreProcessor::new(&image, 64);
    let data = processor.process();
    for (id,&loc) in processor.tiles.iter().enumerate() {
        win.update_grid_cell(loc / 64, data.patterns[id].clone());
    }
    loop {
        win.render();
    }

}
fn render_celtic() {
    let mut win = wfc_rust::Window::new(glam::UVec2::splat(128), 4, 64);
    let image = image::io::Reader::open("./inputs/celtic.png")
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();
    win.update(image.pixels());
    loop {
        win.render();
    }
}

#[allow(unused)]
fn run_simple_patterns() {
    construct_simple_patterns()
        .with_tile_size(4)
        .with_output_dimensions(40, 40)
        .with_pixel_scale(12)
        .log()
        .run_render(KeepRunning);
}
