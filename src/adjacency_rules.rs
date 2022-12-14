use glam::IVec2;
use std::{collections::{HashMap,HashSet}, ops::{Index, IndexMut}, slice::Iter};


#[derive(Debug, Default)]
pub struct AdjacencyRules {
    map: HashMap<usize, [HashSet<usize>;4]>
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

    pub fn allow(&mut self, from_: usize, to_: usize, dir: CardinalDirs) {
        // self.try_add_new(from_);
        self.map.entry(from_)
            .or_insert_with(Default::default)
            [dir].insert(to_);
        // self.map.get_mut(&from_).expect("from_ already in map")[dir].insert(to_);
    }

    pub fn is_allowed(&self, from_: usize, to_:usize, dir: CardinalDirs) -> bool {
        return self.map[&from_][dir].contains(&to_);
    }

    #[cfg(test)]
    pub fn allowed_in_all_dirs(&self, from_: usize, to_: usize) -> bool {
        return CardinalDirs::iter().map(|&dir| self.is_allowed(from_, to_, dir)).all(|b| b);
    }
}

#[derive(Debug,Clone,Copy)]
pub enum CardinalDirs {
    UP,
    LEFT,
    RIGHT,
    DOWN,
}

impl CardinalDirs {
    pub fn iter() -> Iter<'static, Self> {
        return [Self::UP, Self::LEFT, Self::DOWN, Self::RIGHT].iter();
    }
}

impl From<CardinalDirs> for usize {
    fn from(value: CardinalDirs) -> Self {
        match value {
            CardinalDirs::UP => 0,
            CardinalDirs::LEFT => 1,
            CardinalDirs::RIGHT => 2,
            CardinalDirs::DOWN => 3,
        }
    }
}

impl From<CardinalDirs> for IVec2 {
    fn from(value: CardinalDirs) -> Self {
        match value {
            UP => IVec2 {x:0,y:1},
            LEFT => IVec2 {x:-1,y:0},
            RIGHT => IVec2 {x:1,y:0},
            DOWN => IVec2 {x:0,y:-1},
        }
    }
}

impl<T> Index<CardinalDirs> for [T;4] {
    type Output = T;

    fn index(&self, index: CardinalDirs) -> &Self::Output {
        let index: usize = index.into();
        return &self[index];
    }
}

impl<T> IndexMut<CardinalDirs> for [T;4] {
    fn index_mut(&mut self, index: CardinalDirs) -> &mut Self::Output {
        let index: usize = index.into();
        return &mut self[index];
    }
}
