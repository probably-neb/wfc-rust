use glam::UVec2;
use image::{Rgba, RgbaImage};
use std::collections::HashMap;

use crate::{adjacency_rules::{
    AdjacencyRules,
    CardinalDirs::{Down, Left},
}, IdMap};

/// The actual pixel data of the tile_size x tile_size rectangle (PatternRect)
/// corresponding to a tile in the source image
pub type Pattern = Vec<Rgba<u8>>;
pub type RgbaPattern = Vec<u8>;
type LocIdHMap = HashMap<UVec2, usize>;
type PatternIdHMap = HashMap<RgbaPattern, usize>;
// type IdPatternHMap = HashMap<usize, Pattern>;

#[derive(Debug, Default)]
pub struct PreProcessor {
    pub image: RgbaImage,
    pub tile_size: usize,
    /// tiles_x , tiles_y
    pub dims: UVec2,
    /// Map of loc to Tile Id. Mainly for debugging
    pub tile_loc_map: LocIdHMap,
    /// the bottom left corner of the tile_size x tile_size pattern
    /// in the source image corresponding to each unique tile
    pub tiles: IdMap<UVec2>,
    pattern_ids: PatternIdHMap,
}

impl PreProcessor {
    pub fn new(image: RgbaImage, tile_size: usize) -> Self {
        let dims: UVec2 = image.dimensions().into();
        return Self { image, tile_size, dims,..Default::default()};
    }
    pub fn num_tiles(&self) -> usize {
        return self.tiles.len();
    }
    pub fn tile_ids(&self) -> impl Iterator<Item = usize> {
        return 0..(self.num_tiles());
    }
    pub fn tile_locs(&self) -> Vec<UVec2> {
        let mut locs = Vec::new();

        // trim edges
        let max = self.dims - (self.dims % self.tile_size as u32);

        for x in (0..max.x).step_by(self.tile_size) {
            for y in (0..max.y).step_by(self.tile_size) {
                locs.push(UVec2 { x, y });
            }
        }

        return locs;
    }

    pub fn rgba_pattern_at(&self, loc: UVec2) -> RgbaPattern {
        let mut pixels = Vec::with_capacity(self.tile_size * self.tile_size * 4);
        let (min_x, min_y) = loc.into();
        let (max_x, max_y) = (loc + self.tile_size as u32).into();
        for x in min_x..max_x {
            for y in min_y..max_y {
                // pixels.push(self.image[(x, y)].into());
                let mut rgba: Vec<u8> = self.image[(x,y)].0.to_owned().to_vec();
                pixels.append(&mut rgba);
            }
        }
        return pixels;
    }

    pub fn pattern_at(&self, loc: UVec2) -> Pattern {
        let mut pixels = Vec::with_capacity(self.tile_size * self.tile_size);
        let (min_x, min_y) = loc.into();
        let (max_x, max_y) = (loc + self.tile_size as u32).into();
        for x in min_x..max_x {
            for y in min_y..max_y {
                pixels.push(self.image[(x, y)]);
                // let mut rgba: Vec<u8> = self.image[(x,y)].0.to_owned().to_vec();
                // pixels.append(&mut rgba);
            }
        }
        return pixels;
    }

    fn store_pattern_locs(&mut self) {
        self.tiles = vec![UVec2::default(); self.pattern_ids.len()];
        // unsorted vec of unique (id,pattern) entries
        for (&loc, &id) in &self.tile_loc_map {
            self.tiles[id] = loc;
        }
    }
    // fn hash_tiles(&self, locs: Vec<UVec2>) -> HashMap<UVec2,

    pub fn process(&mut self) -> (IdMap<usize>, AdjacencyRules) {
        // incremented to assign each tile a unique id
        let mut num_unique_tiles = 0;
        // a map from tile pixels to it's tile id let mut tile_freqs: Vec<usize> = Vec::new();
        let mut adj_rules = AdjacencyRules::new();
        let mut tile_freqs: IdMap<usize> = IdMap::new();

        // iter over tile locations and store tile pixels
        // and keep track of unique tiles
        let locs = self.tile_locs();
        for loc in locs {
            let pattern = self.rgba_pattern_at(loc);
            let new_tile: bool = !self.pattern_ids.contains_key(&pattern);
            if new_tile {
                self.pattern_ids.insert(pattern.clone(), num_unique_tiles);
                num_unique_tiles += 1;
            }
            if !new_tile {
                tile_freqs[self.pattern_ids[&pattern]] += 1;
            } else {
                tile_freqs.push(1);
            }
            let id = self.pattern_ids[&pattern];
            self.tile_loc_map.insert(loc, id);

            // construct adjacency rules
            // locs are in column major order so left (<) and below (v) tiles are already extracted
            // add the adjacency rules in these directions if not on an edge
            let on_left_edge = loc.x == 0;
            let on_bottom_edge = loc.y == 0;
            let tsize = self.tile_size as u32;
            if !on_left_edge {
                let left_loc = loc - UVec2 { x: tsize, y: 0 };
                let left_id = self.tile_loc_map[&left_loc];
                // and reverse
                adj_rules.allow_bi(id, left_id, Left);
            }
            if !on_bottom_edge {
                let bottom_loc = loc - UVec2 { x: 0, y: tsize };
                let bottom_id = self.tile_loc_map[&bottom_loc];
                // and reverse
                adj_rules.allow_bi(id, bottom_id, Down);
            }
        }
        self.store_pattern_locs();
        assert!(!self.pattern_ids.is_empty());
        assert!(!self.tiles.is_empty());
        return (tile_freqs, adj_rules);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn make_checkerboard(dims: UVec2) -> RgbaImage {
        let white = [255, 255, 255, 255];
        let black = [0, 0, 0, 255];
        let bside = black.repeat(4);
        let wside = white.repeat(4);
        let mut row = Vec::new();
        let mut worb: bool = false;
        for _w in (0..dims.x).step_by(4) {
            let mut color = if worb { wside.clone() } else { bside.clone() };
            worb = !worb;
            row.append(&mut color);
        }
        let mut odd_row = row.clone();
        odd_row.rotate_left(16);
        let mut color = white;
        let mut odd_color = black;
        if !worb {
            color = black;
            odd_color = white;
        }
        row.append(&mut color.repeat((dims.x % 4) as usize));
        odd_row.append(&mut odd_color.repeat((dims.x % 4) as usize));
        let mut img_vec = Vec::new();
        // TODO: fill remaining rows if dims.y % 8 != 0
        for _h in 0..(dims.y / 8) {
            img_vec.append(&mut row.clone().repeat(4));
            img_vec.append(&mut odd_row.clone().repeat(4));
        }
        return RgbaImage::from_vec(dims.x, dims.y, img_vec)
            .expect("size of make_checkerboard correct");
    }

    fn checker_proc() -> PreProcessor {
        let image = make_checkerboard(UVec2 { x: 16, y: 16 });
        let tile_size = 4;
        return PreProcessor {
            image,
            tile_size,
            ..Default::default()
        };
    }

    #[test]
    fn extract_tiles() {
        let mut proc = checker_proc();
        let (tile_freqs, _adj_rules) = proc.process();
        let pattern_ids = proc.pattern_ids;
        let tiles = proc.tile_loc_map;
        // tile ids
        assert_eq!(pattern_ids.len(), 2);
        let mut ids: Vec<&usize> = pattern_ids.values().collect();
        ids.sort();
        assert_eq!(ids[0], &0);
        assert_eq!(ids[1], &1);

        // tile_freqs
        assert_eq!(tile_freqs.iter().sum::<usize>(), 16);
        assert_eq!(tile_freqs[0], tile_freqs[1]);

        // tiles
        assert_eq!(tiles.len(), 16);
        // max id
        assert_eq!(*tiles.values().max().unwrap(), tile_freqs.len() - 1);
    }

    #[test]
    fn generate_adj_rules() {
        let mut proc = checker_proc();
        let (_tile_freqs, adj_rules) = proc.process();
        for i in 0..4 {
            for j in 0..3 {
                print!("{}", proc.image[(i * 4, j * 4)][0] / 255);
            }
            println!("{}", proc.image[(i * 4, 3 * 4)][0] / 255);
        }
        proc.image.save("./checker.png").unwrap();
        // println!("{:?}", proc.image.pixels().step_by(4).map(|p| p[0]/255).collect::<Vec<u8>>());
        println!("{adj_rules:?}");
        assert_eq!(adj_rules.len(), 2);
        assert!(adj_rules.allowed_in_all_dirs(0, 1));
        assert!(adj_rules.allowed_in_all_dirs(1, 0));
    }
}
