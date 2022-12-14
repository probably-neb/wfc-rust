use num_traits::{sign::Unsigned, PrimInt};
use std::{
    fmt::{Debug, Display},
    iter::repeat,
    ops::{Add, Neg, Sub},
    slice::Iter,
};

trait XY {
    type Output;
    fn x(&self) -> Self::Output;
    fn y(&self) -> Self::Output;
}

#[derive(Clone, Copy)]
pub struct Point<T: PrimInt> {
    pub x: T,
    pub y: T,
}

impl<T> Debug for Point<T>
where
    T: PrimInt + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = format!("<{},{}>", self.x, self.y);
        f.write_str(&str)
    }
}

impl<T> Display for Point<T>
where
    T: PrimInt + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = format!("<{},{}>", self.x, self.y);
        f.write_str(&str)
    }
}

pub type Loc = Point<usize>;
pub type Dir = Point<isize>;
type UDir = Point<i8>; // for use in unit dir. Not public
pub type Dimens = Point<usize>;

impl<I> XY for Point<I>
where
    I: PrimInt,
{
    type Output = I;

    fn x(&self) -> I {
        return self.x;
    }

    fn y(&self) -> I {
        return self.y;
    }
}

impl<I> Point<I>
where
    I: PrimInt,
{
    pub fn prod(&self) -> I {
        return self.x * self.y;
    }
}

impl<U> Point<U>
where
    U: PrimInt + Unsigned + Into<usize>,
{
    // TODO: get Into/From impl working to avoid this cheat

    pub fn prodvec<T>(&self) -> Vec<T> {
        return Vec::with_capacity(self.prod().try_into().unwrap());
    }

    fn u(&self) -> Point<usize> {
        let x: usize = self.x.try_into().unwrap();
        let y: usize = self.y.try_into().unwrap();
        return Point { x, y };
    }
}

impl Point<usize> {
    pub fn as_index(&self, w: usize) -> usize {
        return self.y * w + self.x;
    }

    pub fn ncoord_list(&self, n: usize) -> Vec<Loc> {
        // let uself = self.sub((1,1));
        let mut locs = self.prodvec();
        for row in (0..self.y).step_by(n) {
            for col in (0..self.x).step_by(n) {
                let loc = Loc { x: col, y: row };
                locs.push(loc);
            }
        }
        assert_eq!(locs.len(), self.x * self.y);
        return locs;
    }

    pub fn coord_list(&self) -> Vec<Loc> {
        return self.ncoord_list(1);
    }
}

impl Sub<(usize, usize)> for Point<usize> {
    type Output = Self;
    fn sub(self, rhs: (usize, usize)) -> Self::Output {
        return Point {
            x: self.x - rhs.0,
            y: self.y - rhs.1,
        };
    }
}

impl From<Point<i8>> for Point<isize> {
    fn from(p: Point<i8>) -> Self {
        return Point {
            x: p.x.into(),
            y: p.y.into(),
        };
    }
}

impl Loc {
    pub fn add_udir<SI>(&self, dir: Point<SI>) -> Option<Loc>
    where
        SI: num_traits::Signed + num_traits::PrimInt + Into<isize>,
    {
        // NOTE: range usize > range isize: could cause errors with sufficiently large locs
        // (extremely large)
        let ix: isize = self.x.try_into().ok()?;
        let iy: isize = self.y.try_into().ok()?;
        let x: usize = (ix + dir.x().into()).try_into().ok()?;
        let y: usize = (iy + dir.y().into()).try_into().ok()?;
        return Some(Loc { x, y });
    }

    pub fn add_udir_bounds<SI>(&self, dir: Point<SI>, bounds: Dimens) -> Option<Loc>
    where
        SI: num_traits::Signed + num_traits::PrimInt + Into<isize>,
    {
        return self
            .add_udir(dir)
            .filter(|loc| loc.x < bounds.x && loc.y < bounds.y);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CardinalDir {
    UP,
    DOWN,
    RIGHT,
    LEFT,
}

use CardinalDir::*;

impl CardinalDir {
    pub fn dir(&self) -> UDir {
        return match self {
            UP => UDir { x: 0, y: -1 },
            DOWN => UDir { x: 0, y: 1 },
            LEFT => UDir { x: -1, y: 0 },
            RIGHT => UDir { x: 1, y: 0 },
        };
    }

    pub fn idx(&self) -> usize {
        return match self {
            UP => 0,
            DOWN => 1,
            LEFT => 2,
            RIGHT => 3,
        };
    }

    pub fn iter(&self) -> Iter<CardinalDir> {
        return [UP, RIGHT, DOWN, LEFT].iter();
    }
}

/// Returns the opposite direction
impl Neg for CardinalDir {
    type Output = Self;
    fn neg(self) -> Self::Output {
        return match self {
            UP => DOWN,
            DOWN => UP,
            LEFT => RIGHT,
            RIGHT => LEFT,
        };
    }
}

pub const CARDINAL_DIRS: [CardinalDir; 4] = [UP, RIGHT, DOWN, LEFT];

impl XY for CardinalDir {
    type Output = i8;

    fn x(&self) -> i8 {
        return self.dir().x;
    }
    fn y(&self) -> i8 {
        return self.dir().y;
    }
}
