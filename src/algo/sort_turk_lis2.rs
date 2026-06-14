//! Turk + LIS + k=2 median pivot
//!
//! Dansylvain's published variant uses LIS pre-pass + a single global median
//! pivot to split non-LIS elements into two layers on B (above-median → top
//! of B; below-median → bottom of B). Reported ~566 ops at n=100 — close to
//! pure Turk's 561 but with structurally different behaviour on near-sorted
//! input.
//!
//! # Phase outline
//!
//! 1. **Pre-flight.** Already-sorted / circularly-sorted / `len ≤ 3` short
//!    circuits (same as `sort_turk_lis`).
//! 2. **Pre-rotate min to A's top.** Makes LIS extraction linear.
//! 3. **Compute LIS mask** on the rotated A.
//! 4. **Phase 1 — triage by LIS membership + median pivot.** While A holds
//!    any non-LIS element:
//!    - A[0] is in LIS → `ra` (skip).
//!    - A[0] ≥ median → `pb` (lands at top of B → "upper half" layer).
//!    - A[0] <  median → `pb` then `rb` (shoved to bottom of B → "lower half"
//!      layer).
//!
//!    Result: B has its high-value half near the top and low-value half near
//!    the bottom, with values interleaved within each half. The cost-based
//!    back-push finds short rotations into A because the right half of B is
//!    usually close to the right slot in A.
//! 5. **Phase 2 — cost-based back-push.** `pull_cheapest` until B empty.
//! 6. **Rotate min to top** to finish.
//!
//! # Why median is computed on ranks, not raw values
//!
//! The codebase normalises inputs to ranks `0..n-1` (`process_and_rank` in
//! `lib.rs`), so `n / 2` *is* the value median. No separate computation
//! needed — same trick the other rank-driven variants use.

use crate::stacks::{Operation, StackPair};
use std::collections::HashSet;

use super::sort_three::sort_three;
use super::sort_turk_lis::lis_mask;
use super::turk_common::{is_circularly_sorted, pull_cheapest, rotate_min_to_top};

sort_name!();

pub fn sort_turk_lis2(stacks: &mut StackPair) {
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

    // Step 1: pre-rotate min. Linearises the LIS extraction window.
    rotate_min_to_top(stacks);

    let n = stacks.a().len();
    let median = n / 2; // ranks-based pivot — see module docs

    // Step 2: LIS membership set (values, not positions — positions shift).
    let snapshot: Vec<usize> = stacks.a().iter().copied().collect();
    let mask = lis_mask(&snapshot);
    let lis_values: HashSet<usize> = snapshot
        .iter()
        .zip(mask.iter())
        .filter_map(|(&v, &keep)| if keep { Some(v) } else { None })
        .collect();

    // Step 3: triage by LIS membership and median pivot. The dual decision
    // is what makes this distinct from pure `sort_turk_lis`:
    //   - LIS member  → ra
    //   - non-LIS, ≥ median → pb           (lands at top of B)
    //   - non-LIS, <  median → pb then rb  (lands at bottom of B)
    while stacks.a().iter().any(|v| !lis_values.contains(v)) {
        let top = stacks.a()[0];
        if lis_values.contains(&top) {
            stacks.execute(Operation::Ra);
        } else if top >= median {
            stacks.execute(Operation::Pb);
        } else {
            stacks.execute(Operation::Pb);
            stacks.execute(Operation::Rb);
        }
    }

    // Step 4: cost-based back-push. B's two-layer structure means most pulls
    // land short rotations; pull_cheapest handles whatever ordering remains.
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
        assert_sorts_random(&[100, 500], 10, sort_turk_lis2);
    }
}
