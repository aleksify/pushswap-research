mod sort_chunk;
mod sort_insert;
mod sort_selection;
mod sort_three;
mod sort_turk;
#[cfg(test)]
mod test_utils;

pub use sort_chunk::sort_chunk;
pub use sort_insert::sort_insert;
pub use sort_selection::sort_selection;
pub use sort_turk::sort_turk;

use crate::stacks::StackPair;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum Algorithm {
    Selection,
    Insertion,
    KSort,
    Turk,
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Algorithm::Selection => write!(f, "Selection Sort"),
            Algorithm::Insertion => write!(f, "Insertion Sort"),
            Algorithm::KSort => write!(f, "K-Sort"),
            Algorithm::Turk => write!(f, "Turk Sort"),
        }
    }
}

impl Algorithm {
    pub fn sort(self) -> fn(&mut StackPair) {
        match self {
            Algorithm::Selection => sort_selection,
            Algorithm::Insertion => sort_insert,
            Algorithm::KSort => sort_chunk,
            Algorithm::Turk => sort_turk,
        }
    }
}
