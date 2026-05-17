use std::collections::VecDeque;

use crate::stacks::{Operation, RotateExt, StackExt, StackPair};

use super::sort_three::sort_three;

sort_name!();

/// Turk sort: cost-based A→B greedy push, sort_three, cost-based B→A pull.
/// Includes circular-sorted early exit during phase 2
pub fn sort_turk(stacks: &mut StackPair) {
    if stacks.a().len() <= 1 || stacks.a().iter().is_sorted() {
        return;
    }
    if stacks.a().len() <= 3 {
        sort_three(stacks);
        return;
    }
    stacks.execute(Operation::Pb);
    stacks.execute(Operation::Pb);

    let mut early_exit = false;
    while stacks.a().len() > 3 {
        push_cheapest(stacks);
        if is_circularly_sorted(stacks.a()) {
            let min = stacks.a().min_pos();
            stacks.rotate_a_to_top(min);
            early_exit = true;
            break;
        }
    }
    if !early_exit {
        sort_three(stacks);
    }
    while !stacks.b().is_empty() {
        pull_cheapest(stacks);
    }
    let min = stacks.a().min_pos();
    stacks.rotate_a_to_top(min);
}

/// True iff A is ascending around the ring (min may sit anywhere).
fn is_circularly_sorted(a: &VecDeque<usize>) -> bool {
    let n = a.len();
    if n <= 1 {
        return true;
    }
    let min = a.iter().enumerate().min_by_key(|&(_, v)| v).unwrap().0;
    for k in 0..n - 1 {
        let i = (min + k) % n;
        let j = (min + k + 1) % n;
        if a[i] > a[j] {
            return false;
        }
    }
    true
}

/// Returns (cost, is_forward) for rotating `pos` to top of stack with `len` elements.
fn rot_cost(pos: usize, len: usize) -> (usize, bool) {
    let forward = pos <= len / 2;
    (if forward { pos } else { len - pos }, forward)
}

/// Combined rotation cost (shares rr/rrr when directions match).
fn move_cost(pa: usize, sa: usize, pb: usize, sb: usize) -> usize {
    let (ca, da) = rot_cost(pa, sa);
    let (cb, db) = rot_cost(pb, sb);
    if da == db { ca.max(cb) } else { ca + cb }
}

/// Execute combined rotations, using rr/rrr when directions align.
fn apply_rots(stacks: &mut StackPair, pos_a: usize, pos_b: usize) {
    let (mut ca, fwd_a) = rot_cost(pos_a, stacks.a().len());
    let (mut cb, fwd_b) = rot_cost(pos_b, stacks.b().len());
    if fwd_a == fwd_b {
        let shared = ca.min(cb);
        let op = if fwd_a { Operation::Rr } else { Operation::Rrr };
        for _ in 0..shared {
            stacks.execute(op);
        }
        ca -= shared;
        cb -= shared;
    }
    let op_a = if fwd_a { Operation::Ra } else { Operation::Rra };
    let op_b = if fwd_b { Operation::Rb } else { Operation::Rrb };
    for _ in 0..ca {
        stacks.execute(op_a);
    }
    for _ in 0..cb {
        stacks.execute(op_b);
    }
}

/// Find element in A with cheapest move cost to B, rotate both, push.
fn push_cheapest(stacks: &mut StackPair) {
    let sa = stacks.a().len();
    let sb = stacks.b().len();
    let (best_a, best_b) = (0..sa)
        .map(|pa| {
            let pb = stacks.b().max_below_pos(stacks.a()[pa]);
            (move_cost(pa, sa, pb, sb), pa, pb)
        })
        .min_by_key(|&(cost, _, _)| cost)
        .map(|(_, a, b)| (a, b))
        .unwrap();
    apply_rots(stacks, best_a, best_b);
    stacks.execute(Operation::Pb);
}

/// Find element in B with cheapest move cost to A, rotate both, pull.
fn pull_cheapest(stacks: &mut StackPair) {
    let sa = stacks.a().len();
    let sb = stacks.b().len();
    let (best_a, best_b) = (0..sb)
        .map(|pb| {
            let pa = stacks.a().min_above_pos(stacks.b()[pb]);
            (move_cost(pa, sa, pb, sb), pa, pb)
        })
        .min_by_key(|&(cost, _, _)| cost)
        .map(|(_, a, b)| (a, b))
        .unwrap();
    apply_rots(stacks, best_a, best_b);
    stacks.execute(Operation::Pa);
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
