use std::{
    iter::zip,
    ops::{BitAnd, BitAndAssign, BitOr},
    slice::Iter, fmt::Debug,
};

#[derive(Clone, PartialEq, Eq)]
pub struct Domain(pub Vec<bool>);

impl Debug for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let chars: Vec<char> = self.0.iter().map(|b| {
            match b {
                true => 'T',
                false => 'F'
            }
        }).collect();
        let str = format!("Domain{:?}", chars);
        f.write_str(&str)
    }
}

impl Domain {
    pub fn taut(len: usize) -> Domain {
        return Domain(vec![true; len]);
    }

    // for correctness
    pub fn cont(len: usize) -> Domain {
        return Self::new(len);
    }

    pub fn new(size: usize) -> Domain {
        return Domain(vec![false; size]);
    }

    pub fn fill(&mut self, val: bool) {
        self.0.fill(val);
    }

    pub fn iter(&self) -> Iter<bool> {
        return self.0.iter();
    }

    pub fn only(&mut self, idx: usize) {
        self.0.fill(false);
        self.0[idx] = true;
    }

    pub fn len(&self) -> usize {
        return self.0.len();
    }

    /// ands together a vector of Domains
    /// if doms.is_empty() returns a vector of length len that is all false
    pub fn andv(doms: &[Domain], len: usize) -> Domain {
        return match !doms.is_empty() {
            true => doms.iter().fold(Domain::taut(len), |a, b| &a & b),
            false => Domain::cont(len),
        };
    }

    pub fn orv(doms: &[Domain]) -> Domain {
        let len = doms[0].len();
        return doms.iter().fold(Domain::cont(len), |a, b| &a | b);
    }

    pub fn filter<T>(filter: &Self, source: &Vec<T>) -> Vec<T>
    where
        T: Clone,
    {
        return zip(filter.iter(), source)
            .filter(|(&in_domain, _)| in_domain)
            .map(|(_, thing)| thing)
            .cloned()
            .collect();
    }

    pub fn any(&self) -> bool {
        return self.0.contains(&true);
    }
}

impl BitOr for &Domain {
    type Output = Domain;
    fn bitor(self, rhs: Self) -> Self::Output {
        return Domain(
            zip(&self.0, &rhs.0)
                .map(|(a, b)| a | b)
                .collect::<Vec<bool>>(),
        );
    }
}

impl BitAnd for &Domain {
    type Output = Domain;

    fn bitand(self, rhs: Self) -> Self::Output {
        return Domain(
            zip(&self.0, &rhs.0)
                .map(|(a, b)| a & b)
                .collect::<Vec<bool>>(),
        );
    }
}

impl BitAnd for Domain {
    type Output = Domain;
    fn bitand(self, rhs: Self) -> Self::Output {
        return Domain(
            zip(&self.0, &rhs.0)
                .map(|(a, b)| a & b)
                .collect::<Vec<bool>>(),
        );
    }
}

impl BitAndAssign for Domain {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = &*self & &rhs;
    }
}

impl Default for Domain {
    fn default() -> Self {
        return Domain(Vec::new());
    }
}

#[cfg(test)]
mod test {
    use crate::domain;
    use domain::Domain;

    #[test]
    fn bit_and_all_false() {
        let a = Domain(vec![true, true, true, true]);
        let b = Domain(vec![false, false, false, false]);
        assert_eq!(a & b, Domain(vec![false, false, false, false]))
    }

    #[test]
    fn bit_and_all_true() {
        let a = Domain(vec![true, true, true, true]);
        let b = Domain(vec![true, true, true, true]);
        assert_eq!(a & b, Domain(vec![true, true, true, true]))
    }

    #[test]
    fn filter_all_false() {
        let filter = Domain(vec![false, false, false, false]);
        let items = vec![0, 0, 0, 0];
        let empty: Vec<i32> = vec![];
        assert_eq!(empty, Domain::filter(&filter, &items));
    }

    #[test]
    fn filter_some() {
        let filter = Domain(vec![true, false, true, false]);
        let items = vec![1, 0, 2, 0];
        let result: Vec<i32> = vec![1, 2];
        assert_eq!(result, Domain::filter(&filter, &items));
    }

    #[test]
    fn andv_all_true() {
        let a = Domain(vec![true, true, true, true, true]);
        let b = Domain(vec![true, true, true, true, true]);
        let r = Domain(vec![true, true, true, true, true]);
        let ab = &[a, b];
        assert_eq!(Domain::andv(ab, 5), r);
    }

    #[test]
    fn andv_alternating_doesnt_just_do_false() {
        let a = Domain(vec![true, false, true, false, true]);
        let b = Domain(vec![false, true, false, true, false]);
        let r = Domain(vec![false, false, false, false, false]);
        let ab = &[a, b];
        assert_eq!(Domain::andv(ab, 5), r);
    }
}
