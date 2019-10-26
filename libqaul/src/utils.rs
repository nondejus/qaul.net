//! General utility module

use rand::prelude::*;

pub(crate) fn random(len: usize) -> Vec<u8> {
    (0..)
        .map(|_| rand::thread_rng().next_u64())
        .take(len)
        .map(|x| x.to_be_bytes())
        .fold(Vec::new(), |mut acc, arr| {
            acc.extend(arr.iter().cloned());
            acc
        })
}

/// A functional remove/add API for datastructures
pub(crate) trait VecUtils<T: PartialEq> {
    /// Remove from vector, by element
    fn strip(self, t: &T) -> Self;
    /// Add to vector, returning `Self`
    fn add(self, t: T) -> Self;
}

impl<T: PartialEq> VecUtils<T> for Vec<T> {
    #[inline(always)]
    fn strip(mut self, t: &T) -> Self {
        self.into_iter().filter(|i| i != t).collect()
    }

    #[inline(always)]
    fn add(mut self, t: T) -> Self {
        self.push(t);
        self
    }
}
