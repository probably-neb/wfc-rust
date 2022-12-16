use glam::{IVec2, UVec2};
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut, Index, IndexMut, Neg},
    slice::Iter,
};

#[derive(Debug, Default)]
pub struct AdjacencyRules {
    map: HashMap<usize, [HashSet<usize>; 4]>,
}

impl AdjacencyRules {
    pub fn new() -> Self {
        return Self::default();
    }
    /// adds new id if not already present
    fn try_add_new(&mut self, id: usize) {
        // TODO: map.try_insert (unstable feature)
        self.map.entry(id).or_insert_with(Default::default);
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
        counts[0] = by_dirs[0];
        counts[1] = by_dirs[1];
        counts[2] = by_dirs[2];
        counts[3] = by_dirs[3];
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

#[derive(Debug, Clone)]
pub struct EnablerDict(Vec<Enabled>);

impl Deref for EnablerDict {
    type Target = Vec<Enabled>;

    fn deref(&self) -> &Self::Target {
        return &self.0;
    }
}

impl DerefMut for EnablerDict {
    fn deref_mut(&mut self) -> &mut Self::Target {
        return &mut self.0;
    }
}

impl EnablerDict {
    pub fn new(adjacency_rules: &AdjacencyRules) -> Self {
        let enablers = adjacency_rules
            .map
            .keys()
            .map(|&id| adjacency_rules.enabled_by_count(id))
            .collect();
        return Self(enablers);
    }

    pub fn remove(
        &mut self,
        from: usize,
        cell_loc: UVec2,
        dir: CardinalDirs,
        adjacency_rules: &AdjacencyRules,
    ) -> Option<Vec<TileRemovalEvent>> {
        let enabled = adjacency_rules.enabled_by(from, dir);
        let events: Vec<TileRemovalEvent> = enabled
            .iter()
            .filter_map(|&id| {
                let count = self[id][dir];
                match count {
                    1 => {
                        // decrementing this tile would remove all enablers in a dir
                        // making the possibility of this tile impossible
                        // remove the tile completely
                        self[id] = [0; 4];
                        // save event for change propogation
                        Some(TileRemovalEvent {
                            tile_id: from,
                            cell_loc,
                        })
                    }
                    // event is already removed
                    // do nothing
                    0 => None,
                    // event has enablers
                    // decrement enabler count
                    c => {
                        self[id][dir] = c - 1;
                        None
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
