//! Pure Turk + Chunk — equal-width rank buckets, vanilla `pb`, cost back-push.
//!
//! # Algorithm
//!
//! 1. Pre-flight (already-sorted → return; len ≤ 3 → ignored here, handled
//!    by pure Turk's hardcoded sort_three if you wire it in).
//! 2. `chunk_size = max(4, √n × 2)`, `k = ⌈n / chunk_size⌉`.
//!    Per-chunk overhead is O(n) (scan A once per chunk);
//!    per-element cost is O(1) pb plus rotation. Balancing → k ∝ √n.
//! 3. For each bucket `[lo, hi)` low-to-high, while A holds any element in
//!    that range: rotate cheapest in-range to top, `pb`.
//! 4. A is empty. Seed back-push by rotating B's max to top + `pa` (cost
//!    machinery needs ≥ 1 element in A to compute targets).
//! 5. Cost-based back-push via `pull_cheapest` until B empty.
//! 6. Rotate min to top.

use crate::stacks::{Operation, RotateExt, StackExt, StackPair};

use super::turk_common::{cheapest_in_range, pull_cheapest, rotate_min_to_top};

sort_name!();

pub fn sort_turk_chunk(stacks: &mut StackPair) {
    if stacks.a().len() <= 1 || stacks.a().iter().is_sorted() {
        return;
    }

    let n = stacks.a().len();
    let chunk_size = (n.isqrt() * 2).max(4);
    let num_chunks = n.div_ceil(chunk_size);

    // Phase 1: vanilla chunked push. No median pivot, no rb. Just walk
    // buckets low-to-high and pb every in-range element.
    for chunk in 0..num_chunks {
        let lo = chunk * chunk_size;
        let hi = ((chunk + 1) * chunk_size).min(n);

        while let Some(pos) = cheapest_in_range(stacks.a(), lo, hi) {
            stacks.rotate_a_to_top(pos);
            stacks.execute(Operation::Pb);
        }
    }

    // Seed A: phase 1 drains A fully; pull_cheapest would panic on the
    // first call (target-in-A lookup falls back to min_pos of empty deque).
    // Rotate B's max to the top, pa it — now A has one element.
    if stacks.a().is_empty() && !stacks.b().is_empty() {
        let max_pos = stacks.b().max_pos();
        stacks.rotate_b_to_top(max_pos);
        stacks.execute(Operation::Pa);
    }

    while !stacks.b().is_empty() {
        pull_cheapest(stacks);
    }

    rotate_min_to_top(stacks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_random;

    #[test]
    fn random_inputs() {
        assert_sorts_random(&[100, 500], 10, sort_turk_chunk);
    }
}
