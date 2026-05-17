//! Ulysse Gerkens-style 3-way chunk quicksort (`ulsgks/push_swap`).
//!
//! # What's classic quicksort
//!
//! The skeleton in [`rec_chunk_sort`] is plain recursive quicksort:
//!
//! 1. Pick pivot(s), partition a "chunk" of elements by value into sub-chunks.
//! 2. Recurse on each sub-chunk.
//! 3. Base cases handle size 1, 2, 3.
//!
//! [`split_pivots`] returns the two thresholds used to bucket each element.
//! [`chunk_split`] is the partition loop — it walks the chunk once, sending
//! each element to one of three destinations.
//!
//! # What's a deviation from classic quicksort (algorithm-level)
//!
//! - **3-way partition (not 2-way).** Each split produces `min`, `mid`, `max`
//!   sub-chunks. Lower recursion depth and a better fit for the push_swap
//!   2-stack topology, since we can park two sub-chunks in B (one at top, one
//!   at bottom) while leaving the third in A.
//! - **Location-aware chunks.** A chunk lives at one of `TopA / BottomA / TopB
//!   / BottomB`. Bottom-chunks are reached with `rra/rrb`, so a recursive call
//!   can sort a chunk parked at the bottom of B without first unpacking it.
//! - **Pivot tuning** ([`split_pivots`]): B-chunks use 1/2 instead of 2/3 for
//!   `pivot_1` (B's "natural" ordering is reversed compared to A), and small
//!   `BottomB` chunks fall back to a 2-way split. Small A-chunks (< 15)
//!   collapse `mid` into `max` to avoid degenerate 1-element mid buckets.
//!
//! # Algorithm-specific optimizations (NOT covered by `optimizer.rs`)
//!
//! These depend on *which values are where* — information the value-blind
//! peephole optimizer can't access.
//!
//! - [`chunk_to_the_top`]: a `BottomX` chunk that fills its entire stack is
//!   relabeled `TopX`. One line of code, ~10% fewer ops at n=500. Best ROI
//!   optimization in the file.
//! - [`split_max_reduction`]: after a push to `max`, if A's top extends an
//!   already-sorted suffix (consecutive chain ending at the global max),
//!   shrink the `max` chunk — those elements are done.
//! - Location-specific base cases ([`sort_three_chunk`], [`sort_two_chunk`]):
//!   minimal op sequences hand-tuned for each of the 4 chunk locations.
//!
//! # Ulysse optimizations we deliberately *don't* port
//!
//! Measured to cost too much code for the ops they save (n=100/n=500 bench):
//!
//! - `easy_sort` / `easy_sort_second` — drain consecutive values directly:
//!   saved ~8 ops total (≈0.1%) for ~70 LOC.
//! - `sort_five` fast path — only triggers for n=5; recursion handles it.
//! - Complex sub-cases of `split_max_reduction` (consecutive 4-run sort,
//!   `sa`-then-shrink) — saved ~1.5% for ~20 LOC. Kept only the zero-op
//!   "shrink if top already sorted" sub-case.
//!
//! # Ulysse optimizations covered by `optimizer.rs`
//!
//! `post_sort_optimization` in the C code does two passes that our generic
//! optimizer subsumes (and goes beyond):
//!
//! - `eliminate_neutral_op` — cancels `ra/rra`, `sa/sa`, `pa/pb` inverse pairs.
//! - `merge_op` — fuses `ra+rb → rr`, `rra+rrb → rrr`, `sa+sb → ss`.
//!
//! `src/optimizer.rs` does both plus commutative reordering across stacks
//! and superoptimizer-derived multi-op rewrites. Emitting raw single-stack
//! ops here lets the optimizer cut ~10% of the op stream post-hoc.

use crate::stacks::{Operation, StackPair};

use super::sort_three::sort_three;

sort_name!();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Loc {
    TopA,
    BottomA,
    TopB,
    BottomB,
}

#[derive(Debug, Clone, Copy)]
struct Chunk {
    loc: Loc,
    size: usize,
}

pub fn sort_quick3(stacks: &mut StackPair) {
    let n = stacks.a().len();
    if n <= 1 || stacks.is_sorted() {
        return;
    }
    if n == 3 {
        sort_three(stacks);
        return;
    }
    let mut whole = Chunk {
        loc: Loc::TopA,
        size: n,
    };
    rec_chunk_sort(stacks, &mut whole, n);
}

// ---------------------------------------------------------------------------
// CLASSIC QUICKSORT SKELETON
// ---------------------------------------------------------------------------

/// Recursive 3-way quicksort. Classic structure: base case → partition →
/// recurse on each bucket. The single algorithm-specific optimization
/// (`chunk_to_the_top`) wraps the entry; everything else is vanilla
/// quicksort.
fn rec_chunk_sort(stacks: &mut StackPair, chunk: &mut Chunk, total_n: usize) {
    chunk_to_the_top(stacks, chunk); // OPT: relabel
    if chunk.size <= 3 {
        match chunk.size {
            3 => sort_three_chunk(stacks, chunk),
            2 => sort_two_chunk(stacks, chunk),
            1 => sort_one_chunk(stacks, chunk),
            _ => {}
        }
        return;
    }
    let (mut min, mut mid, mut max) = chunk_split(stacks, chunk, total_n);
    rec_chunk_sort(stacks, &mut max, total_n);
    rec_chunk_sort(stacks, &mut mid, total_n);
    rec_chunk_sort(stacks, &mut min, total_n);
}

/// Single partition step. Pivots split values into 3 buckets:
///
/// - `> max_value - pivot_2` (top third by value) → `max`
/// - `> max_value - pivot_1` (middle third) → `mid`
/// - else (bottom third) → `min`
///
/// Each element costs 1–3 ops to route to its destination ([`move_from_to`]).
/// The `split_max_reduction` and `easy_sort` hooks inside the loop are
/// optimizations on top of plain partitioning.
fn chunk_split(
    stacks: &mut StackPair,
    to_split: &mut Chunk,
    total_n: usize,
) -> (Chunk, Chunk, Chunk) {
    let from = to_split.loc;
    let (min_loc, mid_loc, max_loc) = split_locs(from);
    let (pivot_1, pivot_2) = split_pivots(from, to_split.size);
    let max_value = chunk_max_value(stacks, *to_split);

    let mut min = Chunk {
        loc: min_loc,
        size: 0,
    };
    let mut mid = Chunk {
        loc: mid_loc,
        size: 0,
    };
    let mut max = Chunk {
        loc: max_loc,
        size: 0,
    };

    while to_split.size > 0 {
        let next_value = chunk_value(stacks, *to_split, 0);
        to_split.size -= 1;
        if next_value + pivot_2 > max_value {
            move_from_to(stacks, from, max_loc);
            max.size += 1;
            // OPT: maybe trim `max` if A's suffix is already sorted.
            split_max_reduction(stacks, &mut max, total_n);
        } else if next_value + pivot_1 > max_value {
            move_from_to(stacks, from, mid_loc);
            mid.size += 1;
        } else {
            move_from_to(stacks, from, min_loc);
            min.size += 1;
        }
    }
    (min, mid, max)
}

/// Where each bucket lives, given the source chunk's location. Encodes the
/// 2-stack topology: from `TopA`, we send min → BottomB (sits below other B
/// stuff), mid → TopB (closer to A for cheap return), max → BottomA (stays
/// in A but out of the way).
fn split_locs(from: Loc) -> (Loc, Loc, Loc) {
    use Loc::*;
    match from {
        TopA => (BottomB, TopB, BottomA),
        BottomA => (BottomB, TopB, TopA),
        TopB => (BottomB, BottomA, TopA),
        BottomB => (TopB, BottomA, TopA),
    }
}

/// Pivot thresholds, expressed as offsets from `max_value` (so a value `v`
/// goes to `max` when `v + pivot_2 > max_value`, i.e. it's in the top
/// `pivot_2`-wide band). Different shapes for A vs B chunks (different
/// natural orderings), plus small-size carve-outs.
fn split_pivots(loc: Loc, size: usize) -> (usize, usize) {
    let mut pivot_2 = size / 3;
    let mut pivot_1 = match loc {
        Loc::TopA | Loc::BottomA => 2 * size / 3,
        Loc::TopB | Loc::BottomB => size / 2,
    };
    // OPT: very small A chunks — collapse `mid` into `max` (pivot_1 == size).
    if matches!(loc, Loc::TopA | Loc::BottomA) && size < 15 {
        pivot_1 = size;
    }
    // OPT: very small BottomB chunks — 2-way split (mid == max).
    if loc == Loc::BottomB && size < 8 {
        pivot_2 = size / 2;
    }
    (pivot_1, pivot_2)
}

// ---------------------------------------------------------------------------
// BASE CASES (chunk size 1, 2, 3) — location-aware op sequences
// ---------------------------------------------------------------------------
//
// All of these are algorithm-specific optimizations: rather than generically
// moving the chunk to TopA and sorting there, they sort in place using
// minimal op counts tuned for each location.

fn sort_one_chunk(stacks: &mut StackPair, chunk: &mut Chunk) {
    if chunk.loc != Loc::TopA {
        move_from_to(stacks, chunk.loc, Loc::TopA);
    }
    chunk.size -= 1;
}

fn sort_two_chunk(stacks: &mut StackPair, chunk: &mut Chunk) {
    if chunk.loc != Loc::TopA {
        move_from_to(stacks, chunk.loc, Loc::TopA);
        move_from_to(stacks, chunk.loc, Loc::TopA);
    }
    if stacks.a()[0] > stacks.a()[1] {
        stacks.execute(Operation::Sa);
    }
    chunk.size -= 2;
}

/// 3-element chunk sort. Peels the max value to position 2 of A using
/// location-specific minimal op sequences, then delegates the remaining 2
/// to [`sort_two_chunk`].
fn sort_three_chunk(stacks: &mut StackPair, chunk: &mut Chunk) {
    let max = chunk_max_value(stacks, *chunk);
    match chunk.loc {
        Loc::TopA => sort_three_top_a(stacks, max),
        Loc::BottomA => sort_three_bottom_a(stacks, max),
        Loc::TopB => sort_three_top_b(stacks, max),
        Loc::BottomB => sort_three_bottom_b(stacks, max),
    }
    chunk.loc = match chunk.loc {
        // BottomB ends with chunk at TopB (see body); others end at TopA.
        Loc::BottomB => Loc::TopB,
        _ => Loc::TopA,
    };
    chunk.size = 2;
    sort_two_chunk(stacks, chunk);
}

fn sort_three_top_a(stacks: &mut StackPair, max: usize) {
    let a = stacks.a();
    if a[0] == max {
        // [max, b, c, ...] → [b, c, max, ...]
        stacks.execute(Operation::Sa);
        stacks.execute(Operation::Ra);
        stacks.execute(Operation::Sa);
        stacks.execute(Operation::Rra);
    } else if a[1] == max {
        // [b, max, c, ...] → [b, c, max, ...]
        stacks.execute(Operation::Ra);
        stacks.execute(Operation::Sa);
        stacks.execute(Operation::Rra);
    }
    // else max already at position 2: no-op.
}

fn sort_three_bottom_a(stacks: &mut StackPair, max: usize) {
    stacks.execute(Operation::Rra);
    stacks.execute(Operation::Rra);
    let a = stacks.a();
    if a[0] == max {
        stacks.execute(Operation::Sa);
        stacks.execute(Operation::Rra);
    } else if a[1] == max {
        stacks.execute(Operation::Rra);
    } else {
        stacks.execute(Operation::Pb);
        stacks.execute(Operation::Rra);
        stacks.execute(Operation::Sa);
        stacks.execute(Operation::Pa);
    }
}

fn sort_three_top_b(stacks: &mut StackPair, max: usize) {
    stacks.execute(Operation::Pa);
    let b = stacks.b();
    if !b.is_empty() && b[0] == max {
        stacks.execute(Operation::Pa);
        stacks.execute(Operation::Sa);
    } else if b.len() >= 2 && b[1] == max {
        stacks.execute(Operation::Sb);
        stacks.execute(Operation::Pa);
        stacks.execute(Operation::Sa);
    } else {
        stacks.execute(Operation::Pa);
    }
    stacks.execute(Operation::Pa);
}

fn sort_three_bottom_b(stacks: &mut StackPair, max: usize) {
    stacks.execute(Operation::Rrb);
    stacks.execute(Operation::Rrb);
    let b = stacks.b();
    if b[0] == max {
        stacks.execute(Operation::Pa);
        stacks.execute(Operation::Rrb);
    } else if b.len() >= 2 && b[1] == max {
        stacks.execute(Operation::Sb);
        stacks.execute(Operation::Pa);
        stacks.execute(Operation::Rrb);
    } else {
        stacks.execute(Operation::Rrb);
        stacks.execute(Operation::Pa);
    }
}

// ---------------------------------------------------------------------------
// OPTIMIZATIONS that wrap the classic skeleton
// ---------------------------------------------------------------------------

/// OPT: relabel a Bottom* chunk that already covers its entire stack to Top*.
/// Saves op cost downstream (no `rra/rrb` needed to "reach" the chunk).
fn chunk_to_the_top(stacks: &StackPair, chunk: &mut Chunk) {
    if chunk.loc == Loc::BottomA && stacks.a().len() == chunk.size {
        chunk.loc = Loc::TopA;
    } else if chunk.loc == Loc::BottomB && stacks.b().len() == chunk.size {
        chunk.loc = Loc::TopB;
    }
}

/// OPT: after a push to `max`, if A's top now extends an already-sorted
/// consecutive suffix ending at the global max value, shrink the `max`
/// chunk by 1 — that element is in its final position, no recursion needed.
/// Zero ops emitted; pure bookkeeping.
fn split_max_reduction(stacks: &StackPair, max: &mut Chunk, total_n: usize) {
    if max.loc != Loc::TopA {
        return;
    }
    if a_partly_sort(stacks, total_n, 0) {
        max.size = max.size.saturating_sub(1);
    }
}

/// Is `a[from..]` a strictly ascending consecutive run ending at the global
/// max value (`total_n - 1`)? If yes, those elements are in final position.
fn a_partly_sort(stacks: &StackPair, total_n: usize, from: usize) -> bool {
    let a = stacks.a();
    if from >= a.len() {
        return false;
    }
    let mut i = from;
    while a[i] != total_n - 1 {
        let v = a[i];
        i += 1;
        if i >= a.len() {
            return false;
        }
        if a[i] != v + 1 {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Stack inspection + movement primitives (classic, no algorithm cleverness)
// ---------------------------------------------------------------------------

/// `n`th value into the chunk, counting from the chunk's near side
/// (top for `Top*`, bottom for `Bottom*`). `n = 0` is the near edge.
fn chunk_value(stacks: &StackPair, chunk: Chunk, n: usize) -> usize {
    let stk = match chunk.loc {
        Loc::TopA | Loc::BottomA => stacks.a(),
        Loc::TopB | Loc::BottomB => stacks.b(),
    };
    match chunk.loc {
        Loc::TopA | Loc::TopB => stk[n],
        Loc::BottomA | Loc::BottomB => stk[stk.len() - 1 - n],
    }
}

fn chunk_max_value(stacks: &StackPair, chunk: Chunk) -> usize {
    (0..chunk.size)
        .map(|i| chunk_value(stacks, chunk, i))
        .max()
        .unwrap_or(0)
}

/// Move one element from `from`'s near side to `to`'s near side, using 1–3
/// raw ops. No `from == to` case — partitioning never asks for that.
fn move_from_to(stacks: &mut StackPair, from: Loc, to: Loc) {
    use Loc::*;
    use Operation::*;
    let ops: &[Operation] = match (from, to) {
        (TopA, TopB) => &[Pb],
        (TopA, BottomA) => &[Ra],
        (TopA, BottomB) => &[Pb, Rb],
        (TopB, TopA) => &[Pa],
        (TopB, BottomB) => &[Rb],
        (TopB, BottomA) => &[Pa, Ra],
        (BottomA, TopA) => &[Rra],
        (BottomA, TopB) => &[Rra, Pb],
        (BottomA, BottomB) => &[Rra, Pb, Rb],
        (BottomB, TopB) => &[Rrb],
        (BottomB, TopA) => &[Rrb, Pa],
        (BottomB, BottomA) => &[Rrb, Pa, Ra],
        _ => unreachable!("move_from_to: {:?} -> {:?}", from, to),
    };
    for &op in ops {
        stacks.execute(op);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_random;

    #[test]
    fn random_inputs() {
        assert_sorts_random(&[100, 500], 10, sort_quick3);
    }
}
