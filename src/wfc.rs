use crate::{adjacency_map, domain, entropy, image_utils, point};
use adjacency_map::AdjacencyMap;
use domain::Domain;
use entropy::{Entropy, Probability};
use image_utils::Merge;
use point::{CardinalDir, CardinalDir::*, Dimens, Loc, CARDINAL_DIRS};

use core::panic;
use macroquad::texture::Image;
use std::ops::{Index, IndexMut};
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
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
        let dom: Domain = Domain((0..dlen).map(|_| true).collect());
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
                Option::map(mayb_idx, |idx| {
                    self.dom.only(idx);
                });
                self.state = Collapsed;
                self.update_image();
            }
            Collapsed => panic!("Tried to Collapse collapsed Tile"),
        }
    }

    fn update_domain(&mut self, dom: Domain) -> bool {
        let changed = &dom == &self.dom;
        self.dom &= dom;
        return changed;
    }
}

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
            .coord_list()
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
            .filter(|tile| match tile.state {
                Collapsed => false,
                UnCollapsed => true,
            })
            .filter(|tile| tile.entropy() != 0.0)
            .min_by(|t1, t2| t1.entropy().total_cmp(&t2.entropy()))
            .map(|tile| tile.loc);
    }

    fn bounds(&self) -> Dimens {
        // NOTE: do I ever need it to not be as Dimens? maybe don't have w, h
        return Dimens{x:self.w, y: self.h};
    }

    pub fn get_adjacent_tile_locs(&self, loc: Loc) -> Vec<(CardinalDir, Loc)> {
        //TODO: find way to iter over enum itself
        return CARDINAL_DIRS
            .iter()
            .filter_map(|cdir| {
                loc.add_udir_bounds(cdir.dir(), self.bounds())
                    .map(|nloc| (*cdir, nloc))
            })
            .collect();
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
        let index = loc.as_index(self.w);
        return &self[index];
    }
}

impl IndexMut<Loc> for Board {
    fn index_mut(&mut self, loc: Loc) -> &mut Self::Output {
        let index = loc.as_index(self.w);
        return &mut self.tiles[index];
    }
}

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
    pub adj_map: AdjacencyMap<Vec<Domain>>,
}

impl Model {
    pub fn new(
        num_patterns: usize,
        out_dims: Dimens,
        prob_vec: Vec<f32>,
        pattern_vec: Vec<Image>,
        adj_map: AdjacencyMap<Vec<Domain>>,
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
            adj_map,
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
            Collapsing => self.collapse_min_entropy_tile(),
            Propogating { .. } => self.propogate(),
        }
    }

    pub fn propogate(&mut self) {
        match &mut self.state {
            Propogating { stack } => match &mut stack.pop() {
                Some(loc) => {
                    let adjacents = self.board.get_adjacent_tile_locs(*loc);
                    println!("Loc: {:?} | Adjacents: {:?}", loc, adjacents);
                    // TODO: create method to return domain in all directions as adjacency_map and
                    // iterate over that instead
                    for (dir, adj_loc) in adjacents {
                        // the domain of stack_tile towards adjacent_tile
                        let stack_tile_dom = &self.board[*loc].dom;
                        println!("STACK TILE DOM: {:?}", stack_tile_dom);
                        let dom_towards_adjacent_tile =
                            self.adj_map.domain_in_dir(dir, stack_tile_dom);
                        // println!("Dom in dir: {:?} = {:?}", dir,dom_towards_adjacent_tile);

                        let dom_changed = {
                            println!("getting dom of tile at {:?}", adj_loc);
                            let adjacent_tile_dom = &self.board[adj_loc].dom;
                            &dom_towards_adjacent_tile != adjacent_tile_dom
                        };

                        if dom_changed {
                            // update adjacent_tiles domain and push it too the stack
                            let adjacent_tile = &mut self.board[adj_loc];
                            // println!("Updating tile at {:?}", adj_loc);
                            adjacent_tile.update_domain(dom_towards_adjacent_tile);
                            // println!("new domain: {:?}", adjacent_tile.dom);
                            adjacent_tile.update_image();
                            // stack.push(adj_loc);
                        }
                    }
                }
                None => self.state = Collapsing,
            },
            _ => panic!("Tried to propogate non propogating model"),
        }
    }

    pub fn collapse_min_entropy_tile(&mut self) {
        let min_ent_tile_loc = self.board.min_entropy_loc();
        match min_ent_tile_loc {
            Some(loc) => {
                println!("min ent tile state: {:?}", self.board[loc].state);
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
