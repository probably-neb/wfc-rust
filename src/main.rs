use wfc_rust::{simple_patterns::construct_simple_patterns, CompletionBehavior::*, Wfc, WfcWindow};

fn main() {
    // run_simple_patterns();
    run_celtic();
    // render_celtic();
    // render_celtic_patterns();
}

#[allow(unused)]
fn run_celtic() {
    WfcWindow::new(glam::UVec2::splat(256), 2, 32).play(
        KeepOpen,
        Wfc::new_from_image_path("./inputs/celtic.png")
            .with_tile_size(32)
            .with_output_dimensions(256,256)
            .log()
            .wang()
            .wang_flip()
    );
}

#[allow(unused)]
fn run_dual() {
    WfcWindow::new(glam::UVec2::splat(256), 2, 32).play(
        KeepOpen,
        Wfc::new_from_image_path("./inputs/dual.png")
            .with_tile_size(32)
            .with_output_dimensions(256, 256)
            .log()
            .wang()
    );
}

#[allow(unused)]
fn render_celtic_patterns() {
    let mut win = wfc_rust::WfcWindow::new(glam::UVec2::splat(128), 4, 64);
    let image = image::io::Reader::open("./inputs/celtic.png")
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();
    let mut processor = wfc_rust::preprocessor::PreProcessor::new(
        &image,
        64,
        wfc_rust::preprocessor::ProcessorConfig::default(),
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
fn render_celtic() {
    let mut win = wfc_rust::WfcWindow::new(glam::UVec2::splat(128), 4, 64);
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
    WfcWindow::new(glam::UVec2::splat(40), 12, 32).play(
        KeepOpen,
        construct_simple_patterns()
            .with_tile_size(4)
            .with_output_dimensions(40, 40)
            .log(),
    );
}
