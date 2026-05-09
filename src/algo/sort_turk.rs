use crate::stacks::{Operation, RotateExt, StackExt, StackPair};

use super::sort_three::sort_three;

sort_name!();

/// Turk sort: greedily push cheapest to B, sort_three remainder, insert back.
pub fn sort_turk(stacks: &mut StackPair) {
    stacks.execute(Operation::Pb);
    stacks.execute(Operation::Pb);
    if stacks.a().len() <= 3 {
        sort_three(stacks);
        return;
    }
    for _ in 0..stacks.a().len() - 3 {
        push_cheapest(stacks);
    }
    sort_three(stacks);
    while !stacks.b().is_empty() {
        let val = stacks.b()[0];
        let pos = stacks.a().min_above_pos(val);
        stacks.rotate_a_to_top(pos);
        stacks.execute(Operation::Pa);
    }
    let min = stacks.a().min_pos();
    stacks.rotate_a_to_top(min);
}

/// Returns (cost, is_forward) for rotating `pos` to top of stack with `len` elements.
fn rot_cost(pos: usize, len: usize) -> (usize, bool) {
    let forward = pos <= len / 2;
    (if forward { pos } else { len - pos }, forward)
}

/// Compute combined rotation cost to bring positions pa and pb to top.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_random;

    // Slower than chunk/insert tests: push_cheapest is O(n²) per call (scans all of A,
    // each calling max_below_pos which scans all of B), called ~n times = O(n³) total.
    #[test]
    fn random_inputs() {
        assert_sorts_random(&[100, 500], 10, sort_turk);
    }
}
