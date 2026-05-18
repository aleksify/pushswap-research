//! Selection sort.
//!
//! Each iteration: find min of A (linear scan), rotate it to A's top via the
//! shorter of `ra`/`rra`, then `pb` it. Repeat until 3 items remain, finish
//! with `sort_three`, then `pa` back all `n-3` items from B (which sit
//! ascending top-down because they were `pb`'d smallest-first).
//!
//! O(n²) ops (linear scan per iteration × n iterations). Beats `sort_bubble`
//! because rotations use shortest direction, but loses to every chunk/turk
//! variant for n > ~10.

use crate::stacks::{Operation, RotateExt, StackExt, StackPair};

use super::sort_three::sort_three;

sort_name!();

pub fn sort_selection(stacks: &mut StackPair) {
    if stacks.a().len() <= 3 {
        sort_three(stacks);
        return;
    }
    let pushes_needed = stacks.a().len() - 3;
    for _ in 0..pushes_needed {
        let pos = stacks.a().min_pos();
        stacks.rotate_a_to_top(pos);
        stacks.execute(Operation::Pb);
    }
    sort_three(stacks);
    for _ in 0..pushes_needed {
        stacks.execute(Operation::Pa);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_all;

    #[test]
    fn all_permutations() {
        assert_sorts_all(1, 5, sort_selection);
    }
}
