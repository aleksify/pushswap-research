//! Shared cost-calculation and rotation primitives used by the Turk family of
//! sorts (`sort_turk`, `sort_turk3`, `sort_turk_chunk`, `sort_turk_lis`,
//! `sort_turk_lis2`, `sort_turk_lis_chunk`).
//!
//! Every Turk-style variant ultimately needs to ask the same questions:
//!
//! - "How many ops to bring position `p` to the top of a stack of length
//!   `len`, and in which direction?" → [`rot_cost`].
//! - "How many ops to bring position `pa` to the top of A *and* position `pb`
//!   to the top of B, sharing `rr`/`rrr` when both rotate the same direction?"
//!   → [`move_cost`].
//! - "Actually perform that combined rotation, emitting `rr`/`rrr` for the
//!   shared portion." → [`apply_rots`].
//!
//! Built on those: the two cost-minimising move selectors
//! ([`push_cheapest`], [`pull_cheapest`]) and the circular-sorted predicate
//! ([`is_circularly_sorted`]) used for early-exit during phase 2.
//!
//! Keep this module algorithm-agnostic: anything pure-Turk-specific (initial
//! seed, phase ordering, chunking decisions) belongs in the individual
//! `sort_turk_*.rs` files.

use std::collections::VecDeque;

use crate::stacks::{Operation, RotateExt, StackExt, StackPair};

/// Returns `(rotation_count, is_forward)` for bringing `pos` to the top of a
/// stack of length `len`. Picks the shorter of `ra`/`rra` directions; ties
/// favour forward (`pos <= len/2`).
pub(super) fn rot_cost(pos: usize, len: usize) -> (usize, bool) {
    let forward = pos <= len / 2;
    (if forward { pos } else { len - pos }, forward)
}

/// Combined cost to bring `pa` to top of A and `pb` to top of B
/// simultaneously. When the two rotations share a direction we can use
/// `rr`/`rrr` (1 op covers both stacks), so cost = `max(ca, cb)`. Otherwise
/// each rotation pays separately: cost = `ca + cb`. This is the canonical
/// "sign-based 4-case Turk cost" formula (sisittu99 / alx-sch / adi7-x).
pub(super) fn move_cost(pa: usize, sa: usize, pb: usize, sb: usize) -> usize {
    let (ca, da) = rot_cost(pa, sa);
    let (cb, db) = rot_cost(pb, sb);
    if da == db { ca.max(cb) } else { ca + cb }
}

/// Emit the rotations to bring `pos_a` and `pos_b` to their respective tops,
/// using `rr`/`rrr` for the shared prefix when directions match. Caller is
/// expected to follow with `pa`/`pb` to actually move the element.
pub(super) fn apply_rots(stacks: &mut StackPair, pos_a: usize, pos_b: usize) {
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

/// Phase-2 primitive: scan every element of A, find the one whose move to B
/// is cheapest (in combined Turk cost). The target slot in B is the largest
/// B value smaller than the candidate (wrapping to `max(B)` if the candidate
/// undercuts everything currently in B — preserves the descending-B invariant
/// that pure Turk maintains).
///
/// Rotates both stacks then `pb`. Caller is responsible for the surrounding
/// loop (and for the `len(A) > 3` stopping condition).
pub(super) fn push_cheapest(stacks: &mut StackPair) {
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

/// Phase-4 primitive: scan every element of B, find the one whose pull onto A
/// is cheapest. Target slot in A is the smallest A value larger than the
/// candidate (wrapping to `min(A)` if the candidate exceeds everything in A,
/// which lets it land just above the eventual minimum).
///
/// Symmetric to [`push_cheapest`]. Rotates both stacks then `pa`.
pub(super) fn pull_cheapest(stacks: &mut StackPair) {
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

/// True iff A is ascending around the ring — i.e. there exists a single
/// rotation that linearises A into sorted order. Used as the alx-sch early
/// exit: as soon as the residual A reaches this state during phase 2 we can
/// stop pushing, rotate min to the top, and proceed straight to back-push.
///
/// O(n) per check; cheap enough to call after every `pb`.
pub(super) fn is_circularly_sorted(a: &VecDeque<usize>) -> bool {
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

/// Rotate A so its minimum sits at index 0. Used as the standard finishing
/// move on every Turk variant after the back-push phase, and as the inline
/// finisher when [`is_circularly_sorted`] fires during phase 2.
pub(super) fn rotate_min_to_top(stacks: &mut StackPair) {
    let min = stacks.a().min_pos();
    stacks.rotate_a_to_top(min);
}

/// Find the position in `a` of an element whose rank falls in `[lo, hi)` and
/// is closest (in rotation count) to the top. Returns `None` if no such
/// element exists.
///
/// Used by chunked variants (Fred, smart-B, pivot) when scanning A for the
/// next candidate to triage. Equivalent to leogaudin's `scan_from_top` +
/// `scan_from_bottom` pair: by evaluating both `ra` and `rra` distance via
/// [`rot_cost`], we naturally pick the cheaper direction.
pub(super) fn cheapest_in_range(a: &VecDeque<usize>, lo: usize, hi: usize) -> Option<usize> {
    let n = a.len();
    (0..n)
        .filter(|&i| a[i] >= lo && a[i] < hi)
        .min_by_key(|&i| rot_cost(i, n).0)
}
