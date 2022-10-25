use crate::image_utils::Merge;
use crate::point;
use crate::Image;
use point::{Dimens, Dir, Loc};
use std::ops::Index;
use std::ops::IndexMut;
use std::rc::Rc;

pub type Domain = Vec<bool>;

trait Entropy {
    fn entropy(&self) -> f32;
}

type Probability = f32;

impl Entropy for Probability {
    fn entropy(&self) -> Probability {
        let prob = self;
        return prob * (1.0 / prob).log(2.0);
    }
}

impl Entropy for Vec<Probability> {
    fn entropy(&self) -> f32 {
        return self.iter().map(|&prob| prob.entropy()).sum();
    }
}

impl Entropy for Tile {
    fn entropy(&self) -> f32 {
        let probs: Vec<Probability> = self
            .dom
            .iter()
            .zip(self.probs.iter())
            .filter(|(indom, _)| **indom)
            .map(|(_, prob)| *prob)
            .collect();
        if probs.len() <= 1 {
            return 0.0;
        } else {
            return probs.entropy();
        }
    }
}

#[derive(Debug)]
pub enum TileStateEnum {
    Collapsed,
    UnCollapsed,
}
use TileStateEnum::*;

#[derive(Debug)]
pub struct Tile {
    //TODO: could replace Tile with Collapsed enum wrapping tile
    pub loc: Loc,
    pub dom: Domain,
    pub probs: Rc<Vec<Probability>>,
    pub patterns: Rc<Vec<Image>>,
    pub img: Image,
    pub state: TileStateEnum,
}

impl Tile {
    fn new(loc: Loc, dlen: usize, probs: Rc<Vec<Probability>>, patterns: Rc<Vec<Image>>) -> Self {
        let dom: Domain = (0..dlen).map(|_| true).collect();
        let img = Image::empty();
        let mut tile = Tile {
            loc,
            dom,
            probs,
            patterns,
            img,
            state: UnCollapsed,
        };
        tile.update_image();
        return tile;
    }

    fn update_image(&mut self) {
        let mut images = self
            .dom
            .iter()
            .zip(self.patterns.iter())
            .filter(|(&in_domain, _)| in_domain)
            .map(|(_, pic)| pic);
        match self.state {
            Collapsed => images.next().map(|img| {
                self.img.bytes = img.bytes.clone();
            }),
            UnCollapsed => {
                images.fold(&mut self.img, |p1, p2| {
                    Image::merge_mut(p1, p2);
                    return p1;
                });
                return;
            }
        };
    }

    fn collapse(&mut self, heuristic: fn(&Tile) -> Option<usize>) {
        match self.state {
            UnCollapsed => {
                let mayb_idx = heuristic(self);
                self.dom.fill(false);
                Option::map(mayb_idx, |idx| {
                    self.dom[idx] = true;
                });
                self.state = Collapsed;
                self.update_image();
            }
            Collapsed => panic!("Tried to Collapse collapsed Tile"),
        }
    }
}

type AdjacencyMatrix = Vec<[Vec<bool>; 4]>;

#[derive(Debug)]
pub struct Board {
    pub w: usize,
    pub h: usize,
    pub tiles: Vec<Tile>,
}

impl Board {
    fn new(
        num_patterns: usize,
        dimensions: &Dimens,
        probs: &Rc<Vec<Probability>>,
        patterns: &Rc<Vec<Image>>,
    ) -> Self {
        let tiles = dimensions
            .to_coord_list()
            .into_iter()
            .map(|loc| Tile::new(loc, num_patterns, probs.clone(), patterns.clone()))
            .collect();
        return Board {
            w: dimensions.x,
            h: dimensions.y,
            tiles,
        };
    }

    fn iter(&self) -> std::slice::Iter<Tile> {
        return self.tiles.iter();
    }

    pub fn min_entropy_loc(&self) -> Option<Loc> {
        // TODO: get list of min entropy tiles and choose random for less predictable output
        // PERF: don't call entropy on tile so many times
        return self
            .iter()
            .filter(|tile| tile.entropy() != 0.0)
            .min_by(|t1, t2| t1.entropy().total_cmp(&t2.entropy()))
            .map(|tile| tile.loc);
    }
}

impl Index<usize> for Board {
    type Output = Tile;
    fn index(&self, index: usize) -> &Self::Output {
        return &self.tiles[index];
    }
}

impl Index<Loc> for Board {
    type Output = Tile;
    fn index(&self, loc: Loc) -> &Self::Output {
        let index = loc.to_index(self.w);
        return &self[index];
    }
}

impl IndexMut<Loc> for Board {
    fn index_mut(&mut self, loc: Loc) -> &mut Self::Output {
        let index = loc.to_index(self.w);
        return &mut self.tiles[index];
    }
}

// ModelStates
// Not an enum for <S> in Model definition
// allowing type implementations such as from(Model<Propogating>)
// pub trait ModelState {}
// the state after propogating but before collapsing a tile
// also the InitialState
// #[derive(Debug)]
// pub struct Collapsing {}
// impl ModelState for Collapsing {}
// pub type InitialState = Collapsing;
// propogating implications of collapsing a tile (tiles domains are being updated)
// #[derive(Debug)]
// pub struct Propogating {
//     stack: Vec<usize>,
// }
// impl ModelState for Propogating {}
// not going to waste space here explaining what state this represents /s
// #[derive(Debug)]
// pub struct Done {}
// impl ModelState for Done {}

#[derive(Debug)]
pub enum ModelStateEnum {
    Collapsing,
    Propogating { stack: Vec<Loc> },
    Done,
    Bad,
}
use ModelStateEnum::*;

#[derive(Debug)]
pub struct Model /*<S: ModelState>*/ {
    // pub state: S,
    pub state: ModelStateEnum,
    pub out_dims: Dimens,
    pub board: Board,
    pub probs: Rc<Vec<f32>>, // adj_mat: AdjacencyMatrix
}

impl Model {
    pub fn new(
        num_patterns: usize,
        out_dims: Dimens,
        prob_vec: Vec<f32>,
        pattern_vec: Vec<Image>,
    ) -> Model {
        let probs = Rc::new(prob_vec);
        let patterns = Rc::new(pattern_vec);
        let board = Board::new(num_patterns, &out_dims, &probs, &patterns);
        let state = Collapsing;
        return Model {
            out_dims,
            board,
            state,
            probs, // the strong owner of probs
        };
    }

    pub fn to_images(&self) -> Vec<(&Loc, &Image)> {
        return self
            .board
            .iter()
            .map(|tile| (&tile.loc, &tile.img))
            .collect();
    }

    pub fn step(&mut self) {
        match &self.state {
            Bad | Done => (),
            Collapsing => self.collapse(),
            Propogating { .. } => self.propogate(),
        }
    }

    pub fn propogate(&mut self) {
        self.state = Collapsing;
    }

    pub fn collapse(&mut self) {
        let min_ent_tile_loc = self.board.min_entropy_loc();
        match min_ent_tile_loc {
            Some(loc) => {
                let stack = vec![loc];
                self.state = Propogating { stack };
                self.board[loc].collapse(first_allowed_heuristic);
            }
            None => {
                self.state = Done;
            }
        }
    }
}

// enum MaybeModel<S: ModelState> {
//     Yes(Model<S>),
//     No(Model<Done>),
// }

fn first_allowed_heuristic(tile: &Tile) -> Option<usize> {
    let idx;
    for (i, &allowed) in (tile.dom).iter().enumerate() {
        if allowed {
            idx = i;
            return Some(idx);
        }
    }
    return None;
}

// collapse min entropy tile
// impl From<Model<Collapsing>> for MaybeModel<Propogating> {
//     fn from(model: Model<Collapsing>) -> MaybeModel<Propogating> {
//         let min_ent_tile_loc = model.min_nz_entropy();
//         let mayb_idx = Option::map(min_ent_tile_loc, |loc| loc.to_index(model.out_dims.x));
//         match mayb_idx {
//             Some(idx) => {
//                 let stack = vec![idx];
//                 let mut new_model = Model {
//                     state: Propogating { stack },
//                     board: model.board,
//                     probs: model.probs,
//                     out_dims: model.out_dims,
//                 };
//                 new_model.board[idx].collapse(first_allowed_heuristic);
//                 return MaybeModel::Yes(new_model);
//             }
//             None => {
//                 let new_model = Model {
//                     state: Done {},
//                     board: model.board,
//                     probs: model.probs,
//                     out_dims: model.out_dims,
//                 };
//                 return MaybeModel::No(new_model);
//             }
//         }
//     }
// }
