use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    iter::zip,
    ops::{Index, IndexMut},
};

use glam::UVec2;
use derive_more::{Deref,DerefMut};

use crate::{
    adjacency_rules::{AdjacencyRules, EnablerDict, TileRemovalEvent},
    Area, Grid, IdMap, TileId,
};

#[derive(Debug, Clone)]
pub struct Cell {
    pub collapsed: bool,
    pub domain: EnablerDict,
    counts: Vec<usize>,
    shannons: Vec<f32>,
    pub loc: UVec2,
}

impl Cell {
    fn new(len: usize, enabler_dict: EnablerDict, freqs: Vec<usize>, loc: UVec2) -> Self {
        let counts = freqs;
        let shannons = counts
            .iter()
            .map(|&freq| (freq as f32) * (freq as f32).log2())
            .collect();
        return Self {
            collapsed: false,
            domain: enabler_dict,
            counts,
            shannons,
            loc,
        };
    }
    fn entropy(&self, tile_frequencies: &Vec<usize>) -> f32 {
        if self.collapsed {
            return 0_f32;
        }
        // Shannon Entropy = H(x) = -P(x)*log(P(x))
        // H(x0..xn) = H(x0) + H(x1) ... H(xn)
        // P(xi) = wi / sum(w)
        // where wi is the weight of xi and sum(w) is the sum of the weights
        // H(x) = log(sum(w))- (w0*log(w0) -w1*log(w1) ... -wn*log(wn)) / sum(w)
        let total_count = self.domain.filter_allowed(tile_frequencies).sum::<usize>() as f32;
        let total_shannon = self
            .domain
            .filter_allowed(tile_frequencies)
            .map(|freq| (freq as f32) * (freq as f32).log2())
            .sum::<f32>();
        // H(x) = log(sum(w))- (w0*log(w0) -w1*log(w1) ... -wn*log(wn)) / sum(w)
        return total_count.log2() - (total_shannon / total_count);
    }

    fn choose_collapse_tile(&self) -> TileId {
        if self.collapsed {
            unreachable!("Cell has already been collapsed");
        }
        use rand::seq::IteratorRandom;
        let mut rng = rand::thread_rng();
        return self
            .domain
            .allowed_iter()
            .choose(&mut rng)
            .expect("rand works");
    }

    fn collapse(&mut self) {
        let fin: TileId = self.choose_collapse_tile();
        self.domain.disallow_all_but(fin);
        self.collapsed = true;
    }
}

#[derive(Debug, Default)]
pub struct Model {
    entropy_heap: BinaryHeap<EntropyEntry>,
    adjacency_rules: AdjacencyRules,
    board: Board,
    tile_frequencies: IdMap<usize>,
    dims: UVec2,
    wave: Vec<TileRemovalEvent>,
    remaining_uncollapsed: usize,
}

impl Model {
    pub fn new(adjacency_rules: AdjacencyRules, tile_frequencies: Vec<usize>, dims: UVec2) -> Self {
        let len = tile_frequencies.len();

        let num_cells = Grid(dims).area();

        let grid = dbg!(Grid(dims));
        let mut vals = Vec::with_capacity(dbg!(grid.area()) as usize);
        for loc in grid.iter_locs() {
            let cell = Cell::new(
                len,
                EnablerDict::new(&adjacency_rules),
                tile_frequencies.clone(),
                dbg!(loc),
            );
            vals.push(cell);
        }
        let board = Board { grid, vals };
        return Self {
            adjacency_rules,
            tile_frequencies,
            board,
            dims,
            remaining_uncollapsed: Grid(dims).area() as usize,
            ..Default::default()
        };
    }

    pub fn get_cell_mut(&mut self, loc: UVec2) -> &mut Cell {
        return &mut self.board[loc];
    }

    pub fn get_cell(&self, loc: UVec2) -> &Cell {
        return &self.board[loc];
    }

    pub fn get_cell_to_collapse(&mut self) -> UVec2 {

        // while let Some(entry) = self.entropy_heap.pop() {
        //     let cell = &self.board[entry.loc];
        //     if !cell.collapsed {
        //         return entry.loc;
        //     }
        // }

        return self.iter_cells().find_map(|c| if !c.collapsed {Some(c.loc)} else {None}).expect("no Contradiction");

        unreachable!("Entropy Heap should never be empty");
    }

    pub fn collapse_cell(&mut self) {
        let loc = self.get_cell_to_collapse();
        self.get_cell_mut(loc).collapse();
    }

    pub fn step(&mut self) {
        // no tiles left to collapse -> done
        if self.remaining_uncollapsed == 0 {
            return;
        }
        // stack empty -> need to collapse a tile
        if self.wave.is_empty() {
            self.collapse_cell();
        } else {
            // self.propogate();
            return;
        }
    }

    pub fn iter_cells(&self) -> impl Iterator<Item = &Cell> {
        return self.board.iter();
    }
}


#[derive(Debug, Deref, DerefMut, Default)]
pub struct Board {
    grid: Grid,
    #[deref_mut]
    #[deref]
    vals: Vec<Cell>,
}

impl Board {
    #[inline]
    fn index_grid(&self, loc: UVec2) -> usize {
        return (loc.y * self.grid.x + loc.x) as usize;
    }
}

impl Index<UVec2> for Board {
    type Output = Cell;

    fn index(&self, index: UVec2) -> &Self::Output {
        let i = self.index_grid(index);
        return &self.vals[i];
    }
}

impl IndexMut<UVec2> for Board {
    fn index_mut(&mut self, index: UVec2) -> &mut Self::Output {
        let i = self.index_grid(index);
        return &mut self.vals[i];
    }
}

#[derive(Debug)]
struct EntropyEntry {
    entropy: f32,
    loc: UVec2,
}
impl PartialEq for EntropyEntry {
    fn eq(&self, other: &Self) -> bool {
        self.entropy == other.entropy
    }
}

impl Eq for EntropyEntry {}

impl PartialOrd for EntropyEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.entropy.partial_cmp(&other.entropy) {
            // resolve equality by comparing location
            Some(Ordering::Equal) => self.loc.to_array().partial_cmp(&other.loc.to_array()),
            lt_or_gt => lt_or_gt,
        }
    }
}

impl Ord for EntropyEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.entropy.total_cmp(&other.entropy) {
            // resolve equality by comparing location
            Ordering::Equal => self.loc.to_array().cmp(&other.loc.to_array()),
            lt_or_gt => lt_or_gt,
        }
    }
}
