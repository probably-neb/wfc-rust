pub mod adjacency_rules;
pub mod preprocessor;
pub mod tile;
pub mod wfc;
pub mod utils;

use derive_more::{Deref, DerefMut, From};
use glam::UVec2;
use image::RgbaImage;
use preprocessor::{Config, WfcData};
use std::fmt::Debug;

const TILE_SIZE_DEFAULT: usize = 2;
const PIXEL_SCALE_DEFAULT: u32 = 2;

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
// pub mod simple_patterns {
//     use super::*;
//     use adjacency_rules::CardinalDirs;
//     use CardinalDirs::*;
//     pub const CHARS: [&str; 5] = ["' '", "┓", "┛", "┏", "┗"];
//
//     const PRINT_CREATION: bool = false;
//
//     fn allow_all(
//         aaa: [usize; 2],
//         bbb: [usize; 2],
//         dir: CardinalDirs,
//         adjacency_rules: &mut AdjacencyRules,
//     ) {
//         for a in aaa {
//             for b in bbb {
//                 adjacency_rules.allow(a, b, dir);
//                 if PRINT_CREATION {
//                     let ac = CHARS[a];
//                     let bc = CHARS[b];
//                     println!("Allowing:");
//                     match dir {
//                         Up => {
//                             println!("{}", bc);
//                             println!("{}", ac);
//                         }
//                         Down => {
//                             println!("{}", ac);
//                             println!("{}", bc);
//                         }
//                         Left => {
//                             println!("{}{}", bc, ac)
//                         }
//                         Right => {
//                             println!("{}{}", ac, bc)
//                         }
//                     }
//                 }
//             }
//         }
//     }
//
//     pub const BLANK: usize = 0; //' '
//     pub const DL: usize = 1; // ┓
//     pub const LU: usize = 2; // ┛
//     pub const RD: usize = 3; // ┏
//     pub const UR: usize = 4; // ┗
//
//     // ┓ ┛
//     pub const BLANK_RIGHT: [usize; 2] = [DL, LU];
//     // ┏ ┗
//     pub const BLANK_LEFT: [usize; 2] = [RD, UR];
//     // ┏ ┓
//     pub const BLANK_UP: [usize; 2] = [RD, DL];
//     // ┗ ┛
//     pub const BLANK_DOWN: [usize; 2] = [UR, LU];
//
//     pub const B2: [usize; 2] = [BLANK, BLANK];
//
//     pub fn construct_simple_patterns() -> Wfc {
//         let mut adjacency_rules = AdjacencyRules::new();
//         let paths: IdMap<String> = vec!["blank", "dl", "lu", "rd", "ur"]
//             .iter()
//             .map(|&name| format!("./inputs/simple/{}.png", name))
//             .collect();
//         let tile_frequencies: IdMap<usize> = vec![1, 2, 2, 2, 2];
//
//         // matching blank top / bottom
//         allow_all(BLANK_UP, BLANK_DOWN, Up, &mut adjacency_rules);
//
//         // connecting arm top / bottom
//         allow_all(BLANK_DOWN, BLANK_UP, Up, &mut adjacency_rules);
//
//         // matching blank left / right
//         allow_all(BLANK_RIGHT, BLANK_LEFT, Right, &mut adjacency_rules);
//
//         // connecting arm left / right
//         allow_all(BLANK_LEFT, BLANK_RIGHT, Right, &mut adjacency_rules);
//
//         allow_all(B2, BLANK_LEFT, Right, &mut adjacency_rules);
//         allow_all(B2, BLANK_RIGHT, Left, &mut adjacency_rules);
//         allow_all(B2, BLANK_UP, Down, &mut adjacency_rules);
//         allow_all(B2, BLANK_DOWN, Up, &mut adjacency_rules);
//         for &dir in CardinalDirs::iter() {
//             adjacency_rules.allow(BLANK, BLANK, dir);
//         }
//
//         return Wfc::new_from_pattern_paths(paths, adjacency_rules, tile_frequencies)
//             .with_tile_size(4);
//     }
// }
