macro_rules! sort_name {
    () => {
        pub fn name() -> &'static str {
            module_path!().rsplit("::").next().unwrap().strip_prefix("sort_").unwrap()
        }
    };
}

mod sort_insertion;
mod sort_k_chunk;
mod sort_selection;
mod sort_three;
mod sort_turk;
#[cfg(test)]
mod test_utils;

pub use sort_insertion::sort_insertion;
pub use sort_k_chunk::sort_k_chunk;
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
        let name = match self {
            Algorithm::Selection => sort_selection::name(),
            Algorithm::Insertion => sort_insertion::name(),
            Algorithm::KSort => sort_k_chunk::name(),
            Algorithm::Turk => sort_turk::name(),
        };
        write!(f, "{name}")
    }
}

impl Algorithm {
    pub fn sort(self) -> fn(&mut StackPair) {
        match self {
            Algorithm::Selection => sort_selection,
            Algorithm::Insertion => sort_insertion,
            Algorithm::KSort => sort_k_chunk,
            Algorithm::Turk => sort_turk,
        }
    }
}
