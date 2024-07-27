use glam::IVec2;
use std::{
    collections::{HashMap, HashSet},
    iter::zip,
    ops::{Add, Index, IndexMut, Neg},
    slice::Iter,
};

use crate::tile::{IdMap, TileId};

#[derive(Debug, Default, Clone)]
pub struct AdjacencyRules {
    map: HashMap<usize, [HashSet<usize>; 4]>,
}

impl AdjacencyRules {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn len(&self) -> usize {
        return self.map.len();
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn allow(&mut self, from: usize, to: usize, dir: CardinalDirs) {
        self.allow_one_way(from, to, dir);
        self.allow_one_way(to, from, -dir);
    }

    fn allow_one_way(&mut self, from: usize, to: usize, dir: CardinalDirs) {
        self.map.entry(from).or_insert_with(Default::default)[dir].insert(to);
        log::trace!("allowing {from} -> {dir:?} -> {to}");
    }

    pub fn is_allowed(&self, from: usize, to: usize, dir: CardinalDirs) -> bool {
        return self.map[&from][dir].contains(&to);
    }

    // PERF: remove copy (caller should copy if needed)
    pub fn enabled_by(&self, from: TileId, dir: CardinalDirs) -> Vec<TileId> {
        match self.map.get(&from) {
            Some(allowed_adjacents) => allowed_adjacents[dir].iter().copied().collect(),
            None => panic!("no tile entry for tile {from}")
        }
    }

    pub fn maybe_enabled_by(&self, from: TileId, dir: CardinalDirs) -> Option<&HashSet<TileId>> {
        return self.map.get(&from).map(|allowed_adjacents| &allowed_adjacents[dir]);
    }

    fn enabled_by_count(&self, from: usize) -> [usize; 4] {
        let mut by_dirs: [usize; 4] = [0,0,0,0]; 
        for &dir in CardinalDirs::iter() {
            by_dirs[dir] = self.enabled_by(from, dir).len();
        }
        return by_dirs;
    }

    #[cfg(test)]
    pub fn allowed_in_all_dirs(&self, from_: usize, to_: usize) -> bool {
        return CardinalDirs::iter()
            .map(|&dir| self.is_allowed(from_, to_, dir))
            .all(|b| b);
    }
}

type Enabled = [usize; 4];

#[derive(Debug, Clone)]
pub struct EnablerDict {
    enablers: IdMap<Option<Enabled>>,
}

impl EnablerDict {
    pub fn new(adjacency_rules: &AdjacencyRules) -> Self {
        let len = adjacency_rules.map.len();
        let mut enablers: IdMap<Option<Enabled>> = vec![Default::default(); len];
        for (id,enabler_counts) in enablers.iter_mut().enumerate() {
            let counts = adjacency_rules.enabled_by_count(id);
            *enabler_counts = Some(counts);
        }
        return Self { enablers };
    }

    pub fn remove_single(
        &mut self,
        // the tile id of the enabler that may or may not
        // have been enabling some of our tiles
        removed_enabler: TileId,
        // the dir from removed_enabler to us
        dir: CardinalDirs,
        adjacency_rules: &AdjacencyRules,
    ) -> Option<Vec<TileId>> {
        // enabled is the list of tiles the removed_enabler enables pointing towards us
        let enabled_by_enabler: Vec<TileId> = adjacency_rules.enabled_by(removed_enabler, dir);
        let removed_tiles: Vec<TileId> = enabled_by_enabler
            .iter()
            .filter_map(|&id| {
                match &mut self.enablers[id] {
                    // tile is already removed
                    // do nothing
                    None => {
                        log::trace!("tile {id} already removed");
                        None
                    },
                    // tile has enablers
                    Some(counts) => {
                        // count in opposite dir i.e. pointing towards removed_enabler
                        let count_in_dir = &mut counts[-dir];

                        log::trace!("tile {id} not removed yet, decrementing count: {count_in_dir} in -{dir:?}");
                        assert!(count_in_dir != &0);
                        // decrementing this tile would remove all enablers in a dir
                        // making the this tile impossible
                        // i.e. this tile is incompatible with one of the neighboring cells
                        // enabled tiles
                        if count_in_dir == &1 {
                            // remove (disable) the tile completely
                            self.enablers[id] = None;
                            // save event for change propogation
                            Some(id)
                        } else {
                            // decrement other count
                            *count_in_dir -= 1;
                            // no tile TileRemovalEvent
                            None
                        }
                    }
                }
            })
            .collect();
        if removed_tiles.is_empty() {
            return None;
        } else {
            return Some(removed_tiles);
        }
    }

    // TODO: remove unused functions and make sure the ones that are used 
    // aren't doing unnecessary clones

    pub fn allowed_tile_ids(&self) -> impl Iterator<Item = TileId> + '_ {
        return self
            .enablers
            .iter()
            .enumerate()
            .filter_map(|(idx, b)| b.map(|_| idx));
    }

    pub fn filter_allowed<'a, T>(&'a self, other: &'a Vec<T>) -> impl Iterator<Item = &'a T> + '_
    where
        T: Clone,
    {
        assert_eq!(self.enablers.len(), other.len());
        return self.iter_allowed(other).flatten();
    }

    pub fn iter_allowed<'a, T>(&'a self, other: &'a Vec<T>) -> impl Iterator<Item = Option<&'a T>> + '_
    where
        T: Clone,
    {
        assert_eq!(self.enablers.len(), other.len());
        return zip(&self.enablers, other).map(|(b, v)| b.map(|_| v));
    }

    pub fn filter_allowed_enumerate<'a, T>(
        &'a self,
        other: &'a Vec<T>,
    ) -> impl Iterator<Item = (TileId, T)> + '_
    where
        T: Clone,
    {
        assert_eq!(self.enablers.len(), other.len());
        return zip(&self.enablers, other)
            .enumerate()
            .filter_map(|(i, (&b, v))| b.map(|_| (i, v.clone())));
    }


    /// Remove all enabled/allowable/possible tiles except one (the lone survivor!)
    pub fn remove_all_but(&mut self, marcus_luttrell: TileId) -> Vec<TileId> {
        let mut removed_tile_ids = Vec::new();
        for (id, b) in self.enablers.iter_mut().enumerate() {
            if id == marcus_luttrell {
                if b.is_none() {
                    unreachable!(
                        "Contradiction: lone survivor Tile `{}` already not allowed",
                        id
                    );
                }
            } else if b.is_some() {
                *b = None;
                removed_tile_ids.push(id);
            }
        }
        return removed_tile_ids;
    }
}
#[derive(Debug, Clone, Copy)]
pub enum CardinalDirs {
    Up,
    Left,
    Right,
    Down,
}

impl CardinalDirs {
    pub fn as_array() -> [Self; 4] {
        return [Self::Up, Self::Left, Self::Down, Self::Right];
    }
    pub fn as_uvec_array() -> [Self; 4] {
        return [Self::Up, Self::Left, Self::Down, Self::Right];
    }
    pub fn iter() -> Iter<'static, Self> {
        return [Self::Up, Self::Left, Self::Down, Self::Right].iter();
    }
}

impl Neg for CardinalDirs {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            CardinalDirs::Up => Self::Down,
            CardinalDirs::Left => Self::Right,
            CardinalDirs::Right => Self::Left,
            CardinalDirs::Down => Self::Up,
        }
    }
}

impl Add<IVec2> for CardinalDirs {
    type Output = IVec2;

    fn add(self, rhs: IVec2) -> Self::Output {
        let dir: IVec2 = self.into();
        return rhs + dir;
    }
}

impl From<CardinalDirs> for usize {
    fn from(value: CardinalDirs) -> Self {
        match value {
            CardinalDirs::Up => 0,
            CardinalDirs::Left => 1,
            CardinalDirs::Down => 2,
            CardinalDirs::Right => 3,
        }
    }
}

impl From<usize> for CardinalDirs {
    fn from(value: usize) -> Self {
        match value {
            0 => CardinalDirs::Up,
            1 => CardinalDirs::Left,
            2 => CardinalDirs::Down,
            3 => CardinalDirs::Right,
            _ => panic!("Invalid usize for CardinalDirs"),
        }
    }
}


impl From<CardinalDirs> for IVec2 {
    fn from(value: CardinalDirs) -> Self {
        match value {
            CardinalDirs::Up => IVec2 { x: 0, y: -1 },
            CardinalDirs::Left => IVec2 { x: -1, y: 0 },
            CardinalDirs::Right => IVec2 { x: 1, y: 0 },
            CardinalDirs::Down => IVec2 { x: 0, y: 1 },
        }
    }
}

impl<T> Index<CardinalDirs> for [T; 4] {
    type Output = T;

    fn index(&self, index: CardinalDirs) -> &Self::Output {
        let index: usize = index.into();
        return &self[index];
    }
}

impl<T> IndexMut<CardinalDirs> for [T; 4] {
    fn index_mut(&mut self, index: CardinalDirs) -> &mut Self::Output {
        let index: usize = index.into();
        return &mut self[index];
    }
}

#[cfg(test)]
mod test {
}
