use std::ops::{Index, IndexMut};

use glam::UVec2;

#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct UVecVec<T>(pub Vec<Vec<T>>);

impl<T> Index<UVec2> for UVecVec<T> {
    type Output = T;
    fn index(&self, index: UVec2) -> &Self::Output {
        return &self.0[index.y as usize][index.x as usize]
    }
}

impl<T> IndexMut<UVec2> for UVecVec<T> {
    fn index_mut(&mut self, index: UVec2) -> &mut Self::Output {
        return &mut self.0[index.y as usize][index.x as usize]
    }
}

