use glam::UVec2;
use image::{GenericImageView, ImageBuffer, Rgba, RgbaImage, SubImage};
use std::{
    collections::{HashMap, HashSet},
    vec::Vec,
};

use crate::{
    adjacency_rules::{
        AdjacencyRules,
        CardinalDirs::{self, Down, Left, Right, Up},
    },
    tile::{IdMap, TileId},
};

/// The actual pixel data of the tile_size x tile_size rectangle (PatternRect)
/// corresponding to a tile in the source image
pub type RgbaPattern = Vec<Rgba<u8>>;
pub type Pattern = Vec<[u8; 4]>;
pub type U8Pattern = Vec<u8>;
type PatternSubImage<'a> = SubImage<&'a ImageBuffer<Rgba<u8>, Vec<u8>>>;
type LocIdHMap = HashMap<UVec2, usize>;
type PatternIdHMap = HashMap<RgbaPattern, usize>;
type Edge = RgbaPattern;
// type IdPatternHMap = HashMap<usize, Pattern>;

#[derive(Debug, Default)]
pub struct PreProcessor {
    pub image: RgbaImage,
    pub tile_size: usize,
    /// tiles_x , tiles_y
    pub dims: UVec2,
    /// Map of loc to Tile Id. Mainly for debugging
    pub tile_loc_map: LocIdHMap,
    /// the top left corner of the tile_size x tile_size pattern
    /// in the source image corresponding to each unique tile
    pub tiles: IdMap<UVec2>,
    pattern_ids: PatternIdHMap,
    config: ProcessorConfig,
    num_unique_tiles: usize,
    adjacency_rules: AdjacencyRules,
    tile_frequencies: IdMap<usize>,
}

impl PreProcessor {
    pub fn new(image: &RgbaImage, tile_size: usize, config: ProcessorConfig) -> Self {
        let dims: UVec2 = image.dimensions().into();
        return Self {
            image: image.clone(),
            tile_size,
            dims,
            config,
            ..Default::default()
        };
    }
    pub fn num_tiles(&self) -> usize {
        return self.num_unique_tiles;
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

    fn next_tile_id(&mut self) -> usize {
        let id = self.num_unique_tiles;
        self.num_unique_tiles += 1;
        return id;
    }

    fn image_at(&self, loc: UVec2) -> SubImage<&ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let ts_u32 = self.tile_size as u32;
        return self.image.view(loc.x, loc.y, ts_u32, ts_u32);
    }

    pub fn rgba_arr_pattern_at(&self, loc: UVec2) -> Pattern {
        let pixels = self
            .image_at(loc)
            .pixels()
            .map(|(_, _, rgba)| rgba.0)
            .collect();
        return pixels;
    }

    pub fn rgba_pattern_at(&self, loc: UVec2) -> RgbaPattern {
        let pixels = self
            .image_at(loc)
            .pixels()
            .map(|(_, _, rgba)| rgba)
            .collect();
        return pixels;
    }

    pub fn pattern_at(&self, loc: UVec2) -> U8Pattern {
        let pixels = self
            .image_at(loc)
            .pixels()
            .flat_map(|(_, _, rgba)| rgba.0)
            .collect();
        return pixels;
    }

    fn fill_tile_idmap(&mut self) {
        // fill self.tiles
        self.tiles = vec![UVec2::default(); self.num_tiles()];
        for (&loc, &id) in &self.tile_loc_map {
            self.tiles[id] = loc;
        }
    }

    fn add_adjacency_rules_from_previously_parsed_tiles(&mut self, loc: UVec2, id: TileId) {
        // locs are in column major order so left (<) and below (v) tiles are already extracted
        // add the adjacency rules in these directions if not on an edge
        let on_left_edge = loc.x == 0;
        let on_bottom_edge = loc.y == 0;
        let tsize = self.tile_size as u32;
        if !on_left_edge {
            let left_loc = loc - UVec2 { x: tsize, y: 0 };
            let left_id = self.tile_loc_map[&left_loc];
            self.adjacency_rules.allow(id, left_id, Left);
        }
        if !on_bottom_edge {
            let bottom_loc = loc - UVec2 { x: 0, y: tsize };
            let bottom_id = self.tile_loc_map[&bottom_loc];
            self.adjacency_rules.allow(id, bottom_id, Up);
        }
    }

    /// If pattern at loc has not been seen before adds the pattern
    /// to the list of patterns, assigns it an id, and sets it's frequency to 1
    /// if the pattern has been seen before just increments its frequency
    /// returns the (possibly new) tile id
    fn process_tile(&mut self, loc: UVec2) -> TileId {
        let pattern = self.rgba_pattern_at(loc);
        let id = match self.pattern_ids.get(&pattern) {
            // new pattern
            None => {
                let new_id = self.next_tile_id();
                self.pattern_ids.insert(pattern.clone(), new_id);
                self.tile_frequencies.push(1);
                new_id
            }
            // old pattern
            Some(&id) => {
                self.tile_frequencies[id] += 1;
                id
            }
        };
        self.tile_loc_map.insert(loc, id);
        return id;
    }

    fn get_pattern_idvec(&self) -> Vec<Pattern> {
        return self
            .tiles
            .iter()
            .map(|&loc| self.rgba_arr_pattern_at(loc))
            .collect();
    }

    fn create_wfcdata(&self) -> WfcData {
        let patterns = self.get_pattern_idvec();
        let tile_frequencies = self.tile_frequencies.clone();
        let adjacency_rules = self.adjacency_rules.clone();
        return WfcData {
            tile_frequencies,
            adjacency_rules,
            patterns,
        };
    }

    pub fn process_simple_tile(&mut self) -> WfcData {
        // iter over tile locations and store tile pixels
        // and keep track of unique tiles
        let locs = self.tile_locs();
        for loc in locs {
            let id = self.process_tile(loc);

            // construct adjacency rules
            if self.config.wrap {
                let max = self.dims - (self.dims % self.tile_size as u32) - UVec2::ONE;
                let on_right_edge = loc.x == max.x;
                let on_top_edge = loc.y == max.y;
                if on_right_edge {
                    let left_loc = UVec2 { x: 0, y: loc.y };
                    let left_id = self.tile_loc_map[&left_loc];
                    self.adjacency_rules.allow(id, left_id, Right);
                }
                if on_top_edge {
                    let bottom_loc = loc - UVec2 { x: loc.x, y: 0 };
                    let bottom_id = self.tile_loc_map[&bottom_loc];
                    self.adjacency_rules.allow(id, bottom_id, Down);
                }
            }
            self.add_adjacency_rules_from_previously_parsed_tiles(loc, id);
        }

        self.fill_tile_idmap();

        assert!(!self.pattern_ids.is_empty());
        assert!(!self.tiles.is_empty());

        return self.create_wfcdata();
    }

    fn get_edge(&self, sub_img: &PatternSubImage, side: CardinalDirs) -> Edge {
        let ts_u32 = self.tile_size as u32;
        let sub_sub_img = match side {
            Up => sub_img.view(0, 0, ts_u32, 1),
            Left => sub_img.view(0, 0, 1, ts_u32),
            Right => sub_img.view(ts_u32 - 1, 0, 1, ts_u32),
            Down => sub_img.view(0, ts_u32 - 1, ts_u32, 1),
        };
        // sub_sub_img.to_image().save(format!("./edges/{side:?}_{:?}.png",sub_img.bounds())).unwrap();
        let mut edge: Vec<Rgba<u8>> = sub_sub_img.pixels().map(|(_, _, rgba)| rgba).collect();
        if self.config.wang_flip {
            match side {
                Up | Left => (),
                Down | Right => edge.reverse()
            }
        }
        return edge;
    }

    fn get_edges(&self, sub_img: &PatternSubImage) -> [Edge; 4] {
        return CardinalDirs::as_array().map(|dir| self.get_edge(sub_img, dir));
    }

    pub fn process_wang(&mut self) -> WfcData {
        let mut vsides: HashSet<Edge> = HashSet::new();
        let mut hsides: HashSet<Edge> = HashSet::new();
        let mut edgemap: IdMap<[Edge; 4]> = IdMap::new();
        let hdirs = [Up, Down];
        let vdirs = [Left, Right];
        for loc in self.tile_locs() {
            let id = self.process_tile(loc);
            let sub_img = self.image_at(loc);
            let edges = self.get_edges(&sub_img);
            for vdir in vdirs {
                vsides.insert(edges[vdir].clone());
            }
            for hdir in hdirs {
                hsides.insert(edges[hdir].clone());
            }
            // old id
            if id < edgemap.len() {
                // do nothing
            }
            // new id
            else if id == edgemap.len() {
                edgemap.push(edges);
            } else {
                unreachable!("id was incremented twice");
            }
        }
        type EdgeId = usize;
        let vside_map: HashMap<&Edge, EdgeId> = vsides
            .iter()
            .enumerate()
            .map(|(edge_id, edge)| (edge, edge_id))
            .collect();
        let hside_map: HashMap<&Edge, EdgeId> = hsides
            .iter()
            .enumerate()
            .map(|(edge_id, edge)| (edge, edge_id))
            .collect();
        assert!(vside_map.len() == 2);
        assert!(hside_map.len() == 2);
        let mut edge_id_map: IdMap<[EdgeId; 4]> = IdMap::new();
        for tile_id in self.tile_ids() {
            let mut edge_ids: [EdgeId; 4] = Default::default();
            let edges = &edgemap[tile_id];
            for vdir in vdirs {
                let edge = &edges[vdir];
                let edge_id = vside_map.get(edge).unwrap();
                edge_ids[vdir] = *edge_id;
            }
            for hdir in hdirs {
                let edge = &edges[hdir];
                let edge_id = hside_map.get(edge).unwrap();
                edge_ids[hdir] = *edge_id;
            }
            edge_id_map.push(edge_ids);
        }
        dbg!(&edge_id_map);

        for tile_id in self.tile_ids() {
            let edges = &edge_id_map[tile_id];
            for other_tile_id in self.tile_ids() {
                let other_edges = &edge_id_map[other_tile_id];
                // for dir in CardinalDirs::as_array() {
                //     let edge_id = edges[dir];
                //     if other_edges[-dir] == edge_id {
                //         log::info!("allowing {tile_id} -> {dir:?} -> {other_tile_id}");
                //         self.adjacency_rules.allow(tile_id, other_tile_id, dir);
                //     }
                // }
                if other_edges[Down] == edges[Up] {
                    self.adjacency_rules.allow(tile_id, other_tile_id, Up);
                }
                if other_edges[Right] == edges[Left] {
                    self.adjacency_rules.allow(tile_id, other_tile_id, Left);
                }
            }
        }

        self.fill_tile_idmap();

        return dbg!(self.create_wfcdata());
    }

    pub fn process(&mut self) -> WfcData {
        if self.config.wang {
            return self.process_wang();
        } else {
            return self.process_simple_tile();
        }
    }
}

// #[derive(Debug)]
pub struct WfcData {
    pub tile_frequencies: IdMap<usize>,
    pub adjacency_rules: AdjacencyRules,
    pub patterns: IdMap<Pattern>,
}

use std::fmt::Debug;
impl Debug for WfcData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WfcData")
            .field("tile_frequencies", &self.tile_frequencies)
            .field("adjacency_rules", &self.adjacency_rules)
            .field("patterns (len)", &self.patterns.len())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    pub wrap: bool,
    pub wang: bool,
    pub wang_flip: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            wrap: false,
            wang: false,
            wang_flip: false,
        }
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
        return PreProcessor::new(&image, tile_size, ProcessorConfig::default());
    }

    #[test]
    fn extract_tiles() {
        let mut proc = checker_proc();
        let WfcData {
            tile_frequencies,
            adjacency_rules: _,
            patterns: _,
        } = proc.process();
        let pattern_ids = proc.pattern_ids;
        let tiles = proc.tile_loc_map;
        // tile ids
        assert_eq!(pattern_ids.len(), 2);
        let mut ids: Vec<&usize> = pattern_ids.values().collect();
        ids.sort();
        assert_eq!(ids[0], &0);
        assert_eq!(ids[1], &1);

        // tile_freqs
        assert_eq!(tile_frequencies.iter().sum::<usize>(), 16);
        assert_eq!(tile_frequencies[0], tile_frequencies[1]);

        // tiles
        assert_eq!(tiles.len(), 16);
        // max id
        assert_eq!(*tiles.values().max().unwrap(), tile_frequencies.len() - 1);
    }

    #[test]
    fn generate_adj_rules() {
        let mut proc = checker_proc();
        let data = proc.process();
        let adj_rules = data.adjacency_rules;
        for i in 0..4 {
            for j in 0..3 {
                print!("{}", proc.image[(i * 4, j * 4)][0] / 255);
            }
            println!("{}", proc.image[(i * 4, 3 * 4)][0] / 255);
        }
        proc.image.save("./checker.png").unwrap();
        println!("{adj_rules:?}");
        assert_eq!(adj_rules.len(), 2);
        assert!(adj_rules.allowed_in_all_dirs(0, 1));
        assert!(adj_rules.allowed_in_all_dirs(1, 0));
    }
}
