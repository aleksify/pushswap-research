//! Turk + LIS + k-chunk (k>2 by rank)
//!
//! Two trends combined: keep an LIS on A to avoid moving structure twice,
//! and use multi-bucket chunking on the non-LIS push so the back-push phase
//! finds short rotations.
//!
//! # Phase outline
//!
//! 1. Pre-flight (sorted / circular / `len ≤ 3`).
//! 2. Pre-rotate min to A's top (same as `sort_turk_lis2`).
//! 3. Compute LIS mask + value set.
//! 4. **Phase 1 — chunked triage.** Process chunks low-to-high. For each
//!    chunk `[lo, hi)` (rank space):
//!      - Loop while A still holds any *non-LIS* element in that range.
//!        - If A[0] is in LIS → `ra` (skip).
//!        - Else if A[0] is in current chunk's range → `pb` (lands at top
//!          of B). Within the chunk we additionally split by chunk-midpoint:
//!          lower half gets a follow-up `rb`
//!        - Else (non-LIS but wrong chunk) → `ra`, will be picked up later.
//! 5. **Phase 2 — cost-based back-push.** `pull_cheapest` until B empty.
//! 6. Rotate min to top.

use crate::stacks::{Operation, StackPair};
use std::collections::HashSet;

use super::sort_three::sort_three;
use super::sort_turk_lis::lis_mask;
use super::turk_common::{is_circularly_sorted, pull_cheapest, rotate_min_to_top};

sort_name!();

pub fn sort_turk_lis_chunk(stacks: &mut StackPair) {
    if stacks.a().len() <= 1 || stacks.a().iter().is_sorted() {
        return;
    }
    if stacks.a().len() <= 3 {
        sort_three(stacks);
        return;
    }
    if is_circularly_sorted(stacks.a()) {
        rotate_min_to_top(stacks);
        return;
    }

    rotate_min_to_top(stacks);

    let n = stacks.a().len();
    let k = (n.isqrt() / 4).max(2);

    let snapshot: Vec<usize> = stacks.a().iter().copied().collect();
    let mask = lis_mask(&snapshot);
    let lis_values: HashSet<usize> = snapshot
        .iter()
        .zip(mask.iter())
        .filter_map(|(&v, &keep)| if keep { Some(v) } else { None })
        .collect();

    // Phase 1: process chunks low-to-high. The inner loop terminates because
    // every iteration either pb's a non-LIS-in-chunk element (strictly
    // decreasing the chunk's remaining count) or ra's (which after at most n
    // rotations returns the ring to a non-LIS-in-chunk top — guaranteed
    // because we only enter the inner loop while such an element exists).
    for chunk in 0..k {
        let lo = chunk * n / k;
        let hi = ((chunk + 1) * n / k).min(n);
        let mid = (lo + hi) / 2; // sub-chunk pivot for okbrandon-style rb
        loop {
            let remaining = stacks
                .a()
                .iter()
                .filter(|&&v| !lis_values.contains(&v) && v >= lo && v < hi)
                .count();
            if remaining == 0 {
                break;
            }
            let top = stacks.a()[0];
            if lis_values.contains(&top) {
                stacks.execute(Operation::Ra);
            } else if top >= lo && top < hi {
                stacks.execute(Operation::Pb);
                if top < mid {
                    // Lower half of chunk: shove to bottom of B (intra-chunk
                    // layering — see module docs).
                    stacks.execute(Operation::Rb);
                }
            } else {
                stacks.execute(Operation::Ra);
            }
        }
    }

    // Phase 2: cost-based back-push.
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
        assert_sorts_random(&[100, 500], 10, sort_turk_lis_chunk);
    }
}
