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

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn allow(&mut self, from: usize, to: usize, dir: CardinalDirs) {
        self.allow_one_way(from, to, dir);
        self.allow_one_way(to, from, -dir);
    }

    fn allow_one_way(&mut self, from: usize, to: usize, dir: CardinalDirs) {
        // self.try_add_new(from_);
        self.map.entry(from).or_insert_with(Default::default)[dir].insert(to);
        // self.map.get_mut(&from_).expect("from_ already in map")[dir].insert(to_);
    }

    pub fn is_allowed(&self, from: usize, to: usize, dir: CardinalDirs) -> bool {
        return self.map[&from][dir].contains(&to);
    }

    pub fn enabled_by(&self, from: TileId, dir: CardinalDirs) -> Vec<TileId> {
        return self.map[&from][dir].iter().copied().collect();
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
                        log::info!("tile {id} already removed");
                        None
                    },
                    // tile has enablers
                    Some(counts) => {
                        // count in opposite dir i.e. pointing towards removed_enabler
                        let count_in_dir = &mut counts[-dir];

                        log::info!("tile {id} not removed yet, decrementing count: {count_in_dir} in -{dir:?}");
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

    pub fn allowed_iter(&self) -> impl Iterator<Item = TileId> + '_ {
        return self
            .enablers
            .iter()
            .enumerate()
            .filter_map(|(idx, b)| b.map(|_| idx));
    }

    pub fn filter_allowed<'a, T>(&'a self, other: &'a Vec<T>) -> impl Iterator<Item = T> + '_
    where
        T: Clone,
    {
        assert_eq!(self.enablers.len(), other.len());
        return self.iter_allowed(other).flatten();
    }

    pub fn iter_allowed<'a, T>(&'a self, other: &'a Vec<T>) -> impl Iterator<Item = Option<T>> + '_
    where
        T: Clone,
    {
        assert_eq!(self.enablers.len(), other.len());
        return zip(&self.enablers, other).map(|(b, v)| b.map(|_| v.clone()));
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
    use super::*;
    use crate::simple_patterns::*;
    use glam::UVec4;
    use CardinalDirs::*;

    fn simple_patterns_common() -> AdjacencyRules {
        return construct_simple_patterns()
            .wfc_data
            .unwrap()
            .adjacency_rules;
    }

    #[test]
    fn blank_allowed_in_all_dirs() {
        let adj = simple_patterns_common();
        assert!(adj.allowed_in_all_dirs(BLANK, BLANK));
    }

    #[test]
    fn blank_allowed_to_the_right_of_blank_right() {
        let adj = simple_patterns_common();
        for br in BLANK_RIGHT {
            assert!(adj.is_allowed(br, BLANK, Right));
            assert!(adj.is_allowed(BLANK, br, Left));
        }
    }
    #[test]
    fn blank_allowed_to_the_left_of_blank_left() {
        let adj = simple_patterns_common();
        for bl in BLANK_LEFT {
            assert!(adj.is_allowed(bl, BLANK, Left));
            assert!(adj.is_allowed(BLANK, bl, Right));
        }
    }
    #[test]
    fn blank_allowed_below_blank_down() {
        let adj = simple_patterns_common();
        for bd in BLANK_DOWN {
            assert!(adj.is_allowed(bd, BLANK, Down));
            assert!(adj.is_allowed(BLANK, bd, Up));
        }
    }
    #[test]
    fn blank_allowed_above_blank_up() {
        let adj = simple_patterns_common();
        for bu in BLANK_UP {
            assert!(adj.is_allowed(bu, BLANK, Up));
            assert!(adj.is_allowed(BLANK, bu, Down));
        }
    }

    #[test]
    fn blank_not_allowed_to_the_left_of_blank_right() {
        let adj = simple_patterns_common();
        for br in BLANK_RIGHT {
            assert!(!adj.is_allowed(br, BLANK, Left));
            assert!(!adj.is_allowed(BLANK, br, Right));
        }
    }
    #[test]
    fn blank_not_allowed_to_the_right_of_blank_left() {
        let adj = simple_patterns_common();
        for bl in BLANK_LEFT {
            assert!(!adj.is_allowed(bl, BLANK, Right));
            assert!(!adj.is_allowed(BLANK, bl, Left));
        }
    }
    #[test]
    fn blank_not_allowed_above_blank_down() {
        let adj = simple_patterns_common();
        for bd in BLANK_DOWN {
            assert!(!adj.is_allowed(bd, BLANK, Up));
            assert!(!adj.is_allowed(BLANK, bd, Down));
        }
    }

    #[test]
    fn blank_not_allowed_below_blank_up() {
        let adj = simple_patterns_common();
        for bu in BLANK_UP {
            assert!(!adj.is_allowed(bu, BLANK, Down));
            assert!(!adj.is_allowed(BLANK, bu, Up));
        }
    }

    fn all_allowed(froms: [TileId; 2], tos: [TileId; 2], dir: CardinalDirs, adj: &AdjacencyRules) {
        for from in froms {
            for to in tos {
                assert!(adj.is_allowed(from,to,dir), "{} -> {:?} -> {} not allowed", CHARS[from], dir, CHARS[to]);
            }
        }
    }

    fn not_allowed(froms: [TileId; 2], tos: [TileId; 2], dir: CardinalDirs, adj: &AdjacencyRules) {
        for from in froms {
            for to in tos {
                assert!(!adj.is_allowed(from,to,dir), "{} -> {:?} -> {} allowed", CHARS[from], dir, CHARS[to]);
            }
        }
    }

    #[test]
    fn bl_allowed_left_of_br() {
        let adj = &simple_patterns_common();
        all_allowed(BLANK_RIGHT, BLANK_LEFT, Left, adj)
    }
    #[test]
    fn br_not_allowed_right_of_br() {
        let adj = &simple_patterns_common();
        not_allowed(BLANK_RIGHT, BLANK_RIGHT, Right, adj)
    }
    #[test]
    fn bl_not_allowed_left_of_bl() {
        let adj = &simple_patterns_common();
        not_allowed(BLANK_LEFT, BLANK_LEFT, Left, adj)
    }

    #[test]
    fn remove_all_but_one_enabler() {
        let adj = simple_patterns_common();
        let mut enab = EnablerDict::new(&adj);
        let mut removed_ids = enab.remove_all_but(BLANK);
        assert!(removed_ids.len() == 4);
        removed_ids.sort();
        assert!(removed_ids == vec![1, 2, 3, 4]);
    }

    fn usize4_to_vec4(arr: [usize; 4]) -> UVec4 {
        let [x, y, z, w] = arr;
        return UVec4::new(x as u32, y as u32, z as u32, w as u32);
    }

    fn enabler_counts_common() -> Vec<[usize;4]> {
        let adj = simple_patterns_common();
        let enab = EnablerDict::new(&adj);
        let counts: Vec<[usize; 4]> = enab
            .enablers
            .iter()
            .map(|opt_c| {
                opt_c.expect("All enabler counts should start out as Some(count)")
            })
            .collect();
        return counts;
    }
    #[test]
    fn blank_enabler_counts_all_3() {
        let counts = enabler_counts_common();
        assert!(usize4_to_vec4(counts[BLANK]) == UVec4::splat(3), "{:?} != {:?}", counts[BLANK], UVec4::splat(3));
    }
    fn connect_2_blank_3(id: TileId, connect_dirs: [CardinalDirs; 2], counts: &[[usize; 4]]) {
        let [dir1, dir2] = connect_dirs;
        for dir in [dir1, dir2] {
            assert!(counts[id][dir] == 2, "tile: {} ({}) -> {:?} -> count = {} != {}", CHARS[id],id,dir, counts[id][dir],2);
        }
        for dir in [-dir1, -dir2] {
            assert!(counts[id][dir] == 3, "tile: {} ({}) -> {:?} -> count = {} != {}", CHARS[id],id,dir, counts[id][dir],3);
        }
    }
    #[test]
    fn lu_c2b3() {
        let counts = enabler_counts_common();
        connect_2_blank_3(LU, [Left, Up], &counts);
    }
    #[test]
    fn rd_c2b3() {
        let counts = enabler_counts_common();
        connect_2_blank_3(RD, [Right, Down], &counts);
    }
    #[test]
    fn ur_c2b3() {
        let counts = enabler_counts_common();
        &counts;
        connect_2_blank_3(UR, [Right, Up], &counts);
    }
    #[test]
    fn dl_c2b3() {
        let counts = enabler_counts_common();
        connect_2_blank_3(DL, [Left, Down], &counts);
    }

    #[test]
    fn no_enabler_counts_are_zero() {
        let adj = simple_patterns_common();
        for id in 0..5 {
            let counts = adj.enabled_by_count(id);
            let v4 = usize4_to_vec4(counts);
            let b4 = v4.cmpeq(UVec4::ZERO);
            assert!(!b4.any());
        }
    }
}
