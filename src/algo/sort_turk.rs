//! Pure Turk (Ali Yigit Ogun's algorithm) — the baseline of the Turk family.

use crate::stacks::{Operation, StackPair};

use super::sort_three::sort_three;
use super::turk_common::{
    is_circularly_sorted, pull_cheapest, push_cheapest, rotate_min_to_top,
};

sort_name!();

/// Pure Turk:
/// 1. Blind seed: push first 2 of A to B.
/// 2. Phase 2: while `|A| > 3`, push the cost-cheapest A element to B.
///    After each push, check if A is circularly sorted; if so, rotate min to
///    top and break out (alx-sch's enhancement).
/// 3. Phase 3: hardcoded `sort_three` on the final 3 of A.
/// 4. Phase 4: while B non-empty, pull the cost-cheapest B element back to A.
/// 5. Rotate A's minimum to the top.
pub fn sort_turk(stacks: &mut StackPair) {
    // Already sorted / trivially small: nothing to do.
    if stacks.a().len() <= 1 || stacks.a().iter().is_sorted() {
        return;
    }
    if stacks.a().len() <= 3 {
        sort_three(stacks);
        return;
    }

    stacks.execute(Operation::Pb);
    stacks.execute(Operation::Pb);

    // Phase 2: greedy push + circular early-exit.
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
        sort_three(stacks);
    }

    // Phase 4: cost-based back-push.
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
        assert_sorts_random(&[100, 500], 10, sort_turk);
    }

    #[test]
    fn circular_early_exit() {
        let mut stacks = StackPair::new(vec![8, 9, 1, 2, 3, 4, 5, 6, 7, 0]);
        sort_turk(&mut stacks);
        assert!(stacks.is_sorted());
    }
}
