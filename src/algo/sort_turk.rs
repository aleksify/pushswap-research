use crate::stacks::{Operation, RotateExt, StackExt, StackPair};

use super::sort_three::sort_three;

/// Turk sort: greedily push cheapest to B, sort_three remainder, insert back.
pub fn sort_turk(stacks: &mut StackPair) {
    stacks.execute(Operation::Pb);
    stacks.execute(Operation::Pb);
    while stacks.a().len() > 3 {
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
    let mut ra = pos_a as isize;
    let mut rb = pos_b as isize;
    if ra > stacks.a().len() as isize / 2 {
        ra -= stacks.a().len() as isize;
    }
    if rb > stacks.b().len() as isize / 2 {
        rb -= stacks.b().len() as isize;
    }
    while ra > 0 && rb > 0 {
        stacks.execute(Operation::Rr);
        ra -= 1;
        rb -= 1;
    }
    while ra < 0 && rb < 0 {
        stacks.execute(Operation::Rrr);
        ra += 1;
        rb += 1;
    }
    while ra > 0 {
        stacks.execute(Operation::Ra);
        ra -= 1;
    }
    while ra < 0 {
        stacks.execute(Operation::Rra);
        ra += 1;
    }
    while rb > 0 {
        stacks.execute(Operation::Rb);
        rb -= 1;
    }
    while rb < 0 {
        stacks.execute(Operation::Rrb);
        rb += 1;
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
