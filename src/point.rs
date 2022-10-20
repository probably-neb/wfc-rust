use std::{ops, option::Iter, iter::repeat};
use num_traits::{PrimInt,sign::Unsigned};

#[derive(Debug,Clone,Copy)]
pub struct Point<T: PrimInt> {
    pub x: T,
    pub y: T,
}

pub type Loc = Point<usize>;
pub type Dir = Point<isize>;
pub type Dimens = Point<usize>;
pub type UnitDir = Point<i8>;

// trait ReplicatExt: Iterator {
//     fn replicatn(self, n: usize, len: usize) -> Self
//         where Self: std::marker::Sized + std::clone::Clone,
//         {
//             return repeat(self)
//                 .take(n*len);
//         }
//     fn replicat(self, len: usize) -> Self
//         where Self: std::marker::Sized + std::clone::Clone,
//         {
//             return self.replicatn(1, len);
//         }
// }

// impl<I: Iterator> ReplicatExt for I {}

impl<I> Point<I>
where
    I: PrimInt + Into<usize>,
{
    pub fn prod(&self) -> I {
        return self.x * self.y;
    }

    pub fn prodvec<T>(&self) -> Vec<T> {
        return Vec::with_capacity(self.prod().try_into().unwrap());
    }

}

// impl<I: PrimInt + Into<usize>> From<Point<I>> for Point<usize> {
//     fn from(p: Point<I>) -> Point<usize> {
//         return Point{x: p.x.try_into().unwrap(),y: p.y.try_into().unwrap()};
//     }
// }

impl<U> Point<U>
where 
    U: PrimInt + Unsigned + Into<usize>,
{
    // TODO: get Into/From impl working to avoid this cheat
    fn u(&self) -> Point<usize> {
        let x: usize = self.x.try_into().unwrap();
        let y: usize = self.y.try_into().unwrap();
        return Point{x,y};
    }

    pub fn to_ncoord_list(&self, n: usize) -> Vec<Loc> {
        let uself = self.u();
        let mut locs = self.prodvec();
        for row in (0..uself.y).step_by(n) {
            for col in (0..uself.x).step_by(n) {
                let loc = Loc{x:col,y:row};
                locs.push(loc);
            }
        }
        return locs;
    }

    pub fn to_coord_list(&self) -> Vec<Loc> {
        return self.to_ncoord_list(1);
    }
}

impl Point<usize> {
    pub fn to_index(&self, w: usize) -> usize {
        return self.y * w + self.x;
    }
}

pub const UP: UnitDir = UnitDir { x: 0, y: -1 };
pub const DOWN: UnitDir = UnitDir { x: 0, y: 1 };
pub const LEFT: UnitDir = UnitDir { x: -1, y: 0 };
pub const RIGHT: UnitDir = UnitDir { x: 1, y: 0 };

pub const CARDINALDIRS: [UnitDir; 4] = [UP, DOWN, LEFT, RIGHT];
