use crate::point;
use point::{Dimens, Dir, Loc, Point};

pub type IdVec = Vec<bool>;

#[derive(Debug, Clone)]
pub struct Tile {
    pub loc: Loc,
    pub dom: Vec<bool>,
}

fn variable_entropy(prob: f32) -> f32 {
    return prob * (1.0 / prob).log(2.0);
}

impl Tile {
    fn new(loc: Loc, dlen: usize) -> Self {
        let dom: IdVec = (0..dlen).map(|_| true).collect();
        return Tile { loc, dom };
    }

    pub fn entropy(&self, probs: &Vec<f32>) -> f32 {
        assert_eq!(self.dom.len(), probs.len());
        return self
            .dom
            .iter()
            .zip(probs.iter())
            .filter(|(dom, _)| **dom)
            .map(|(_, prob)| prob)
            .map(|prob| variable_entropy(*prob))
            .sum();
    }
}

type AdjacencyMatrix = Vec<[Vec<bool>; 4]>;

trait Board {
    fn new(num_patterns: usize, dimensions: &Dimens) -> Self;
}

impl Board for Vec<Tile> {
    fn new(num_patterns: usize, dimensions: &Dimens) -> Self {
        return dimensions
            .to_coord_list()
            .into_iter()
            .map(|loc| Tile::new(loc, num_patterns))
            .collect();
    }
}

#[derive(Debug)]
pub struct Model {
    pub out_dims: Dimens,
    pub board: Vec<Tile>,
    // adj_mat: AdjacencyMatrix
}

impl Model {
    pub fn new(num_patterns: usize, out_dims: Dimens) -> Self {
        let board = Board::new(num_patterns, &out_dims);
        return Model { out_dims, board };
    }

    pub fn iter_tiles(&self) -> std::slice::Iter<Tile> {
        return self.board.iter();
    }

    pub fn min_nz_entropy(&self, probs: &Vec<f32>) -> Option<Loc> {
        // TODO: get list of min entropy tiles and choose random for less predictable output
        return self.iter_tiles()
                .map(|tile| (tile.loc, tile.entropy(probs)))
                .filter(|(_, entropy)| *entropy != 0.0)
                .min_by(|(_, e1), (_, e2)| e1.total_cmp(e2))
                .map(|(loc,_)| loc);
    }
}
