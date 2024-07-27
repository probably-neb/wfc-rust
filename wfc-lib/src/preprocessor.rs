use glam::UVec2;
use image::{GenericImageView, ImageBuffer, Rgba, RgbaImage, SubImage};
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    vec::Vec,
};

use crate::{
    adjacency_rules::{
        AdjacencyRules,
        CardinalDirs::{self, Down, Left, Right, Up},
    },
    tile::{IdMap, TileId},
    UVec2Iter,
};

/// The actual pixel data of the tile_size x tile_size rectangle (PatternRect)
/// corresponding to a tile in the source image
pub type RgbaPattern = Vec<Rgba<u8>>;
pub type Pattern = Vec<[u8; 4]>;
pub type PatternRef<'p> = Vec<&'p [u8; 4]>;
pub type U8Pattern = Vec<u8>;
type PatternSubImage<'a> = SubImage<&'a ImageBuffer<Rgba<u8>, Vec<u8>>>;
type LocIdHMap = HashMap<UVec2, usize>;
type Edge = RgbaPattern;
type EdgeId = usize;
// type IdPatternHMap = HashMap<usize, Pattern>;

/// The data returned by a preprocessor required to run the wfc algorithm
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

#[derive(Debug, Clone, Copy)]
#[cfg_attr(
    feature = "web",
    derive(serde::Deserialize, tsify::Tsify),
    serde(rename_all = "lowercase")
)]
pub enum EdgeMethod {
    // Edges that match are considered compatible
    Perfect,
    // Edges that are adjacent are considered compatible
    Adjacent,
    // TODO: find out if flip is just a subset of adjacent
    Flip,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(
    feature = "web",
    derive(serde::Deserialize, tsify::Tsify),
    serde(rename_all = "lowercase")
)]
pub enum AdjacencyMethod {
    Adjacency,
    Edge(EdgeMethod),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(
    feature = "web",
    derive(serde::Deserialize, tsify::Tsify),
    serde(rename_all = "lowercase")
)]
pub enum PatternMethod {
    Overlapping,
    Tiled,
}

#[cfg(feature = "web")]
#[derive(serde::Deserialize, tsify::Tsify, Copy, Clone, Debug)]
#[serde(rename = "UVec2", remote = "UVec2")]
/// Clone of UVec2 for deserializing
pub struct WrappedUVec2 {
    x: u32,
    y: u32,
}
#[cfg(feature = "web")]
impl Into<UVec2> for WrappedUVec2 {
    fn into(self) -> UVec2 {
        return UVec2::new(self.x, self.y);
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(
    feature = "web",
    derive(serde::Deserialize, tsify::Tsify),
    serde(rename_all = "lowercase")
)]
pub struct Config {
    #[cfg_attr(feature = "web", serde(with = "WrappedUVec2"))]
    pub tile_size: UVec2,
    pub adjacency_method: AdjacencyMethod,
    pub pattern_method: PatternMethod,
}

fn preprocess_simple_tiled(image: RgbaImage, config: Config) -> WfcData {
    let tile_size = config.tile_size;
    let mut patterns: IdMap<PatternRef> = Vec::new();
    let mut adjacency_rules: AdjacencyRules = AdjacencyRules::new();
    let mut tile_frequencies: IdMap<TileId> = IdMap::new();

    let mut current_id = 0;
    let image_dims: UVec2 = image.dimensions().into();
    let mut loc_id_map = crate::utils::UVecVec(vec![
        vec![None; image_dims.x as usize];
        image_dims.y as usize
    ]);

    let locs = get_tile_locs(image_dims, tile_size);
    for loc in locs {
        let pattern: PatternRef = pattern_at(&image, loc, tile_size);
        let pattern_id = if let Some(existing_id) = patterns.iter().position(|p| p == &pattern) {
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
        loc_id_map[loc] = Some(pattern_id);

        // construct adjacency rules
        // if self.config.wrap {
        //     let max = image_dims - (image_dims % tile_size as u32) - UVec2::ONE;
        //     let on_right_edge = loc.x == max.x;
        //     let on_top_edge = loc.y == max.y;
        //     if on_right_edge {
        //         let left_loc = UVec2 { x: 0, y: loc.y };
        //         let left_id = self.loc_id_map[&left_loc];
        //         adjacency_rules.allow(pattern_id, left_id, Right);
        //     }
        //     if on_top_edge {
        //         let bottom_loc = loc - UVec2 { x: loc.x, y: 0 };
        //         let bottom_id = self.loc_id_map[&bottom_loc];
        //         adjacency_rules.allow(pattern_id, bottom_id, Down);
        //     }
        // }

        // patterns are processed in column major order so left (<) and below (v) tiles are already extracted
        // add the adjacency rules in these directions if not on an edge
        let on_left_edge = loc.x == 0;
        let on_bottom_edge = loc.y == 0;
        if !on_bottom_edge {
            let bottom_loc = loc
                - UVec2 {
                    x: 0,
                    y: tile_size.y,
                };
            let bottom_id = loc_id_map[bottom_loc].expect("tile below already processed");
            adjacency_rules.allow(pattern_id, bottom_id, Up);
        }
        if !on_left_edge {
            let left_loc = loc
                - UVec2 {
                    x: tile_size.x,
                    y: 0,
                };
            let left_id = loc_id_map[left_loc].expect("tile left already processed");
            adjacency_rules.allow(pattern_id, left_id, Left);
        }
    }

    let patterns = patterns.into_iter().map(pattern_ref_to_owned).collect();
    return WfcData {
        tile_frequencies,
        adjacency_rules,
        patterns,
    };
}

fn preprocess_edges(image: RgbaImage, config: Config) -> WfcData {
    let tile_size = config.tile_size;
    let mut vsides: HashSet<Edge> = HashSet::new();
    let mut hsides: HashSet<Edge> = HashSet::new();
    let mut edgemap: IdMap<[Edge; 4]> = IdMap::new();

    let hdirs = [Up, Down];
    let vdirs = [Left, Right];

    let mut patterns: IdMap<PatternRef> = IdMap::new();
    let mut tile_frequencies = IdMap::new();

    let mut current_id = 0;

    for loc in get_tile_locs(image.dimensions().into(), tile_size) {
        let pattern: PatternRef = pattern_at(&image, loc, tile_size);
        // TODO: rework edge functions to use patterns instead of sub-images
        let sub_img = sub_image_at(&image, loc, tile_size);

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

        let mut edges = get_edges(&sub_img, tile_size);

        if let AdjacencyMethod::Edge(EdgeMethod::Flip) = config.adjacency_method {
            edges[Down].reverse();
            edges[Right].reverse();
        }

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
    // assert!(vside_map.len() == 2);
    // assert!(hside_map.len() == 2);
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

    let patterns: IdMap<Pattern> = patterns.into_iter().map(pattern_ref_to_owned).collect();

    WfcData {
        tile_frequencies,
        adjacency_rules,
        patterns,
    }
}

fn preprocess_adjacent_edges(image: RgbaImage, config: Config) -> WfcData {
    let tile_size = config.tile_size;
    let mut patterns: HashMap<Pattern, (TileId, usize)> = HashMap::new();
    let mut edges: HashMap<Edge, EdgeId> = HashMap::new();
    let image_dims: UVec2 = image.dimensions().into();
    let mut loc_id_map = crate::utils::UVecVec(vec![
        vec![None; image_dims.x as usize];
        image_dims.y as usize
    ]);
    let mut edge_adjacencies = AdjacencyRules::new();

    // map of [pattern_id][side] => edge_id
    let mut pattern_edge_ids = IdMap::new();
    // map of [edge_id][side] => pattern_id[] (patterns that had [edge_id] on [side])
    let mut edge_pattern_map: IdMap<[HashSet<TileId>; 4]> = IdMap::new();

    for loc in get_tile_locs(image_dims, tile_size) {
        let pattern_img = sub_image_at(&image, loc, tile_size);
        let pattern: Pattern = pattern_img.pixels().map(|(_, _, rgba)| rgba.0).collect();
        let (pattern_id, edge_ids) = if let Some((existing_id, count)) = patterns.get_mut(&pattern)
        {
            // existing pattern
            let id = *existing_id;
            *count += 1;
            (id, pattern_edge_ids[id])
        } else {
            // new pattern
            let pattern_id = patterns.len();
            patterns.insert(pattern, (pattern_id, 1));
            let pattern_edges = get_edges(&pattern_img, tile_size);
            let edge_ids = pattern_edges.map(|edge| {
                let edge_id = if let Some(&edge_id) = edges.get(&edge) {
                    edge_id
                } else {
                    // new edge
                    let edge_id = edges.len();
                    edges.insert(edge, edge_id);
                    edge_pattern_map.push(Default::default());
                    edge_id
                };
                edge_id
            });

            // PERF: unroll
            for side in 0..4 {
                edge_pattern_map[edge_ids[side]][side].insert(pattern_id);
            }
            pattern_edge_ids.push(edge_ids);
            (pattern_id, edge_ids)
        };

        loc_id_map[loc] = Some(edge_ids);

        let on_left_edge = loc.x == 0;
        let on_bottom_edge = loc.y == 0;
        if !on_bottom_edge {
            // bottom edge of the current pattern
            let bottom_edge_id = pattern_edge_ids[pattern_id][Down];
            let bottom_loc = loc - UVec2::new(0, tile_size.y);
            let tile_below_top_edge_id =
                loc_id_map[bottom_loc].expect("tile below already processed")[Up];
            edge_adjacencies.allow(bottom_edge_id, tile_below_top_edge_id, Down);
        }
        if !on_left_edge {
            let left_edge_id = pattern_edge_ids[pattern_id][Left];
            let left_loc = loc - UVec2::new(tile_size.x, 0);
            let tile_left_right_edge_id =
                loc_id_map[left_loc].expect("tile left already processed")[Right];
            edge_adjacencies.allow(left_edge_id, tile_left_right_edge_id, Left);
        }
    }
    #[cfg(test)]
    assert_eq!(edges.len(),2);

    println!(
        "edge adjs: {:?} len={}",
        edge_adjacencies,
        edge_adjacencies.len()
    );
    println!(
        "edge pmap: {:?} len={}",
        edge_pattern_map,
        edge_pattern_map.len()
    );
    let mut adjacency_rules = AdjacencyRules::new();
    for pattern_id in 0..patterns.len() {
        // add left and bottom edges only because allowing a -> b also allows a <- b
        let left_id = pattern_edge_ids[pattern_id][Left];
        let adjacent_edges =
            if let Some(adjacent_edges) = edge_adjacencies.maybe_enabled_by(left_id, Left) {
                adjacent_edges
            } else {
                // if a unique edge is only found on the left side of the image and config.wrap is
                // false then it will have no allowed neighbors
                continue;
            };

        for adjacent_edge in adjacent_edges.iter() {
            for other_pattern_id in edge_pattern_map[*adjacent_edge][Right].iter() {
                adjacency_rules.allow(pattern_id, *other_pattern_id, Left);
            }
        }

        let bottom_id = pattern_edge_ids[pattern_id][Down];
        let adjacent_edges =
            if let Some(adjacent_edges) = edge_adjacencies.maybe_enabled_by(bottom_id, Down) {
                adjacent_edges
            } else {
                // if a unique edge is only found on the bottom side of the image and config.wrap is
                // false then it will have no allowed neighbors
                continue;
            };
        for adjacent_edge in adjacent_edges {
            for other_pattern_id in edge_pattern_map[*adjacent_edge][Up].iter() {
                adjacency_rules.allow(pattern_id, *other_pattern_id, Down);
            }
        }
    }

    let mut pattern_data: Vec<(Pattern, usize, usize)> = patterns
        .into_iter()
        .map(|(p, (p_id, c))| (p, p_id, c))
        .collect();
    pattern_data.sort_by_key(|(_, p_id, _)| *p_id);

    #[cfg(test)]
    assert!(pattern_data
        .iter()
        .enumerate()
        .all(|(i, (_, id, _))| i == *id));

    let (patterns, tile_frequencies) = pattern_data.into_iter().map(|(p, _, c)| (p, c)).unzip();
    return WfcData {
        tile_frequencies,
        adjacency_rules,
        patterns,
    };
}

pub fn preprocess(image: RgbaImage, config: Config) -> WfcData {
    match config.adjacency_method {
        AdjacencyMethod::Adjacency => {
            return preprocess_simple_tiled(image, config);
        }
        AdjacencyMethod::Edge(EdgeMethod::Perfect) | AdjacencyMethod::Edge(EdgeMethod::Flip) => {
            return preprocess_edges(image, config);
        }
        AdjacencyMethod::Edge(EdgeMethod::Adjacent) => {
            return preprocess_adjacent_edges(image, config);
        }
        _ => unimplemented!(),
    }
}

fn get_edges(sub_img: &PatternSubImage, tile_size: UVec2) -> [Edge; 4] {
    // TODO: implement the logic done by sub_img.view for patterns (it's not that hard and will
    // remove the need to extract pattern twice)
    return CardinalDirs::as_array().map(|dir| {
        let sub_sub_img = match dir {
            Up => sub_img.view(0, 0, tile_size.x, 1),
            Left => sub_img.view(0, 0, 1, tile_size.y),
            Right => sub_img.view(tile_size.x - 1, 0, 1, tile_size.y),
            Down => sub_img.view(0, tile_size.y - 1, tile_size.x, 1),
        };
        let edge: Vec<Rgba<u8>> = sub_sub_img.pixels().map(|(_, _, rgba)| rgba).collect();
        return edge;
    });
}

// TODO: consider creating iterator type for iterating over locs to avoid allocating vec
// unnecessarily
pub fn get_tile_locs(image_dims: UVec2, tile_size: UVec2) -> Vec<UVec2> {
    // trim edges
    let max = image_dims - (image_dims % tile_size);

    let num_tiles = (max.x / tile_size.x) * (max.y / tile_size.y);
    let mut locs = Vec::with_capacity(num_tiles as usize);

    for y in (0..max.y).step_by(tile_size.y as usize) {
        for x in (0..max.x).step_by(tile_size.x as usize) {
            locs.push(UVec2 { x, y });
        }
    }

    return locs;
}

fn sub_image_at(
    image: &RgbaImage,
    loc: UVec2,
    tile_size: UVec2,
) -> SubImage<&ImageBuffer<Rgba<u8>, Vec<u8>>> {
    return image.view(loc.x, loc.y, tile_size.x, tile_size.y);
}

pub fn pattern_at(image: &RgbaImage, loc: UVec2, tile_size: UVec2) -> PatternRef {
    // this is what sub_image_at under the hood
    get_tile_locs(tile_size, UVec2::ONE)
        .into_iter()
        .map(|l| l + loc)
        .map(|l| &image.get_pixel(l.x, l.y).0)
        .collect()
}

// TODO: consider using this function to get HashMap<UVec2, PatternRef> that can then be used too:
// 1) convert to hashset for easy assigning of ids
// 2) with hashset allocating of tile_frequencies and adjacency_rules can be done with size
// 3) constructing tile_frequencies and adjacency_rules /may/ be made simpler by constructing loc: id map from the hashmap

fn extract_patterns(image: &RgbaImage, tile_size: UVec2) -> Vec<(UVec2, PatternRef)> {
    return get_tile_locs(image.dimensions().into(), tile_size)
        .iter()
        .map(|&loc| (loc, pattern_at(&image, loc, tile_size)))
        .collect();
}

fn pattern_ref_to_owned(pref: PatternRef) -> Pattern {
    pref.into_iter().map(|p| p.to_owned()).collect()
}

#[cfg(test)]
mod test {
    // TODO: recreate tests
    use super::*;

    #[test]
    fn get_edges_1_row() {
        const WIDTH: usize = 4;
        let img_bytes: [u8; 4 * WIDTH] = [1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 1, 1, 1, 1];
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(WIDTH as u32, 1, img_bytes.to_vec()).unwrap();
        let edges = get_edges(
            &img.view(0, 0, WIDTH as u32, 1),
            UVec2::new(WIDTH as u32, 1),
        );
        assert_eq!(edges[Up].len(), WIDTH);
        assert_eq!(edges[Down], edges[Up]);
        assert_eq!(
            edges[Right], edges[Left],
            "R {:?} != {:?} L",
            edges[Right], edges[Left]
        );
        assert_eq!(edges[Left].len(), 1);
    }

    fn load_image_from_bytes(raw_data: &[u8]) -> image::RgbaImage {
        // TODO: consider whether decoding here is really necessary
        //
        // Assuming it is so that Image figures out how to give me the vec of
        // pixels I want
        let reader = image::io::Reader::new(std::io::Cursor::new(raw_data))
            .with_guessed_format()
            .expect("Cursor io never fails");
        let image = reader.decode().unwrap().to_rgba8();
        return image;
    }
    const LEDGE_IMG: &[u8] = include_bytes!("../../inputs/ledge.png");
    const DUAL_IMG: &[u8] = include_bytes!("../../inputs/dual.png");

    #[test]
    fn ledge_img() {
        let image = load_image_from_bytes(DUAL_IMG);
        let config = Config {
            tile_size: UVec2::splat(32),
            adjacency_method: AdjacencyMethod::Edge(EdgeMethod::Adjacent),
            pattern_method: PatternMethod::Tiled,
        };
        let data = preprocess(image, config);
        assert_eq!(
            data.patterns.len(),
            16,
            "expected 16 patterns found: {}",
            data.patterns.len()
        );
        assert_eq!(
            data.patterns.len(),
            data.tile_frequencies.len(),
            "patterns.len != tile_frequencies.len"
        );
    }
}
