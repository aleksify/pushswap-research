//! Find the longest already-ordered subsequence of A and keep those elements
//! in place. Push everything else to B, then cost-pull back. Each kept element
//! saves the ≥2 ops of being moved twice (`pb` + `pa`); in theory we save up
//! to ~2 × LIS_length ops total. For random uniform input, expected LIS
//! length ≈ 2√n (≈ 20 at n=100, ≈ 45 at n=500), so the theoretical ceiling is
//! ~40 ops saved at n=100. In practice rotations to make room for incoming B
//! elements eat most of that — LIS-based sorts are *competitive* with pure
//! Turk on small/medium stacks but rarely dominant. The advantage scales
//! better at large n where preserved structure pays off.
//!
//!
//!    **Pre-rotate min to A's top before LIS extraction.** Costs ≤ n/2 ops up
//!    front but:
//!    - LIS extraction stays linear — no need for circular LIS or wrap-around
//!      handling.
//!    - The resulting LIS is longer on average than the un-rotated version
//!      because the sequence can run from the natural beginning of the sorted
//!      order without wrapping.
//!
//! # Phase outline
//!
//! 1. **Pre-flight.** Already-sorted or circularly-sorted → handle directly.
//!    `len ≤ 3` → `sort_three`.
//! 2. **Pre-rotate min to top of A.** Now A's contents read linearly from the
//!    sorted start.
//! 3. **Compute LIS mask** on the rotated contents via O(n²) DP.
//! 4. **Phase 1 — triage by membership.** While A still holds any non-LIS
//!    element:
//!    - If A[0] is in the LIS set → `ra` (LIS member cycles to bottom of A,
//!      staying in A).
//!    - Else → `pb` (non-LIS member moves to B with no ordering hint).
//! 5. **Phase 2 — cost-based back-push.** Drain B via `pull_cheapest`. Each
//!    pull places its element in A at the smallest A-value larger than it,
//!    slotting between LIS members.
//! 6. **Rotate min to top** to finish (usually a no-op since the back-push
//!    naturally keeps min near the top, but cheap and safe to call).
//!
//! # Why phase 1 terminates in ≤ n iterations
//!
//! Every A[0] is either a LIS member (consumed via `ra`, never re-checked
//! until the ring wraps) or a non-LIS member (consumed via `pb`, removed from
//! A). A's size strictly decreases on every `pb`. Worst case: all LIS members
//! sit consecutively at the top, all non-LIS at the bottom — n iterations
//! total (LIS_length `ra`'s followed by (n − LIS_length) `pb`'s).

use std::collections::HashSet;

use crate::stacks::{Operation, StackPair};

use super::sort_three::sort_three;
use super::turk_common::{is_circularly_sorted, pull_cheapest, rotate_min_to_top};

sort_name!();

pub fn sort_turk_lis(stacks: &mut StackPair) {
    if stacks.a().len() <= 1 || stacks.a().iter().is_sorted() {
        return;
    }
    if stacks.a().len() <= 3 {
        sort_three(stacks);
        return;
    }
    // Free win: A is one rotation away from sorted. Skip LIS work entirely.
    if is_circularly_sorted(stacks.a()) {
        rotate_min_to_top(stacks);
        return;
    }

    // Step 1: pre-rotate min to A's top. After this, A's content reads from
    // the natural sorted start, so the LIS extraction can ignore wrap-around.
    rotate_min_to_top(stacks);

    // Step 2: snapshot A's current order and compute LIS membership.
    let snapshot: Vec<usize> = stacks.a().iter().copied().collect();
    let mask = lis_mask(&snapshot);
    // Collect the actual *values* (not positions) in the LIS. Values are
    // stable under rotation; positions are not.
    let lis_values: HashSet<usize> = snapshot
        .iter()
        .zip(mask.iter())
        .filter_map(|(&v, &keep)| if keep { Some(v) } else { None })
        .collect();

    // Step 3: triage. ra LIS members, pb non-LIS members, until A is purely
    // LIS. We test the "any non-LIS in A" predicate by iterating A's contents
    // — O(n) per check but n shrinks, so total O(n²) which is fine for the
    // sizes push_swap targets (n ≤ 500).
    while stacks.a().iter().any(|v| !lis_values.contains(v)) {
        if lis_values.contains(&stacks.a()[0]) {
            stacks.execute(Operation::Ra);
        } else {
            stacks.execute(Operation::Pb);
        }
    }

    // Step 4: cost-back-push. A now holds LIS elements in ascending order
    // (possibly rotated). pull_cheapest slots each B element between the
    // appropriate LIS pair, using the same 4-case cost machinery as pure
    // Turk's phase 4.
    while !stacks.b().is_empty() {
        pull_cheapest(stacks);
    }

    rotate_min_to_top(stacks);
}

/// O(n²) DP longest-increasing-subsequence. Returns a boolean mask where
/// `mask[i] == true` iff `a[i]` belongs to (one) longest increasing
/// subsequence of `a`.
///
/// Standard predecessor-pointer backtrack: walk forward filling `lengths[i]`
/// = length of the longest LIS ending at `i`, recording `prev[i]` = the index
/// that produced that length. After the forward pass, find the position
/// achieving the global maximum length and walk back through `prev` to mark
/// the chain.
///
/// Ties are broken by the first predecessor encountered, which favours LIS
/// chains that start earlier in `a`. Combined with the min-pre-rotation, that
/// means the chosen LIS tends to start near A's top — fewer rotations later
/// when slotting B elements between LIS members.
pub(super) fn lis_mask(a: &[usize]) -> Vec<bool> {
    let n = a.len();
    if n == 0 {
        return Vec::new();
    }
    let mut lengths = vec![1usize; n];
    let mut prev: Vec<Option<usize>> = vec![None; n];
    let mut best = 0usize;
    for i in 1..n {
        for j in 0..i {
            if a[j] < a[i] && lengths[j] + 1 > lengths[i] {
                lengths[i] = lengths[j] + 1;
                prev[i] = Some(j);
            }
        }
        if lengths[i] > lengths[best] {
            best = i;
        }
    }
    let mut mask = vec![false; n];
    let mut cur = Some(best);
    while let Some(i) = cur {
        mask[i] = true;
        cur = prev[i];
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_random;

    #[test]
    fn random_inputs() {
        assert_sorts_random(&[100, 500], 10, sort_turk_lis);
    }

    #[test]
    fn lis_mask_basic() {
        // Classic LIS example: [10, 22, 9, 33, 21, 50, 41, 60] → LIS length 5
        // e.g. {10, 22, 33, 50, 60} or {10, 22, 33, 41, 60}.
        let a = vec![10, 22, 9, 33, 21, 50, 41, 60];
        let m = lis_mask(&a);
        let kept: Vec<usize> = a.iter().zip(m.iter()).filter_map(|(v, k)| k.then_some(*v)).collect();
        assert_eq!(kept.len(), 5);
        assert!(kept.windows(2).all(|w| w[0] < w[1]));
    }
}
