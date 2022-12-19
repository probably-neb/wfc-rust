pub mod adjacency_rules;
pub mod preprocessor;
pub mod wfc;
pub mod tile;


pub trait Area {
    type Output;
    fn area(&self) -> Self::Output;
}

impl Area for Grid {
    type Output = u32;

    fn area(&self) -> Self::Output {
        return self.x * self.y;
    }
}

use derive_more::{Deref, DerefMut, From};
use glam::UVec2;

#[derive(Deref, DerefMut, From, Clone, Debug, Default)]
pub struct Grid(pub UVec2);

impl Grid {
    pub fn iter_locs(&self) -> impl Iterator<Item = UVec2> {
        return UVec2Iter::new(UVec2::ZERO, self.0);
    }
}

#[derive(Clone, Debug)]
pub struct UVec2Iter {
    pub cur: UVec2,
    pub end: UVec2,
}

impl UVec2Iter {
    pub fn new(start: UVec2, end: UVec2) -> Self {
        return Self { cur: start, end };
    }
}

impl Iterator for UVec2Iter {
    type Item = UVec2;

    fn next(&mut self) -> Option<Self::Item> {
        let mut ret = Some(self.cur);
        if self.cur.x == self.end.x {
            self.cur.x = 0;
            self.cur.y += 1;
            ret = Some(self.cur);
        }
        if self.cur.y == self.end.y {
            ret = None
        } else {
            self.cur.x += 1;
        }
        return ret;
    }
}
