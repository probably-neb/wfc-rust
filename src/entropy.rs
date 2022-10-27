use crate::wfc::Tile;

pub type Probability = f32;

pub trait Entropy {
    fn entropy(&self) -> f32;
}

impl Entropy for Probability {
    fn entropy(&self) -> Probability {
        let prob = self;
        return prob * (1.0 / prob).log(2.0);
    }
}

impl Entropy for Vec<Probability> {
    fn entropy(&self) -> f32 {
        return self.iter().map(|&prob| prob.entropy()).sum();
    }
}

impl Entropy for Tile {
    fn entropy(&self) -> f32 {
        let probs: Vec<Probability> = self
            .dom
            .iter()
            .zip(self.probs.iter())
            .filter(|(indom, _)| **indom)
            .map(|(_, prob)| *prob)
            .collect();
        if probs.len() <= 1 || matches!(self.state, crate::wfc::TileStateEnum::Collapsed) {
            return 0.0;
        } else {
            return probs.entropy();
        }
    }
}
