mod adjacency_map;
mod entropy;
mod image_utils;
mod point;
mod simple_patterns; // mod render;
mod wfc;
use core::time;
use point::Loc;
use std::thread;

use adjacency_map::AdjacencyMap;
mod domain;
extern crate futures;
extern crate macroquad;

use macroquad::prelude::*;
use macroquad::texture::{self, Image, Texture2D};
use macroquad::ui::root_ui;
use simple_patterns::get_simple_patterns;
use wfc::Model;

#[macroquad::main("WAVE FUNCTION COLLAPSE")]
async fn main() {
    const N: u16 = 4;
    let sps = get_simple_patterns();
    const PROBS: [f32; 5] = [0.2, 0.2, 0.2, 0.2, 0.2];
    let pictures: Vec<Image> = sps.iter().map(|simpl| simpl.img.clone()).collect();
    let out_dims = point::Dimens { x: 10, y: 10 };
    let adj_map = AdjacencyMap::from_individual_adj_maps(
        sps.iter()
            .map(|p| {
                (
                    p.id as usize,
                    AdjacencyMap::from_tup_array(p.allowed_neighbors),
                )
            })
            .collect(),
    );

    let mut model: Model = Model::new(5, out_dims, PROBS.to_vec(), pictures, adj_map);
    let mut zoom: f32 = 0.01;
    let mut target: (f32, f32) = (0., 0.);
    let mut prev_mouse_pos: (f32, f32) = (0., 0.);

    let mut play = false;

    loop {
        if is_mouse_button_down(MouseButton::Right) {
            let mouse_pos = mouse_position();
            target.0 += -1.0 * (mouse_pos.0 - prev_mouse_pos.0) * zoom;
            target.1 += (mouse_pos.1 - prev_mouse_pos.1) * zoom;
        }
        if is_key_down(KeyCode::W) {
            target.1 += 1.0;
        }
        if is_key_down(KeyCode::S) {
            target.1 -= 1.0;
        }
        if is_key_down(KeyCode::A) {
            target.0 -= 1.0;
        }
        if is_key_down(KeyCode::D) {
            target.0 += 1.0;
        }
        match mouse_wheel() {
            (_x, y) if y != 0.0 => {
                // Normalize mouse wheel values is browser (chromium: 53, firefox: 3)
                #[cfg(target_arch = "wasm32")]
                let y = if y < 0.0 {
                    -1.0
                } else if y > 0.0 {
                    1.0
                } else {
                    0.0
                };
                zoom *= 1.1f32.powf(y);
            }
            _ => (),
        }
        // TODO: render if cli arg says to render (or inverse)
        if let wfc::ModelStateEnum::Done = &model.state {
            clear_background(GREEN);
        } else if let wfc::ModelStateEnum::Bad = &model.state {
            clear_background(RED);
        } else {
            clear_background(WHITE);
        }

        set_camera(&Camera2D {
            zoom: vec2(zoom, zoom * screen_width() / screen_height()),
            // zoom: vec2(0.01,0.01),
            // target: vec2(15.0, 15.0),
            target: vec2(target.0, target.1),
            // offset: vec2(offset.0, offset.1),
            ..Default::default()
        });

        let imgs = model.to_images();
        let model_textures: Vec<(point::Loc, Texture2D)> = imgs
            .iter()
            .map(|(&loc, pic)| (loc, Texture2D::from_image(pic)))
            .collect();
        let nusize: usize = N.into();
        for (loc, img) in &model_textures {
            img.set_filter(texture::FilterMode::Nearest);
            draw_texture(
                *img,
                (nusize * loc.x) as f32,
                (nusize * loc.y) as f32,
                WHITE,
            );
        }
        if is_mouse_button_down(MouseButton::Left) {
            let mouse_pos = mouse_position();
            let adjusted_pos = (mouse_pos.0 / (nusize as f32), mouse_pos.1 / (nusize as f32));
            if adjusted_pos.0 > 0.0 && adjusted_pos.1 > 0.0 {
                if adjusted_pos.0 < (model.out_dims.x as f32)
                    && adjusted_pos.1 < (model.out_dims.y as f32)
                {
                    let x = adjusted_pos.0.round() as usize;
                    let y = adjusted_pos.1.round() as usize;
                    if x < model.out_dims.x && y < model.out_dims.y {
                        let pnt = Loc { x, y };
                        let tile = &model.board[pnt];
                        let str = format!("{tile:?}");
                        root_ui().label(None, &str);
                    }
                }
            }
        }

        if root_ui().button(None, "STEP") {
            model.step()
        }
        if play {
            thread::sleep(time::Duration::from_millis(10));
            model.step();
        }
        if root_ui().button(None, "PLAY") {
            play = !play;
        }
        // if root_ui().button(None, "Collapse") {
        //     model.collapse();
        //     // model.tile_at_mut(Loc {x:0,y:0}).dom =
        // }
        prev_mouse_pos = mouse_position();
        next_frame().await
    }
}
