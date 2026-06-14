//! Fred Orion's 3-way triage variant of Turk+Chunk.
//!
//! Empirically the strongest published pure-chunked Turk variant:
//! **~555 mean ops at n=100** (beats pure Turk's 561) and **~4216 at n=500**
//! (vs. pure Turk's ~5100). The combination of cheap bulk transfer with a
//! lightly pre-ordered B makes the back-push phase very cheap.
//!
//! # See also: relationship to `sort_quick3`
//!
//! turk3's Phase 1 triage emits exactly the same ops as `sort_quick3`'s
//! top-level `chunk_split` from `TopA`. The mapping is:
//!
//! | turk3 rank-third | quick3 destination (via `split_locs(TopA)`) | Ops    |
//! |------------------|---------------------------------------------|--------|
//! | top  → `ra`      | max → BottomA                               | `Ra`   |
//! | mid  → `pb`      | mid → TopB                                  | `Pb`   |
//! | bot  → `pb rb`   | min → BottomB                               | `Pb,Rb`|
//!
//! Conceptually: **turk3 = quick3 truncated to recursion depth 1**, with a
//! cost-based back-push (`pull_cheapest`) replacing further recursion. quick3
//! recurses into each of the 3 sub-buckets using location-aware
//! `move_from_to` routing; turk3 stops after one triage pass and uses the
//! generic Turk cost model to pull B back to A.
//!
//! The shared op-emission pattern is too small (3 lines) to factor out
//! cleanly without dragging quick3's `Loc` enum + `move_from_to` table into
//! turk3, where 9 of the 12 source/dest pairs would be dead code.
//!
//! # Philosophy
//!
//! Split values into three equal thirds by rank: **bottom**, **middle**,
//! **top**. Walk A from the top exactly once and triage each element by which
//! third it belongs to:
//!
//! | Third  | Action  | Effect on B                             |
//! |--------|---------|-----------------------------------------|
//! | Top    | `ra`    | element rotates to bottom of A (stays)  |
//! | Middle | `pb`    | lands at top of B                       |
//! | Bottom | `pb rb` | lands at bottom of B                    |
//!
//! 1 or 2 ops per element — close to the minimum any algorithm could pay for
//! bulk movement. After triage A holds only top-third elements, B holds
//! everything else with the bottom-third already at the bottom of B (a "free"
//! layering that exploits B's circularity).
//!
//! # Phase outline
//!
//! 1. **Triage.** Walk A, apply the table above, until no middle/bottom
//!    elements remain on A.
//! 2. **Residual push.** A now holds only top-third elements (~n/3). Run the
//!    pure-Turk cost-cheapest push loop on the residual until 3 are left, with
//!    the alx-sch circular-sorted early exit. (Most of the time we exit early
//!    because the top third arrives in near-sorted order after the triage.)
//! 3. **sort_three** the leftover 3 on A (if we didn't early-exit).
//! 4. **Cost-based back-push.** Drain B into A using `pull_cheapest`. B's
//!    pre-existing top-vs-bottom layering means the cost function finds short
//!    rotations for most pulls.
//! 5. **Rotate min to top** to finish.

use crate::stacks::{Operation, StackPair};

use super::sort_three::sort_three;
use super::turk_common::{is_circularly_sorted, pull_cheapest, push_cheapest, rotate_min_to_top};

sort_name!();

pub fn sort_turk3(stacks: &mut StackPair) {
    if stacks.a().len() <= 1 || stacks.a().iter().is_sorted() {
        return;
    }
    if stacks.a().len() <= 3 {
        sort_three(stacks);
        return;
    }

    let n = stacks.a().len();
    // Rank-based thirds. With integer division the top third absorbs the
    // remainder (its range is [two_third, n)), which is fine — it's the third
    // that stays on A anyway, so a slight size imbalance has no cost.
    let third = n / 3;
    let two_third = 2 * n / 3;

    // Phase 1: triage. Loop terminates when A holds only top-third elements.
    // Worst case O(n) ops (every element is touched at most once via ra or
    // pb-or-pb-rb), so it terminates in ≤ ~n iterations.
    while stacks.a().iter().any(|&v| v < two_third) {
        let top = stacks.a()[0];
        if top >= two_third {
            // Top third: rotate to bottom of A, will be handled in phase 2.
            stacks.execute(Operation::Ra);
        } else if top >= third {
            // Middle third: lands at top of B.
            stacks.execute(Operation::Pb);
        } else {
            // Bottom third: pb places at top of B, then rb shoves to bottom.
            // This puts the smallest values at the bottom of B, exploiting
            // circularity so cost-back-push finds them via short rrb's.
            stacks.execute(Operation::Pb);
            stacks.execute(Operation::Rb);
        }
    }

    // Phase 2: residual top-third push with circular early-exit.
    // A often becomes circularly sorted very quickly here — top-third values
    // were left in their original relative order after the triage, and that
    // order is typically near-monotonic for random inputs.
    let mut early_exit = false;
    while stacks.a().len() > 3 {
        push_cheapest(stacks);
        if is_circularly_sorted(stacks.a()) {
            rotate_min_to_top(stacks);
            early_exit = true;
            break;
        }
    }
    if !early_exit {
        // Phase 3: sort_three handles len 0..=3.
        sort_three(stacks);
    }

    // Phase 4: cost-based back-push. Reuses the same cost machinery as pure
    // Turk; B's existing layering makes this cheaper here than for pure Turk.
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
        assert_sorts_random(&[100, 500], 10, sort_turk3);
    }
}
