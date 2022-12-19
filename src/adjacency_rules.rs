use glam::{IVec2, UVec2};
use std::{
    collections::{HashMap, HashSet},
    iter::zip,
    ops::{Index, IndexMut, Neg},
    slice::Iter,
};

use crate::tile::{IdMap, TileId};
use derive_more::{Deref, DerefMut};

#[derive(Debug, Default, DerefMut, Deref)]
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

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn allow_bi(&mut self, from: usize, to: usize, dir: CardinalDirs) {
        self.allow(from, to, dir);
        self.allow(to, from, -dir);
    }

    pub fn allow(&mut self, from_: usize, to_: usize, dir: CardinalDirs) {
        // self.try_add_new(from_);
        self.map.entry(from_).or_insert_with(Default::default)[dir].insert(to_);
        // self.map.get_mut(&from_).expect("from_ already in map")[dir].insert(to_);
    }

    pub fn is_allowed(&self, tile_dom_in: usize, q: usize, dir: CardinalDirs) -> bool {
        return self.map[&tile_dom_in][dir].contains(&q);
    }

    fn enabled_by(&self, from: usize, dir: CardinalDirs) -> Vec<usize> {
        return self.map[&from][dir].iter().copied().collect();
    }

    fn enabled_by_count(&self, from: usize) -> [usize; 4] {
        let mut counts = [0; 4];
        let by_dirs: Vec<usize> = CardinalDirs::iter()
            .map(|&d| self.enabled_by(from, d).len())
            .collect();
        counts.copy_from_slice(&by_dirs);
        return counts;
    }

    #[cfg(test)]
    pub fn allowed_in_all_dirs(&self, from_: usize, to_: usize) -> bool {
        return CardinalDirs::iter()
            .map(|&dir| self.is_allowed(from_, to_, dir))
            .all(|b| b);
    }
}

type Enabled = [usize; 4];

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct EnablerDict{ 
    #[deref_mut]
    #[deref]
    enablers: IdMap<Option<Enabled>>,
    loc: UVec2, 
}

impl EnablerDict {
    pub fn build(adjacency_rules: &AdjacencyRules) -> Self {
        let enablers: IdMap<Option<Enabled>> = adjacency_rules
            .map
            .keys()
            .map(|&id| match adjacency_rules.enabled_by_count(id) {
                [0, 0, 0, 0] => None,
                counts => Some(counts),
            })
            .collect();
        let loc = UVec2::default();
        return Self{ enablers, loc};
    }

    pub fn with_loc(&self, loc: UVec2) -> Self {
        return Self {enablers: self.enablers.to_owned(), loc};
    }

    pub fn remove_single(
        &mut self,
        from: TileId,
        dir: CardinalDirs,
        adjacency_rules: &AdjacencyRules,
    ) -> Option<Vec<TileRemovalEvent>> {
        let enabled = adjacency_rules.enabled_by(from, dir);
        let events: Vec<TileRemovalEvent> = enabled
            .iter()
            .filter_map(|&id| {
                match self[id] {
                    // event is already removed
                    // do nothing
                    None => None,
                    // event has enablers
                    Some(mut counts) => {
                        let count_in_dir = counts[dir];
                        match count_in_dir {
                            // decrementing this tile would remove all enablers in a dir
                            // making the this tile impossible
                            // i.e. this tile is incompatible with one of the neighboring cells
                            // enabled tiles
                            1 => {
                                // remove (disable) the tile completely
                                self[id] = None;
                                // save event for change propogation
                                Some(TileRemovalEvent {
                                    tile_id: from,
                                    cell_loc: self.loc,
                                })
                            }
                            // some other count
                            _other_count => {
                                // decrement other count
                                counts[dir] -= 1;
                                // no tile TileRemovalEvent
                                None
                            }
                        }
                    }
                }
            })
            .collect();
        if events.is_empty() {
            return None;
        } else {
            return Some(events);
        }
    }

    pub fn allowed_iter(&self) -> impl Iterator<Item = TileId> + '_ {
        return self
            .iter()
            .enumerate()
            .filter_map(|(idx, b)| b.map(|_| idx));
    }

    pub fn filter_allowed<'a, T>(&'a self, other: &'a Vec<T>) -> impl Iterator<Item = T> + '_
    where
        T: Clone,
    {
        assert_eq!(self.len(), other.len());
        return self.iter_allowed(other).flatten();
    }

    pub fn iter_allowed<'a, T>(&'a self, other: &'a Vec<T>) -> impl Iterator<Item = Option<T>> + '_
    where
        T: Clone,
    {
        assert_eq!(self.len(), other.len());
        return zip(&self.enablers, other).map(|(b, v)| b.map(|_| v.clone()));
    }

    pub fn filter_allowed_enumerate<'a, T>(
        &'a self,
        other: &'a Vec<T>,
    ) -> impl Iterator<Item = (TileId, T)> + '_
    where
        T: Clone,
    {
        assert_eq!(self.len(), other.len());
        return zip(&self.enablers, other)
            .enumerate()
            .filter_map(|(i, (&b, v))| b.map(|_| (i, v.clone())));
    }

    /// Remove all enabled/allowable/possible tiles except one (the lone survivor!)
    pub fn remove_all_but(&mut self, marcus_luttrell: TileId) -> Vec<TileRemovalEvent> {
        let mut events = Vec::new();
        let cell_loc = self.loc;
        for (i, b) in self.iter_mut().enumerate() {
            if i == marcus_luttrell {
                if b.is_none() {
                    unreachable!(
                        "Contradiction: lone survivor Tile `{}` already not allowed",
                        i
                    );
                }
            } else if b.is_some() {
                *b = None;
                events.push(TileRemovalEvent {tile_id: i, cell_loc});
            }
        }
        return events;
    }
}

#[derive(Debug, Clone)]
pub struct TileRemovalEvent {
    pub tile_id: usize,
    pub cell_loc: UVec2,
}

#[derive(Debug, Clone, Copy)]
pub enum CardinalDirs {
    Up,
    Left,
    Right,
    Down,
}

impl CardinalDirs {
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

impl From<CardinalDirs> for usize {
    fn from(value: CardinalDirs) -> Self {
        match value {
            CardinalDirs::Up => 0,
            CardinalDirs::Left => 1,
            CardinalDirs::Right => 2,
            CardinalDirs::Down => 3,
        }
    }
}

impl From<CardinalDirs> for IVec2 {
    fn from(value: CardinalDirs) -> Self {
        match value {
            CardinalDirs::Up => IVec2 { x: 0, y: 1 },
            CardinalDirs::Left => IVec2 { x: -1, y: 0 },
            CardinalDirs::Right => IVec2 { x: 1, y: 0 },
            CardinalDirs::Down => IVec2 { x: 0, y: -1 },
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

// pub struct Domain(IdMap<bool>);
// impl Domain {
//     fn new(len: usize) -> Self {
//         return Self(vec![true; len]);
//     }
//     pub fn allowed_iter(&self) -> impl Iterator<Item = TileId> + '_ {
//         return self
//             .iter()
//             .enumerate()
//             .filter_map(|(idx, &b)| if b { Some(idx) } else { None });
//     }
//     pub fn filter_allowed<'a, T>(&'a self, other: &'a Vec<T>) -> impl Iterator<Item = T> + '_
//     where
//         T: Clone,
//     {
//         assert_eq!(self.len(), other.len());
//         return zip(&self.0, other).filter_map(|(&b, v)| if b { Some(v.clone()) } else { None });
//     }
//     pub fn filter_allowed_enumerate<'a, T>(
//         &'a self,
//         other: &'a Vec<T>,
//     ) -> impl Iterator<Item = (TileId, T)> + '_
//     where
//         T: Clone,
//     {
//         assert_eq!(self.len(), other.len());
//         return zip(&self.0, other).enumerate().filter_map(|(i, (&b, v))| {
//             if b {
//                 Some((i, v.clone()))
//             } else {
//                 None
//             }
//         });
//     }
//     pub fn disallow_all_but(&mut self, exception: TileId) {
//         for (i, b) in self.iter_mut().enumerate() {
//             if i == exception {
//                 if !*b {
//                     unreachable!("Contradiction: TileId `{}` already not allowed", i);
//                 }
//                 // *b = true;
//             } else {
//                 *b = false;
//             }
//         }
//     }
// }
