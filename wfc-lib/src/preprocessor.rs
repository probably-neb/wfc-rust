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
pub type PatternRef<'p> = Vec<&'p [u8; 4]>;
pub type U8Pattern = Vec<u8>;
type PatternSubImage<'a> = SubImage<&'a ImageBuffer<Rgba<u8>, Vec<u8>>>;
type LocIdHMap = HashMap<UVec2, usize>;
type PatternIdHMap = HashMap<RgbaPattern, usize>;
type Edge = RgbaPattern;
// type IdPatternHMap = HashMap<usize, Pattern>;

/// The data returned by a preprocessor required to run the wfc algorithm
pub struct WfcData {
    pub tile_frequencies: IdMap<usize>,
    pub adjacency_rules: AdjacencyRules,
    pub patterns: IdMap<Pattern>,
}

pub trait PreProcessor {
    fn process(self, image: RgbaImage) -> WfcData;
}

#[derive(Debug, Default, Clone)]
pub struct SimpleTiledPreProcessor {
    pub tile_size: usize,
    /// Map of loc to Tile Id. Mainly for debugging
    loc_id_map: LocIdHMap,
    config: ProcessorConfig,
}

impl SimpleTiledPreProcessor {
    pub fn new(tile_size: usize, config: ProcessorConfig) -> Self {
        return Self {
            tile_size,
            config,
            ..Default::default()
        };
    }
}

impl PreProcessor for SimpleTiledPreProcessor {
    fn process(mut self, image: RgbaImage) -> WfcData {
        let mut patterns: IdMap<PatternRef> = Vec::new();
        let mut adjacency_rules: AdjacencyRules = AdjacencyRules::new();
        let mut tile_frequencies: IdMap<TileId> = IdMap::new();

        let mut current_id = 0;
        let image_dims: UVec2 = image.dimensions().into();

        let locs = get_tile_locs(image_dims, self.tile_size);
        // add capacity for locs to loc_id_map
        self.loc_id_map.reserve(locs.len());
        for loc in locs {
            let pattern: PatternRef = pattern_at(&image, loc, self.tile_size);
            let pattern_id = if let Some(existing_id) = patterns.iter().position(|p| p == &pattern)
            {
                tile_frequencies[existing_id] += 1;
                existing_id
            } else {
                // add new pattern
                let id = current_id;
                current_id += 1;
                patterns.push(pattern);
                tile_frequencies.push(1);
                id
            };

            self.loc_id_map.insert(loc, pattern_id);

            // construct adjacency rules
            if self.config.wrap {
                let max = image_dims - (image_dims % self.tile_size as u32) - UVec2::ONE;
                let on_right_edge = loc.x == max.x;
                let on_top_edge = loc.y == max.y;
                if on_right_edge {
                    let left_loc = UVec2 { x: 0, y: loc.y };
                    let left_id = self.loc_id_map[&left_loc];
                    adjacency_rules.allow(pattern_id, left_id, Right);
                }
                if on_top_edge {
                    let bottom_loc = loc - UVec2 { x: loc.x, y: 0 };
                    let bottom_id = self.loc_id_map[&bottom_loc];
                    adjacency_rules.allow(pattern_id, bottom_id, Down);
                }
            }

            // patterns are processed in column major order so left (<) and below (v) tiles are already extracted
            // add the adjacency rules in these directions if not on an edge
            let on_left_edge = loc.x == 0;
            let on_bottom_edge = loc.y == 0;
            let tsize = self.tile_size as u32;
            if !on_bottom_edge {
                let bottom_loc = loc - UVec2 { x: 0, y: tsize };
                let bottom_id = self
                    .loc_id_map
                    .get(&bottom_loc)
                    .expect("tile below already processed");
                adjacency_rules.allow(pattern_id, *bottom_id, Up);
            }
            if !on_left_edge {
                let left_loc = loc - UVec2 { x: tsize, y: 0 };
                let left_id = self
                    .loc_id_map
                    .get(&left_loc)
                    .expect("tile left already processed");
                adjacency_rules.allow(pattern_id, *left_id, Left);
            }
        }

        let patterns = patterns.into_iter().map(pattern_ref_to_owned).collect();
        return WfcData {
            tile_frequencies,
            adjacency_rules,
            patterns,
        };
    }
}

#[derive(Debug, Clone)]
pub struct WangPreprocessor {
    tile_size: usize,
}

impl WangPreprocessor {
    pub fn new(tile_size: usize) -> Self {
        Self { tile_size }
    }
}

impl PreProcessor for WangPreprocessor {
    fn process(self, image: RgbaImage) -> WfcData {
        let mut vsides: HashSet<Edge> = HashSet::new();
        let mut hsides: HashSet<Edge> = HashSet::new();
        let mut edgemap: IdMap<[Edge; 4]> = IdMap::new();

        let hdirs = [Up, Down];
        let vdirs = [Left, Right];

        let mut patterns: IdMap<PatternRef> = IdMap::new();
        let mut tile_frequencies = IdMap::new();

        let mut current_id = 0;

        let get_edges = |sub_img: &PatternSubImage| -> [Edge; 4] {
            return CardinalDirs::as_array().map(|dir| {
                let ts_u32 = self.tile_size as u32;
                let sub_sub_img = match dir {
                    Up => sub_img.view(0, 0, ts_u32, 1),
                    Left => sub_img.view(0, 0, 1, ts_u32),
                    Right => sub_img.view(ts_u32 - 1, 0, 1, ts_u32),
                    Down => sub_img.view(0, ts_u32 - 1, ts_u32, 1),
                };
                let mut edge: Vec<Rgba<u8>> =
                    sub_sub_img.pixels().map(|(_, _, rgba)| rgba).collect();
                // if self.config.wang_flip {
                // NOTE: assumes wang_flip. This may be an error but every wang tile I've seen so
                // far either has symetric edges or requires a flip
                match dir {
                    Up | Left => (),
                    Down | Right => edge.reverse(),
                }
                // }
                return edge;
            });
        };

        for loc in get_tile_locs(image.dimensions(), self.tile_size) {
            let pattern: PatternRef = pattern_at(&image, loc, self.tile_size);
            // TODO: rework edge functions to use patterns instead of sub-images
            let sub_img = sub_image_at(&image, loc, self.tile_size);

            let id = if let Some(existing_id) = patterns.iter().position(|p| p == &pattern) {
                tile_frequencies[existing_id] += 1;
                existing_id
            } else {
                // add new pattern
                let new_pattern_id = current_id;
                current_id += 1;
                patterns.push(pattern);
                tile_frequencies.push(1);
                new_pattern_id
            };

            let edges = get_edges(&sub_img);
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
        // TODO: check if having more than 2 possible sides breaks things
        assert!(vside_map.len() == 2);
        assert!(hside_map.len() == 2);
        let mut edge_id_map: IdMap<[EdgeId; 4]> = IdMap::new();
        let max_id = patterns.len() - 1;
        for tile_id in 0..=max_id {
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

        let mut adjacency_rules = AdjacencyRules::new();

        for tile_id in 0..=max_id {
            let edges = &edge_id_map[tile_id];
            for other_tile_id in 0..=max_id {
                let other_edges = &edge_id_map[other_tile_id];
                if other_edges[Down] == edges[Up] {
                    adjacency_rules.allow(tile_id, other_tile_id, Up);
                }
                if other_edges[Right] == edges[Left] {
                    adjacency_rules.allow(tile_id, other_tile_id, Left);
                }
            }
        }

        let patterns: IdMap<Pattern> = patterns
            .into_iter()
            .map(|p| pattern_ref_to_owned(p))
            .collect();

        WfcData {
            tile_frequencies,
            adjacency_rules,
            patterns,
        }
    }
}

// TODO: consider creating iterator type for iterating over locs to avoid allocating vec
// unnecessarily
pub fn get_tile_locs<U: Into<UVec2>>(image_dims: U, tile_size: usize) -> Vec<UVec2> {
    let image_dims: UVec2 = image_dims.into();
    // trim edges
    let max = image_dims - (image_dims % tile_size as u32);

    let num_tiles = ((max.x / tile_size as u32) * (max.y / tile_size as u32)) as usize;
    let mut locs = Vec::with_capacity(num_tiles);

    for y in (0..max.y).step_by(tile_size) {
        for x in (0..max.x).step_by(tile_size) {
            locs.push(UVec2 { x, y });
        }
    }

    return locs;
}

fn sub_image_at(
    image: &RgbaImage,
    loc: UVec2,
    tile_size: usize,
) -> SubImage<&ImageBuffer<Rgba<u8>, Vec<u8>>> {
    return image.view(loc.x, loc.y, tile_size as u32, tile_size as u32);
}

pub fn pattern_at(image: &RgbaImage, loc: UVec2, tile_size: usize) -> PatternRef {
    get_tile_locs(UVec2::splat(tile_size as u32), 1)
        .into_iter()
        .map(|l| l + loc)
        .map(|l| &image.get_pixel(l.x, l.y).0)
        .collect()
}

// TODO: consider using this function to get HashMap<UVec2, PatternRef> that can then be used too:
// 1) convert to hashset for easy assigning of ids
// 2) with hashset allocating of tile_frequencies and adjacency_rules can be done with size
// 3) constructing tile_frequencies and adjacency_rules /may/ be made simpler by constructing loc: id map from the hashmap

fn extract_patterns(image: &RgbaImage, tile_size: usize) -> Vec<(UVec2, PatternRef)> {
    return get_tile_locs(image.dimensions(), tile_size)
        .iter()
        .map(|&loc| (loc, pattern_at(&image, loc, tile_size)))
        .collect();
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

fn pattern_ref_to_owned(pref: PatternRef) -> Pattern {
    pref.into_iter().map(|p| p.to_owned()).collect()
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

    fn checker_proc() -> SimpleTiledPreProcessor {
        let tile_size = 4;
        return SimpleTiledPreProcessor::new(tile_size, ProcessorConfig::default());
    }

    #[test]
    fn preprocessor_output_on_checkerboard() {
        let mut proc = checker_proc();
        let image = make_checkerboard(UVec2 { x: 16, y: 16 });
        let WfcData {
            tile_frequencies,
            adjacency_rules: _,
            patterns,
        } = proc.clone().process(image);
        assert_eq!(patterns.len(), 2, "{plen} patterns found for checkerboard", plen=patterns.len());
        let pattern_ids: HashMap<Pattern, TileId> = patterns
            .iter()
            .enumerate()
            .map(|(i, p)| (p.to_owned(), i))
            .collect();
        // tile ids
        assert_eq!(pattern_ids.len(), 2);
        let mut ids: Vec<&usize> = pattern_ids.values().collect();
        ids.sort();
        assert_eq!(ids[0], &0);
        assert_eq!(ids[1], &1);

        // tile_freqs
        assert_eq!(tile_frequencies.iter().sum::<usize>(), 16);
        assert_eq!(tile_frequencies[0], tile_frequencies[1]);
    }

    #[test]
    fn generate_checkerboard_adj_rules() {
        let mut proc = checker_proc();
        let image = make_checkerboard(UVec2 { x: 16, y: 16 });
        let data = proc.process(image);
        let adj_rules = data.adjacency_rules;
        // for i in 0..4 {
        //     for j in 0..3 {
        //         print!("{}", proc.image[(i * 4, j * 4)][0] / 255);
        //     }
        //     println!("{}", proc.image[(i * 4, 3 * 4)][0] / 255);
        // }
        // image.save("./checker.png").unwrap();
        println!("{adj_rules:?}");
        assert_eq!(adj_rules.len(), 2, "more than two tiles in adjacency_rules");
        assert!(adj_rules.allowed_in_all_dirs(0, 1));
        assert!(adj_rules.allowed_in_all_dirs(1, 0));
    }
    // TODO: test: get_tile_locs on empty image should return empty vec
    #[test]
    fn pattern_at_returns_correctly_oriented_image() {
        let tile_size = 16;
        let random_image = RgbaImage::from_fn(tile_size, tile_size, |x, y| {
            let x = x as u8;
            let y = y as u8;
            Rgba([x, x, y, y])
        });
        let pattern = pattern_at(&random_image, UVec2::ZERO, tile_size as usize);
        let image_pixels: PatternRef = random_image.pixels().map(|p| &p.0).collect();
        assert!(pattern.len() == image_pixels.len(), "images have different sizes: pattern={plen} image={idims:?}={ilen}", plen=pattern.len(), idims=random_image.dimensions(), ilen=image_pixels.len());
        for x in 0..tile_size {
            for y in 0..tile_size {
                let i = (y * tile_size + x) as usize;
                assert!(*pattern[i] == *image_pixels[i], "pixel at {x},{y} does not match");
            }
        }
    }
}
