use std::{collections::BinaryHeap, cmp::Ordering, ops::{Deref, DerefMut}, iter::zip};

use glam::UVec2;

use crate::adjacency_rules::AdjacencyRules;

#[derive(Debug,Clone)]
pub struct Cell {
    collapsed: bool,
    domain: Domain,
}

impl Cell {
    fn new(len: usize) -> Self {
        return Self { collapsed: false, domain: Domain::new(len) };
    }
    fn entropy(&self, tile_frequencies: &Vec<usize>) -> f32 {
        // Shannon Entropy = H(x) = -P(x)*log(P(x))
        // H(x0..xn) = H(x0) + H(x1) ... H(xn)
        // P(xi) = wi / sum(w)
        // where wi is the weight of xi and sum(w) is the sum of the weights
        // H(x) = log(sum(w))- (w0*log(w0) -w1*log(w1) ... -wn*log(wn)) / sum(w)
        let total_count =
            self.domain
            .filter_allowed(tile_frequencies)
            .sum::<usize>() as f32;
        let total_shannon = 
            self.domain
            .filter_allowed(tile_frequencies)
            .map(|freq| (freq as f32) * (freq as f32).log2())
            .sum::<f32>();
        // H(x) = log(sum(w))- (w0*log(w0) -w1*log(w1) ... -wn*log(wn)) / sum(w)
        return total_count.log2() - (total_shannon / total_count);
    }
}

#[derive(Debug, Default)]
pub struct Model {
    entropy: BinaryHeap<EntropyEntry>,
    adjacency_rules: AdjacencyRules,
    grid: Vec<Vec<Cell>>,
    tile_frequencies: Vec<usize>,
    output_dims: UVec2,
}

impl Model {
    pub fn new(adjacency_rules: AdjacencyRules, tile_frequencies: Vec<usize>, output_dims: UVec2) -> Self {
        let len = tile_frequencies.len();
        let grid = vec![vec![Cell::new(len); output_dims.y as usize]; output_dims.x as usize];
        return Self {adjacency_rules, tile_frequencies, grid, output_dims,..Default::default()};
    }
}

#[derive(Debug,Clone)]
struct Domain(Vec<bool>);

impl Deref for Domain {
    type Target = Vec<bool>;

    fn deref(&self) -> &Self::Target {
        return &self.0;
    }
}

impl DerefMut for Domain {
    fn deref_mut(&mut self) -> &mut Self::Target {
        return &mut self.0;
    }
}

impl Domain {
    fn new(len: usize) -> Self {
        return Self(vec![true;len]);
    }
    fn allowed_iter(&self) -> impl Iterator<Item=usize> + '_ {
        return self.iter()
            .enumerate()
            .filter_map(|(idx,&b)| {
                if b {
                    Some(idx)
                } else {
                    None
                }
            });
    }
    fn filter_allowed<'a, T>(&'a self, other: &'a Vec<T>) -> impl Iterator<Item=T> + '_
    where T: Clone {
        assert_eq!(self.len(), other.len());
        return zip(&self.0,other)
            .filter_map(|(&b,v)| {
                if b {
                    Some(v.clone())
                } else {
                    None
                }
            });
    }
    fn filter_allowed_enumerate<'a, T>(&'a self, other: &'a Vec<T>) -> impl Iterator<Item=(usize,T)> + '_
    where T: Clone {
        assert_eq!(self.len(), other.len());
        return zip(&self.0,other)
            .enumerate()
            .filter_map(|(i,(&b,v))| {
                if b {
                    Some((i,v.clone()))
                } else {
                    None
                }
            });
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

impl Eq for EntropyEntry { }

impl PartialOrd for EntropyEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.entropy.partial_cmp(&other.entropy) {
            // resolve equality by comparing location
            Some(Ordering::Equal) => self.loc.to_array().partial_cmp(&other.loc.to_array()),
            lt_or_gt => lt_or_gt
        }
    }
}

impl Ord for EntropyEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.entropy.total_cmp(&other.entropy) {
            // resolve equality by comparing location
            Ordering::Equal => self.loc.to_array().cmp(&other.loc.to_array()),
            lt_or_gt => lt_or_gt
        }
    }
}
