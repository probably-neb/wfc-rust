use macroquad::prelude::*;
use macroquad::texture::{self, Image, Texture2D};

//credit: https://stackoverflow.com/a/29321264
pub fn blend_rgb_val(a: f32, b: f32, t: f32) -> f32 {
    let asqr = a.powi(2);
    let bsqr = b.powi(2);
    let invt = 1.0 - t;
    return ((invt * asqr) + (t * bsqr)).sqrt();
}

//credit: https://stackoverflow.com/a/29321264
pub fn blend_alpha_value(a: f32, b: f32, t: f32) -> f32 {
    return (1.0 - t) * a + t * b;
}

//credit: https://stackoverflow.com/a/29321264
pub fn blend_rgba(c1: Color, c2: Color) -> Color {
    let t: f32 = 0.5; //blend factor
    return Color {
        r: blend_rgb_val(c1.r, c2.r, t),
        g: blend_rgb_val(c1.g, c2.g, t),
        b: blend_rgb_val(c1.b, c2.b, t),
        a: blend_alpha_value(c1.a, c2.a, t),
    };
}

pub fn image_colors(img: &Image) -> Vec<Color> {
    return img
        .get_image_data()
        .iter()
        .map(|[r, g, b, a]| Color::from_rgba(*r, *g, *b, *a))
        .collect();
}

pub fn is_image_empty(i: &Image) -> bool {
    return i.width == 0 && i.height == 0;
}

pub trait Merge {
    fn merge(a: &Self, b: &Self) -> Self;
    fn merge_mut(a: &mut Self, b: &Self);
}

fn apply_tup<A,B,C>(tup: (A,B), f: fn(A,B) -> C) -> C {
    f(tup.0,tup.1)
}

impl Merge for Image {
    fn merge(a: &Self, b: &Self) -> Image {
        let mut newimg = a.clone();
        let newclrs: Vec<Color> = image_colors(a)
            .into_iter()
            .zip(image_colors(b).into_iter())
            .map(|(ac, bc)| blend_rgba(ac, bc))
            .collect();
        newimg.update(&newclrs);
        return newimg;
    }

    fn merge_mut(a: &mut Self, b: &Self) {
        if a.width() == 0 && a.height() == 0 {
            a.width = b.width;
            a.height = b.height;
            a.bytes = b.bytes.clone();
        }
        let merged_colors: Vec<u8> = std::iter::zip(image_colors(a), image_colors(b))
            .flat_map(|(ac, bc)| {
                let clr = blend_rgba(ac, bc);
                let bytes: [u8; 4] = clr.into();
                return bytes;
            }).collect();
        a.width = std::cmp::min(a.width, b.width);
        a.height = std::cmp::min(a.height, b.height);
        a.bytes = merged_colors;
    }
}
