use crate::image_utils::Merge;
use crate::point;
use crate::Image;
use point::{Dimens, Dir, Loc, Point};
use std::rc::{Rc, Weak};

pub type IdVec = Vec<bool>;

#[derive(Debug, Clone)]
pub struct Tile {
    pub loc: Loc,
    pub dom: Vec<bool>,
    pub probs: Rc<Vec<Probability>>,
    pub patterns: Rc<Vec<Image>>,
    pub img: Image,
}

#[derive(Debug)]
pub struct Model /*<S: ModelState>*/ {
    // pub state: S,
    pub state: ModelStateEnum,
    pub out_dims: Dimens,
    pub board: Vec<Tile>,
    pub probs: Rc<Vec<f32>>, // adj_mat: AdjacencyMatrix
}

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
// fn fst<A, B>(tup: (A, B)) -> A {
//     return tup.0;
// }

// fn snd<A, B>(tup: (&A, &B)) -> B {
//     return *tup.1;
// }

// fn mapfst<A, B, C>(tup: (A, B), f: fn(A) -> C) -> (C, B) {
//     return (f(tup.0), tup.1);
// }
// fn mapsnd<A, B, C>(tup: (A, B), f: fn(B) -> C) -> (A, C) {
//     return (tup.0, f(tup.1));
// }
//
// fn merge_images(dest: &mut Image, images: &Vec<Image>) {
//     let merged = images.iter().reduce(|a, b| &Image::merge(a, &b));
//     merged.map(|img| {
//         dest.width = img.width;
//         dest.height = img.height;
//         dest.bytes = img.bytes.clone();
//     });
// }

impl Tile {
    fn new(loc: Loc, dlen: usize, probs: Rc<Vec<Probability>>, patterns: Rc<Vec<Image>>) -> Self {
        let dom: IdVec = (0..dlen).map(|_| true).collect();
        let img = Image::empty();
        let mut tile = Tile {
            loc,
            dom,
            probs,
            patterns,
            img,
        };
        tile.update_image();
        return tile;
    }

    fn update_image(&mut self) {
        // PERF: don't clone all images
        let mut possible_pics = self
            .dom
            .iter()
            .zip(self.patterns.iter())
            .filter(|(&in_domain, _)| in_domain)
            .map(|(_, pic)| pic);
        if self.entropy() == 0.0 {
            match possible_pics.next() {
                Some(img) => self.img = img.clone(),
                None => return
            }
        } else {
            possible_pics.fold(&mut self.img, |p1, p2| {
                Image::merge_mut(p1, p2);
                return p1;
            });
        }
    }

    fn collapse(&mut self, heuristic: fn(&Tile) -> Option<usize>) {
        let mayb_idx = heuristic(self);
        self.dom.fill(false);
        Option::map(mayb_idx, |idx| {
            self.dom[idx] = true;
        });
        self.update_image();
    }
}

type AdjacencyMatrix = Vec<[Vec<bool>; 4]>;

trait Board {
    fn new(
        num_patterns: usize,
        dimensions: &Dimens,
        probs: &Rc<Vec<Probability>>,
        patterns: &Rc<Vec<Image>>,
    ) -> Self;
}

impl Board for Vec<Tile> {
    fn new(
        num_patterns: usize,
        dimensions: &Dimens,
        probs: &Rc<Vec<Probability>>,
        patterns: &Rc<Vec<Image>>,
    ) -> Self {
        return dimensions
            .to_coord_list()
            .into_iter()
            .map(|loc| Tile::new(loc, num_patterns, probs.clone(), patterns.clone()))
            .collect();
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
    Propogating { stack: Vec<usize> },
    Done,
    Bad,
}

// impl ModelState for ModelStateEnum {}

impl Model {
    pub fn new(
        n: u16,
        num_patterns: usize,
        out_dims: Dimens,
        prob_vec: Vec<f32>,
        pattern_vec: Vec<Image>,
    ) -> Model {
        let probs = Rc::new(prob_vec);
        let patterns = Rc::new(pattern_vec);
        let board = Board::new(num_patterns, &out_dims, &probs, &patterns);
        let state = ModelStateEnum::Collapsing;
        return Model {
            out_dims,
            board,
            state,
            probs, // the strong owner of probs
        };
    }

    pub fn tile_at_mut(&mut self, loc: Loc) -> &mut Tile {
        let idx = self.out_dims.x * loc.y + loc.x;
        return &mut self.board[idx];
    }

    pub fn iter_tiles(&self) -> std::slice::Iter<Tile> {
        return self.board.iter();
    }

    pub fn min_nz_entropy(&self) -> Option<Loc> {
        // TODO: get list of min entropy tiles and choose random for less predictable output
        // PERF: don't call entropy on tile so many times
        return self
            .iter_tiles()
            .filter(|tile| tile.entropy() != 0.0)
            .min_by(|t1, t2| t1.entropy().total_cmp(&t2.entropy()))
            .map(|tile| tile.loc);
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
            ModelStateEnum::Bad | ModelStateEnum::Done => (),
            ModelStateEnum::Collapsing => self.collapse(),
            ModelStateEnum::Propogating { .. } => self.propogate(),
        }
    }

    pub fn propogate(&mut self) {
        self.state = ModelStateEnum::Collapsing;
    }

    pub fn collapse(&mut self) {
        let min_ent_tile_loc = self.min_nz_entropy();
        let mayb_idx = Option::map(min_ent_tile_loc, |loc| loc.to_index(self.out_dims.x));
        match mayb_idx {
            Some(idx) => {
                let stack = vec![idx];
                self.state = ModelStateEnum::Propogating { stack };
                self.board[idx].collapse(first_allowed_heuristic);
            }
            None => {
                self.state = ModelStateEnum::Done;
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
