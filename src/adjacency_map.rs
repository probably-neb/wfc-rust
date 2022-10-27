use std::{
    ops::{Index, IndexMut},
    process::Output,
};

use crate::{domain::Domain, point::{CardinalDir, CARDINAL_DIRS}};

#[derive(Debug)]
pub struct AdjacencyMap<M>([M; 4]);

impl AdjacencyMap<Domain> {
    pub fn new(vec_size: usize) -> Self {
        let arr = [
            Domain::new(vec_size),
            Domain::new(vec_size),
            Domain::new(vec_size),
            Domain::new(vec_size),
        ];
        return AdjacencyMap(arr);
    }

    pub fn empty() -> AdjacencyMap<Domain> {
        let vec = vec![Domain::default(); 4];
        return match vec.try_into().ok() {
            Some(dom) => AdjacencyMap(dom),
            None => panic!("cannot create domain"),
        };
    }

    pub fn from_tup_array(arr: [(CardinalDir, [bool; 5]); 4]) -> Self {
        let mut adj_map = Self::empty();
        for tup in arr {
            adj_map[tup.0] = Domain(tup.1.to_vec());
        }
        return adj_map;
    }
}

impl AdjacencyMap<Vec<Domain>> {
    pub fn empty() -> AdjacencyMap<Vec<Domain>> {
        let arr = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        return AdjacencyMap(arr);
    }

    pub fn from_individual_adj_maps(maps: Vec<(usize, AdjacencyMap<Domain>)>) -> Self {
        let vec_size = maps.len();
        let vec = vec![Domain::cont(1); vec_size];
        let arr = [
            vec.clone(),
            vec.clone(),
            vec.clone(),
            vec
        ];
        let mut adj_map = AdjacencyMap(arr);

        for (id,map) in maps {
            for dir in CARDINAL_DIRS {
                adj_map[dir][id] = map[dir].clone();
            }
        }
        return adj_map;
    }

    pub fn domain_in_dir(&self, dir: CardinalDir, filter: &Domain) -> Domain {
        let vec = Domain::filter(filter, &self[dir]);
        return Domain::andv(&vec);
    }
}

impl<M> Index<usize> for AdjacencyMap<M> {
    type Output = M;
    fn index(&self, idx: usize) -> &Self::Output {
        return &self.0[idx];
    }
}

impl<M> Index<CardinalDir> for AdjacencyMap<M> {
    type Output = M;
    fn index(&self, dir: CardinalDir) -> &Self::Output {
        return &self[dir.idx()];
    }
}

impl<M> IndexMut<usize> for AdjacencyMap<M> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        return &mut self.0[idx];
    }
}

impl<M> IndexMut<CardinalDir> for AdjacencyMap<M> {
    fn index_mut(&mut self, dir: CardinalDir) -> &mut Self::Output {
        return &mut self[dir.idx()];
    }
}
