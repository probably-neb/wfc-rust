use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    iter::{repeat, zip},
    ops::{Index, IndexMut},
};

use derive_more::{Deref, DerefMut};
use glam::UVec2;

use crate::{
    adjacency_rules::{AdjacencyRules, EnablerDict, TileRemovalEvent},
    preprocessor::{Pattern, RgbaArrPattern},
    tile::{IdMap, TileId},
    Area, Grid,
};

/// A Cell corresponds to a pattern in the output image
/// It tracks the possible tiles (and their corresponding patterns) from
/// the input image
/// When the number of possible tiles is 1 the cell is considered to
/// be "collapsed" and in it's final state
#[derive(Debug, Clone)]
pub struct Cell {
    pub collapsed: bool,
    pub domain: EnablerDict,
    pub probability_dict: ProbabilityDict,
    pub loc: UVec2,
}

impl Cell {
    fn new(probability_dict: ProbabilityDict, enabler_dict: EnablerDict, loc: UVec2) -> Self {
        return Self {
            collapsed: false,
            domain: enabler_dict,
            probability_dict,
            loc,
        };
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

    fn collapse(&mut self) -> Vec<TileRemovalEvent> {
        let fin: TileId = self.choose_collapse_tile();
        self.collapsed = true;
        let tile_removed_events = self.domain.remove_all_but(fin);
        return tile_removed_events;
    }

    pub fn render(&self, patterns: &Vec<RgbaArrPattern>, tile_size: usize) -> RgbaArrPattern {
        let allowed_patterns: Vec<(TileId, RgbaArrPattern)> =
            self.domain.filter_allowed_enumerate(patterns).collect();
        if allowed_patterns.len() == 1 {
            return allowed_patterns[0].1.to_owned();
        }
        return allowed_patterns
            .iter()
            .map(|(id, p)| -> Vec<[usize; 4]> {
                let count: usize = self.probability_dict.counts[*id];
                p.iter()
                    .map(|pixel| pixel.map(|channel| channel as usize * count))
                    .collect()
            })
            .fold(vec![[0; 4]; tile_size * tile_size], |acc, pat| {
                zip(acc, pat)
                    .map(|(acc_pix, pat_pix)| {
                        [
                            acc_pix[0] + pat_pix[0],
                            acc_pix[1] + pat_pix[1],
                            acc_pix[2] + pat_pix[2],
                            acc_pix[3] + pat_pix[3],
                        ]
                    })
                    .collect()
            })
            .iter()
            .map(|weighted_pix| {
                let mut fin_pix = weighted_pix.map(|channel| {
                    (channel / self.probability_dict.total_count) as u8
                });
                fin_pix[3] = 255;
                fin_pix
            })
            .collect();
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
    tile_size: usize,
}

impl Model {
    pub fn new(adjacency_rules: AdjacencyRules, tile_frequencies: Vec<usize>, dims: UVec2) -> Self {
        let len = tile_frequencies.len();

        let num_cells = Grid(dims).area();

        // TODO: consider just initializing these in  cell init
        // for cleanliness
        let probability_dict = ProbabilityDict::new(&tile_frequencies);
        let enabler_dict = EnablerDict::build(&adjacency_rules);

        let grid = dbg!(Grid(dims));
        let mut vals = Vec::with_capacity(num_cells as usize);
        for loc in grid.iter_locs() {
            let cell = Cell::new(probability_dict.clone(), enabler_dict.with_loc(loc), loc);
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

    pub fn get_cell_to_collapse(&mut self) -> Option<UVec2> {
        // while let Some(entry) = self.entropy_heap.pop() {
        //     let cell = &self.board[entry.loc];
        //     if !cell.collapsed {
        //         return entry.loc;
        //     }
        // }

        return self
            .iter_cells()
            .find_map(|c| if !c.collapsed { Some(c.loc) } else { None })

        // unreachable!("Entropy Heap should never be empty");
    }

    pub fn collapse_cell(&mut self) {
        match self.get_cell_to_collapse() {
            Some(loc) => {
                let tile_removed_events = self.get_cell_mut(loc).collapse();
                self.wave = tile_removed_events;
                self.remaining_uncollapsed -= 1;
            }
            None => ()
        }
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
struct Board {
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

#[derive(Debug, Clone)]
pub struct ProbabilityDict {
    counts: IdMap<usize>,
    total_shannons: f32,
    total_count: usize,
}

impl ProbabilityDict {
    fn new(tile_frequencies: &IdMap<usize>) -> Self {
        let counts = tile_frequencies.to_owned();
        let total_shannons = counts.iter().map(|&freq| Self::partial_shannon(freq)).sum();
        let total_count = tile_frequencies.iter().sum();
        return Self {
            counts,
            total_shannons,
            total_count,
        };
    }

    /// one of the `w0 * log(w0)` terms in the simplified entropy equation
    #[inline]
    fn partial_shannon(freq: usize) -> f32 {
        let freq = freq as f32;
        return freq * freq.log2();
    }

    /// Calculates the Shannon Entropy `H(x) = -P(x)*log(P(x))`
    ///
    /// The Shannon Entropy For set `x` of items `x0..xn` is
    /// `H(x) = H(x0) + H(x1) ... H(xn)`
    ///
    /// The Probability of an entry `xi` is `P(xi) = wi / sum(w)`
    /// where `wi` is the weight of xi i.e. `self.counts[i]`
    /// and `sum(w)` is the sum of the weights i.e. `self.total_count`
    ///
    /// The simplified Entropy Equation is then:
    /// `H(x) = log(sum(w))- (w0*log(w0) -w1*log(w1) ... -wn*log(wn)) / sum(w)`
    fn entropy(&self) -> f32 {
        if self.total_count == 0 {
            return std::f32::NAN;
        }
        let total_count: f32 = self.total_count as f32;
        // Calculate entropy using simplified shannon entropy
        return total_count.log2() - (self.total_shannons / total_count);
    }

    /// Returns an IdMap of the relative probabilities of each tile
    pub fn relative_probabilities(&self) -> IdMap<f32> {
        return zip(&self.counts, repeat(self.total_count as f32))
            .map(|(&c, t)| (c as f32) / t)
            .collect();
    }

    fn remove(&mut self, id: TileId) {
        self.total_count -= self.counts[id];
        self.total_shannons -= Self::partial_shannon(self.counts[id]);
        self.counts[id] = 0;
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
