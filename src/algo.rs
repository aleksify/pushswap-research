macro_rules! sort_name {
    () => {
        pub fn name() -> &'static str {
            module_path!()
                .rsplit("::")
                .next()
                .unwrap()
                .strip_prefix("sort_")
                .unwrap()
        }
    };
}

mod sort_chunk;
mod sort_insertion;
mod sort_k_chunk;
mod sort_quick2;
mod sort_quick3;
mod sort_radix;
mod sort_selection;
mod sort_three;
mod sort_turk;
mod sort_turk3;
mod sort_turk_chunk;
mod sort_turk_lis;
mod sort_turk_lis2;
mod sort_turk_lis_chunk;
#[cfg(test)]
mod test_utils;
mod turk_common;

pub use sort_chunk::sort_chunk;
pub use sort_insertion::sort_insertion;
pub use sort_k_chunk::sort_k_chunk;
pub use sort_quick2::sort_quick2;
pub use sort_quick3::sort_quick3;
pub use sort_radix::sort_radix;
pub use sort_selection::sort_selection;
pub use sort_turk::sort_turk;
pub use sort_turk_chunk::sort_turk_chunk;
pub use sort_turk_lis::sort_turk_lis;
pub use sort_turk_lis_chunk::sort_turk_lis_chunk;
pub use sort_turk_lis2::sort_turk_lis2;
pub use sort_turk3::sort_turk3;

use crate::stacks::StackPair;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum Algorithm {
    Selection,
    Insertion,
    Chunk,
    KSort,
    Turk,
    Turk3,
    TurkChunk,
    TurkLis,
    TurkLis2,
    TurkLisChunk,
    Quick2,
    Quick3,
    Radix,
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Algorithm {
    pub const ALL: &[Algorithm] = &[
        Algorithm::Selection,
        Algorithm::Insertion,
        Algorithm::Chunk,
        Algorithm::KSort,
        Algorithm::Turk,
        Algorithm::Turk3,
        Algorithm::TurkChunk,
        Algorithm::TurkLis,
        Algorithm::TurkLis2,
        Algorithm::TurkLisChunk,
        Algorithm::Quick2,
        Algorithm::Quick3,
        Algorithm::Radix,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Algorithm::Selection => sort_selection::name(),
            Algorithm::Insertion => sort_insertion::name(),
            Algorithm::Chunk => sort_chunk::name(),
            Algorithm::KSort => sort_k_chunk::name(),
            Algorithm::Turk => sort_turk::name(),
            Algorithm::Turk3 => sort_turk3::name(),
            Algorithm::TurkChunk => sort_turk_chunk::name(),
            Algorithm::TurkLis => sort_turk_lis::name(),
            Algorithm::TurkLis2 => sort_turk_lis2::name(),
            Algorithm::TurkLisChunk => sort_turk_lis_chunk::name(),
            Algorithm::Quick2 => sort_quick2::name(),
            Algorithm::Quick3 => sort_quick3::name(),
            Algorithm::Radix => sort_radix::name(),
        }
    }

    pub fn from_name(name: &str) -> Option<Algorithm> {
        Self::ALL.iter().find(|a| a.name() == name).copied()
    }

    pub fn sort(self) -> fn(&mut StackPair) {
        match self {
            Algorithm::Selection => sort_selection,
            Algorithm::Insertion => sort_insertion,
            Algorithm::Chunk => sort_chunk,
            Algorithm::KSort => sort_k_chunk,
            Algorithm::Turk => sort_turk,
            Algorithm::Turk3 => sort_turk3,
            Algorithm::TurkChunk => sort_turk_chunk,
            Algorithm::TurkLis => sort_turk_lis,
            Algorithm::TurkLis2 => sort_turk_lis2,
            Algorithm::TurkLisChunk => sort_turk_lis_chunk,
            Algorithm::Quick2 => sort_quick2,
            Algorithm::Quick3 => sort_quick3,
            Algorithm::Radix => sort_radix,
        }
    }
}
