use core::time;
use std::thread;

use macroquad::prelude::*;
use macroquad::texture::{self, Image, Texture2D};
use macroquad::ui::root_ui;

use wfc_rust::preprocessor;

#[macroquad::main("WAVE FUNCTION COLLAPSE")]
async fn main() {
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

        set_camera(&Camera2D {
            zoom: vec2(zoom, zoom * screen_width() / screen_height()),
            // zoom: vec2(0.01,0.01),
            // target: vec2(15.0, 15.0),
            target: vec2(target.0, target.1),
            // offset: vec2(offset.0, offset.1),
            ..Default::default()
        });

        if play {
            thread::sleep(time::Duration::from_millis(10));
            // model.step();
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
