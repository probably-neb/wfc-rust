mod simple_patterns;
// mod render;
mod point;
mod wfc;
extern crate futures;
extern crate macroquad;
use macroquad::prelude::*;
use macroquad::texture::{self, Image, Texture2D};
use simple_patterns::get_simple_patterns;

#[macroquad::main("BasicShapes")]
async fn main() {
    let n: u16 = 4;
    let sps = get_simple_patterns();
    let pictures: Vec<Image> = sps.iter().map(|simpl| simpl.img.clone()).collect();
    let default_image = Image::gen_image_color(n, n, BLACK);
    let out_dims = point::Dimens { x: 10, y: 10 };
    let model = wfc::Model::new(10, out_dims);
    println!("{:?}", model);
    let img = model_to_image(model, &pictures, &default_image);
    let model_textures: Vec<(point::Loc, Texture2D)> = img
        .iter()
        .map(|(loc, pic)| (*loc, Texture2D::from_image(pic)))
        .collect();
    let mut zoom: f32 = 0.01;
    let mut target: (f32,f32) = (0.,0.);
    let mut prev_mouse_pos: (f32,f32) = (0.,0.);
    // let mut mouse_offset = (0.,0.);

    loop {
        if is_mouse_button_down(MouseButton::Right) {
            let mouse_pos = mouse_position();
            target.0 += -1.0*(mouse_pos.0 - prev_mouse_pos.0)*zoom;
            target.1 += (mouse_pos.1 - prev_mouse_pos.1)*zoom;
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
        clear_background(WHITE);
        set_camera(&Camera2D {
            zoom: vec2(zoom, zoom * screen_width() / screen_height()),
            // zoom: vec2(0.01,0.01),
            // target: vec2(15.0, 15.0),
            target: vec2(target.0, target.1),
            // offset: vec2(offset.0, offset.1),
            ..Default::default()
        });

        let nusize: usize = n.into();
        for (loc, img) in &model_textures {
            img.set_filter(texture::FilterMode::Nearest);
            draw_texture(
                *img,
                (nusize * loc.x) as f32,
                (nusize * loc.y) as f32,
                WHITE,
            );
        }
        prev_mouse_pos = mouse_position();
        next_frame().await
    }
}

//credit: https://stackoverflow.com/a/29321264
fn blend_rgb_val(a: f32, b: f32, t: f32) -> f32 {
    let asqr = a.powi(2);
    let bsqr = b.powi(2);
    let invt = 1.0 - t;
    return ((invt * asqr) + (t * bsqr)).sqrt();
}

//credit: https://stackoverflow.com/a/29321264
fn blend_alpha_value(a: f32, b: f32, t: f32) -> f32 {
    return (1.0 - t) * a + t * b;
}

//credit: https://stackoverflow.com/a/29321264
fn blend_rgba(c1: Color, c2: Color) -> Color {
    let t: f32 = 0.5; //blend factor
    return Color {
        r: blend_rgb_val(c1.r, c2.r, t),
        g: blend_rgb_val(c1.g, c2.g, t),
        b: blend_rgb_val(c1.b, c2.b, t),
        a: blend_alpha_value(c1.a, c2.a, t),
    };
}

fn image_colors(img: &Image) -> Vec<Color> {
    return img
        .get_image_data()
        .iter()
        .map(|[r, g, b, a]| Color::from_rgba(*r, *g, *b, *a))
        .collect();
}

trait Merge {
    fn merge(a: &Self, b: &Self) -> Self;
}

impl Merge for Image {
    fn merge(a: &Self, b: &Self) -> Self {
        let mut newimg = a.clone();
        let newclrs: Vec<Color> = image_colors(a)
            .into_iter()
            .zip(image_colors(b).into_iter())
            .map(|(ac, bc)| blend_rgba(ac, bc))
            .collect();
        newimg.update(&newclrs);
        return newimg;
    }
}

fn image_from_tile_dom(dom: &wfc::IdVec, pictures: &[Image], def: &Image) -> Image {
    return dom
        .iter()
        .zip(pictures.iter())
        .filter(|(in_domain, _)| **in_domain)
        .map(|(_, pic)| pic)
        .fold(def.clone(), |p1, p2| {
            return Image::merge(&p1, p2);
        });
}

fn model_to_image(model: wfc::Model, pictures: &[Image], def: &Image) -> Vec<(point::Loc, Image)> {
    return model
        .board
        .iter()
        .map(|tile| (tile.loc, image_from_tile_dom(&tile.dom, pictures, def)))
        .collect();
}
