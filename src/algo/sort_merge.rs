//! **Not included in the binary** — performance is abysmal. True linear-time
//! merge on two stacks would require reversing a run (which itself costs
//! O(run) ops + auxiliary scratch we don't have), so the merge step here
//! degrades to insertion, making the total O(n²) with worse constants than
//! plain insertion sort. Kept here only as a reference of the classical
//! divide-and-conquer structure on stacks; the module is not wired into
//! `algo.rs`.
//!
//! Recursive top-down merge sort.
//!
//! Split top n of A in half by `pb`-ing `n/2` items to B. Recursively sort
//! each half (right half stays on A; left half on B, sorted by the symmetric
//! `sort_b` routine that uses A as scratch). Merge via insertion: for each
//! item on top of B's sorted half, find its position p in A's sorted top,
//! rotate A by p, `pa`, rotate back by p. Repeat `n/2` times.
//!
//! # Why insertion-based merge, not linear-time merge
//!
//! On two stacks, you can't merge two ascending runs in O(l+r) ops without
//! reversing one of them (which itself costs O(run) ops + auxiliary space).
//! `pb` reverses on transfer, so naively pushing a run from A to B gives you
//! that run *descending* on B — wrong direction for the standard
//! `pick-smaller` merge. Reversing on the stack would need a third stack.
//!
//! So we degrade the merge step to O(l·(l+r)) by inserting each B-side item
//! into A's sorted run individually. Total cost: T(n) = 2·T(n/2) + O(n²) ⇒
//! O(n²) — same asymptotic class as insertion sort.
//!
//! # Why include it at all
//!
//! Architectural diversity. Quick/turk/radix all beat this on op count;
//! merge is here as the canonical "divide-and-conquer + combine" entry of
//! the classical-sorts set.

use crate::stacks::{Operation, StackPair};

sort_name!();

pub fn sort_merge(stacks: &mut StackPair) {
    let n = stacks.a().len();
    if n <= 1 || stacks.is_sorted() {
        return;
    }
    sort_a(stacks, n);
}

fn sort_a(stacks: &mut StackPair, n: usize) {
    if n <= 1 {
        return;
    }
    if n == 2 {
        if stacks.a()[0] > stacks.a()[1] {
            stacks.execute(Operation::Sa);
        }
        return;
    }
    let half = n / 2;
    for _ in 0..half {
        stacks.execute(Operation::Pb);
    }
    sort_a(stacks, n - half);
    sort_b(stacks, half);
    for k in 0..half {
        let v = stacks.b()[0];
        let cur = (n - half) + k;
        let mut p = 0;
        while p < cur && stacks.a()[p] < v {
            p += 1;
        }
        for _ in 0..p {
            stacks.execute(Operation::Ra);
        }
        stacks.execute(Operation::Pa);
        for _ in 0..p {
            stacks.execute(Operation::Rra);
        }
    }
}

fn sort_b(stacks: &mut StackPair, n: usize) {
    if n <= 1 {
        return;
    }
    if n == 2 {
        if stacks.b()[0] > stacks.b()[1] {
            stacks.execute(Operation::Sb);
        }
        return;
    }
    let half = n / 2;
    for _ in 0..half {
        stacks.execute(Operation::Pa);
    }
    sort_b(stacks, n - half);
    sort_a(stacks, half);
    for k in 0..half {
        let v = stacks.a()[0];
        let cur = (n - half) + k;
        let mut p = 0;
        while p < cur && stacks.b()[p] < v {
            p += 1;
        }
        for _ in 0..p {
            stacks.execute(Operation::Rb);
        }
        stacks.execute(Operation::Pb);
        for _ in 0..p {
            stacks.execute(Operation::Rrb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_random;

    #[test]
    fn random_inputs() {
        assert_sorts_random(&[100, 500], 10, sort_merge);
    }
}
