//! Insertion sort.
//!
//! Phase 1: scan A once, keeping a running `max_seen`. Each element ≥
//! `max_seen` is "in its ascending position" — `ra` it (rotates to bottom of
//! A) and update `max_seen`. Each element < `max_seen` breaks ascent — `pb`
//! it to B. After the pass, A holds an ascending ring of "natural-run"
//! elements; B holds the out-of-order ones.
//!
//! Phase 2: for each item on top of B, find its target slot in A (smallest A
//! value ≥ the item, with rank-wrap), rotate A so that slot is on top, `pa`.
//! Finish with min-to-top.
//!
//! O(n²) worst case. Faster than `sort_bubble`/`sort_selection` thanks to
//! Phase 1 keeping already-ordered elements untouched — strongly favours
//! near-sorted input.

use crate::stacks::{Operation, RotateExt, StackExt, StackPair};

sort_name!();

pub fn sort_insertion(stacks: &mut StackPair) {
    push_unsorted(stacks);
    while !stacks.b().is_empty() {
        let val = stacks.b()[0];
        let pos = stacks.a().min_above_pos(val);
        stacks.rotate_a_to_top(pos);
        stacks.execute(Operation::Pa);
    }
    let min = stacks.a().min_pos();
    stacks.rotate_a_to_top(min);
}

fn push_unsorted(stacks: &mut StackPair) {
    let n = stacks.a().len();
    let mut max_seen = stacks.a()[0];
    for _ in 0..n {
        if stacks.a()[0] >= max_seen {
            max_seen = stacks.a()[0];
            stacks.execute(Operation::Ra);
        } else {
            stacks.execute(Operation::Pb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_random;

    #[test]
    fn random_inputs() {
        assert_sorts_random(&[100, 500], 10, sort_insertion);
    }
}
