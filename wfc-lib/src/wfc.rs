use std::{
    cmp::{Ordering, Reverse},
    collections::{BinaryHeap, hash_map::RandomState},
    iter::{repeat, zip},
    ops::{Index, IndexMut},
};

use derive_more::{Deref, DerefMut};
use glam::{IVec2, UVec2};

use crate::{
    adjacency_rules::{AdjacencyRules, CardinalDirs, EnablerDict},
    preprocessor::Pattern,
    tile::{IdMap, TileId},
    Area, Grid,
};
use rand::prelude::*;
// TODO: rename all "tile" to "pattern" and "cell" to "tile"

/// A Cell corresponds to a pattern in the output image
/// It tracks the possible tiles (and their corresponding patterns) from
/// the input image
/// When the number of possible tiles is 1 the cell is considered to
/// be "collapsed" and in it's final state
#[derive(Debug, Clone)]
pub struct Cell {
    pub collapsed_to: Option<TileId>,
    pub domain: EnablerDict,
    pub probability_dict: ProbabilityDict,
    pub loc: UVec2,
    pub random_entropy: f32,
}

impl Cell {
    fn new(probability_dict: ProbabilityDict, enabler_dict: EnablerDict, loc: UVec2, random_entropy: f32) -> Self {
        return Self {
            collapsed_to: None,
            domain: enabler_dict,
            probability_dict,
            loc,
            random_entropy
        };
    }

    pub fn collapsed(&self) -> bool {
        return self.collapsed_to.is_some();
    }

    fn choose_collapse_tile(&self) -> TileId {
        if self.collapsed() {
            unreachable!("Cell has already been collapsed");
        }
        let mut rng = rand::thread_rng();
        return self
            .domain
            .allowed_tile_ids()
            .choose(&mut rng)
            .expect("cell has possible tiles");
    }

    fn collapse(&mut self) -> Vec<TileRemovalEvent> {
        let fin: TileId = self.choose_collapse_tile();
        self.collapsed_to = Some(fin);
        let removed_tile_ids = self.domain.remove_all_but(fin);
        let tile_removed_events =
            TileRemovalEvent::from_list_of_removed_tiles(removed_tile_ids, self.loc);
        return tile_removed_events;
    }

    fn remove_enabler(
        &mut self,
        enabler: TileId,
        from_dir: CardinalDirs,
        adjacency_rules: &AdjacencyRules,
    ) -> Option<Vec<TileRemovalEvent>> {
        if self.collapsed() {
            // assert!(adjacency_rules.enabled_by(enabler, from_dir).contains(&self.collapsed_to.unwrap()), "Contradiction: trying to remove enabler {enabler} which enables {} which this cell is collapsed to", self.collapsed_to.unwrap());
            // log::warn!("Contradiction: tried to remove enabler: {enabler} from cell that was collapsed to that tile");
            return None;
        }
        let removed_tiles = self
            .domain
            .remove_single(enabler, from_dir, adjacency_rules)?;
        for &tile in &removed_tiles {
            self.probability_dict.remove(tile);
        }
        let events = TileRemovalEvent::from_list_of_removed_tiles(removed_tiles, self.loc);
        return Some(events);
    }

    // TODO: Move this too window
    pub fn render(&self, patterns: &Vec<Pattern>, tile_size: usize) -> Pattern {
        let allowed_patterns: Vec<(TileId, Pattern)> =
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
                let mut fin_pix =
                    weighted_pix.map(|channel| (channel / self.probability_dict.total_count) as u8);
                fin_pix[3] = 255;
                fin_pix
            })
            .collect();
    }

    fn get_entropy_entry(&self) -> EntropyEntry {
        return EntropyEntry {
            entropy: self.probability_dict.entropy() + self.random_entropy,
            loc: self.loc,
        };
    }
}

#[derive(Debug, Default)]
pub struct Model {
    entropy_heap: MinEntropyHeap,
    adjacency_rules: AdjacencyRules,
    board: Board,
    // tile_frequencies: IdMap<usize>,
    // dims: UVec2,
    wave: Vec<TileRemovalEvent>,
    pub remaining_uncollapsed: u32,
    updated_cells: Vec<UVec2>,
    // tile_size: usize,
}

impl Model {
    pub fn new(adjacency_rules: AdjacencyRules, tile_frequencies: Vec<usize>, dims: UVec2) -> Self {
        let grid = Grid(dims);
        let num_cells = grid.area();

        // TODO: consider just initializing these in  cell init
        // for cleanliness
        let probability_dict = ProbabilityDict::new(&tile_frequencies);
        let mut entropy_heap = MinEntropyHeap::new();
        let enabler_dict = EnablerDict::new(&adjacency_rules);

        let mut cells = Vec::with_capacity(num_cells as usize);

        for (i, loc) in grid.iter_locs().enumerate() {
            let random_entropy = rand::thread_rng().gen();
            let cell = Cell::new(probability_dict.clone(), enabler_dict.clone(), loc, random_entropy);
            entropy_heap.push(cell.get_entropy_entry());
            cells.push(cell);
        }
        let board = Board { grid, vals: cells };
        let updated_cells = Vec::with_capacity(num_cells as usize);
        return Self {
            adjacency_rules,
            updated_cells,
            // tile_frequencies,
            board,
            // dims,
            entropy_heap,
            remaining_uncollapsed: num_cells,
            ..Default::default()
        };
    }

    pub fn get_cell_mut(&mut self, loc: UVec2) -> Option<&mut Cell> {
        return self.board.get_cell_mut(loc.as_ivec2());
    }

    pub fn get_cell(&self, loc: UVec2) -> Option<&Cell> {
        return self.board.get_cell(loc.as_ivec2());
    }

    pub fn get_cell_to_collapse(&mut self) -> Option<UVec2> {
        if self.remaining_uncollapsed == 0 {
            return None;
        }
        while let Some(entry) = self.entropy_heap.pop() {
            let cell = self
                .get_cell(entry.loc)
                .expect("entropy heap entries should all be inbounds");
            if !cell.collapsed() {
                return Some(cell.loc);
            }
        }

        unreachable!("Entropy Heap should never be empty");
    }

    pub fn collapse_cell(&mut self) {
        if let Some(loc) = self.get_cell_to_collapse() {
            log::info!("Collapsing Cell at {loc:?}");
            let tile_removed_events = {
                let cell = &mut self
                    .get_cell_mut(loc)
                    .expect("entropy heap entries should all be inbounds");
                cell.collapse()
            };

            self.wave = tile_removed_events;
            self.remaining_uncollapsed -= 1;
            self.updated_cells.push(loc);
            log::trace!(
                "Collapsed cell {:?}. Removed {}/{} tile options",
                loc,
                self.wave.len(),
                self.adjacency_rules.len()
            );
        }
    }

    pub fn propogate(&mut self) {
        match self.wave.pop() {
            Some(event) => {
                log::info!(
                    "Propogating removal of tile {} from cell at {:?}",
                    event.tile_id,
                    event.cell_loc
                );
                assert!(self.board.inbounds(event.cell_loc.as_ivec2()));

                let adjacent_tile_locs = self.board.cardinal_neighbors(event.cell_loc);
                for (dir, adjacent_tile_loc) in adjacent_tile_locs {
                    if !self.board.inbounds(adjacent_tile_loc) {
                        continue;
                    }
                    log::trace!(
                        "{:?} -> {:?} -> {:?}",
                        event.cell_loc,
                        dir,
                        adjacent_tile_loc
                    );
                    let adj_cell = self
                        .board
                        .get_cell_mut(adjacent_tile_loc)
                        .expect("adjacent tile is inbounds");
                    if let Some(tile_removed_events) =
                        adj_cell.remove_enabler(event.tile_id, dir, &self.adjacency_rules)
                    {
                        log::trace!("removed {} options", tile_removed_events.len());
                        for event in tile_removed_events {
                            self.wave.push(event);
                        }
                        self.updated_cells.push(adjacent_tile_loc.as_uvec2());
                    }
                    let entropy_entry = adj_cell.get_entropy_entry();
                    self.entropy_heap.push(entropy_entry);
                }
            }
            None => unreachable!("If wave was empty we should have collapsed a cell instead"),
        }
    }

    /// steps the model one iteration by either collapsing or propogating TileRemovalEvents
    pub fn step(&mut self) -> Vec<UVec2> {
        // no tiles left to collapse -> done
        if self.remaining_uncollapsed == 0 {
            for cell in self.iter_cells() {
                assert!(cell.domain.allowed_tile_ids().count() == 1);
            }
            return Vec::new();
        }
        // stack empty -> need to collapse a tile
        if self.wave.is_empty() {
            self.collapse_cell();
        } else {
            self.propogate();
        }

        let mut updated_cells: Vec<UVec2> = Vec::with_capacity(self.updated_cells.len());
        // NOTE: append moves the values out of self.updated_cells and into the new vec that is
        // returned
        updated_cells.append(&mut self.updated_cells);
        return updated_cells;
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
    pub fn inbounds(&self, loc: IVec2) -> bool {
        return loc.cmpge(IVec2::ZERO).all() && loc.cmplt(self.grid.as_ivec2()).all();
    }

    pub fn cardinal_neighbors(&self, loc: UVec2) -> [(CardinalDirs, IVec2); 4] {
        let loc = loc.as_ivec2();
        let neighbors = [
            CardinalDirs::Up,
            CardinalDirs::Right,
            CardinalDirs::Down,
            CardinalDirs::Left,
        ]
        .map(|dir| (dir, dir + loc));
        return neighbors;
    }

    pub fn get_cell(&self, loc: IVec2) -> Option<&Cell> {
        if !self.inbounds(loc) {
            return None;
        }
        return Some(&self[loc]);
    }

    pub fn get_cell_mut(&mut self, loc: IVec2) -> Option<&mut Cell> {
        if !self.inbounds(loc) {
            return None;
        }
        return Some(&mut self[loc]);
    }
}

impl Index<IVec2> for Board {
    type Output = Cell;

    fn index(&self, index: IVec2) -> &Self::Output {
        let i = self.index_grid(index.as_uvec2());
        return &self.vals[i];
    }
}

impl IndexMut<IVec2> for Board {
    fn index_mut(&mut self, index: IVec2) -> &mut Self::Output {
        let i = self.index_grid(index.as_uvec2());
        return &mut self.vals[i];
    }
}

#[derive(Debug, Clone)]
pub struct ProbabilityDict {
    pub counts: IdMap<usize>,
    total_shannons: f32,
    pub total_count: usize,
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

#[derive(Default, Debug)]
pub struct MinEntropyHeap(BinaryHeap<Reverse<EntropyEntry>>);

impl MinEntropyHeap {
    fn push(&mut self, e: EntropyEntry) {
        self.0.push(Reverse(e));
    }
    fn pop(&mut self) -> Option<EntropyEntry> {
        // unwrap from reverse
        // TODO: just return loc?
        self.0.pop().map(|Reverse(entry)| entry)
    }
    pub fn new() -> Self {
        return Self(BinaryHeap::new());
    }
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
pub struct TileRemovalEvent {
    pub tile_id: usize,
    pub cell_loc: UVec2,
}

impl TileRemovalEvent {
    pub fn new(tile_id: TileId, cell_loc: UVec2) -> Self {
        return Self { tile_id, cell_loc };
    }
    pub fn from_list_of_removed_tiles(removed_tiles: Vec<TileId>, loc: UVec2) -> Vec<Self> {
        return removed_tiles
            .iter()
            .map(|&removed_tile_id| Self::new(removed_tile_id, loc))
            .collect();
    }
}

#[cfg(test)]
mod test {
    use crate::simple_patterns::{construct_simple_patterns, CHARS};

    use super::*;

    fn all_adjacency_rules_satisfied(model: &Model) {
        for cell_loc in model.board.grid.iter_locs() {
            let cell = model.get_cell(cell_loc).unwrap();
            for (dir, adjacent_cell_loc) in model.board.cardinal_neighbors(cell_loc) {
                // if adjacent_cell_loc inbounds (adj_cell exists)
                if let Some(adj_cell) = model.board.get_cell(adjacent_cell_loc) {
                    let mut cell_domain_in_dir: Vec<usize> = cell
                        .domain
                        .allowed_tile_ids()
                        .flat_map(|tile_id| model.adjacency_rules.enabled_by(tile_id, dir))
                        .collect();
                    cell_domain_in_dir.sort();
                    cell_domain_in_dir.dedup();
                    for adj_allowed_tile_id in adj_cell.domain.allowed_tile_ids() {
                        assert!(cell_domain_in_dir.contains(&adj_allowed_tile_id), "cell at {cell_loc:?} with domain {:?} in direction {dir:?} has neighbor at {adjacent_cell_loc} with possible tile {} that should not be allowed", cell_domain_in_dir.iter().map(|&tile_id| CHARS[tile_id]).collect::<Vec<&str>>(), CHARS[adj_allowed_tile_id]);
                    }
                }
            }
        }
    }

    #[test]
    fn adjacency_rules_fulfilled_always() {
        let mut wfc = construct_simple_patterns().with_output_dimensions(40, 40);
        let mut model = wfc.get_model();
        while model.remaining_uncollapsed > 0 {
            if model.wave.is_empty() {
                all_adjacency_rules_satisfied(&model);
            }
            model.step();
        }
    }

    #[test]
    fn entropy_calculations() {
        let tile_frequencies = [1, 2, 3, 4, 5];
        let calculated_entropy: f32 = 2.149;
        let mut prob = ProbabilityDict::new(&tile_frequencies.to_vec());
        assert!((calculated_entropy - prob.entropy()) <= f32::EPSILON);
        prob.remove(4);
        assert!(prob.total_count == 10);
        let calculated_entropy: f32 = 1.621;
        assert!((calculated_entropy - prob.entropy()) <= f32::EPSILON);
    }

    #[test]
    fn entropy_heap_is_min_heap() {
        let mut heap = MinEntropyHeap::new();
        let min_entry = EntropyEntry {
            entropy: 0.1,
            loc: UVec2::X,
        };
        let max_entry = EntropyEntry {
            entropy: 0.2,
            loc: UVec2::Y,
        };
        heap.push(min_entry);
        heap.push(max_entry);
        assert!(heap.pop() == Some(min_entry));
    }
}
