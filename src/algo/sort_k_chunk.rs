//! K-chunk sort: chunk sort with a "K shape" on B.
//!
//! Same skeleton as `sort_chunk` (sqrt-sized rank buckets, then max-to-top
//! back-push), with one twist on the forward pass: when the top of A holds a
//! value below the bucket currently being processed (i.e. a value already
//! `pb`'d in a *prior* bucket would have been a candidate, but here we use
//! a running `pushed` watermark), we `pb` *then* `rb`. That shoves the
//! "already-handled low values" to the bottom of B so the current bucket's
//! values stack on top.
//!
//! Visualised, B grows as a K: bucket 0 at the bottom, bucket 1 layered above,
//! …, current bucket at the top. Result: when the back-push phase begins, B
//! is already roughly ordered by chunk, so the max-to-top pulls travel
//! shorter distances on average than in vanilla `sort_chunk`.
//!
//! Chunk width is `sqrt(n) * 1.4` (empirical sweet spot — slightly wider
//! buckets reduce the number of forward passes without bloating back-push
//! rotation distance).

use crate::stacks::{Operation, RotateExt, StackExt, StackPair};

sort_name!();

/// Default chunk-width factor in tenths: `isqrt(n) * 1.4`.
pub const DEFAULT_FACTOR_TENTHS: usize = 14;

pub fn sort_k_chunk(stacks: &mut StackPair) {
    sort_k_chunk_with(stacks, DEFAULT_FACTOR_TENTHS);
}

/// K-chunk with a tunable chunk-width factor (in tenths of `isqrt(n)`), so the
/// `main.rs` race can search over chunk widths (idea H6). `factor_tenths = 14`
/// reproduces [`sort_k_chunk`].
pub fn sort_k_chunk_with(stacks: &mut StackPair, factor_tenths: usize) {
    push_chunks_to_b(stacks, factor_tenths);
    push_back_to_a(stacks);
}

/// Push A to B in chunks sized by sqrt(n), smaller values rotate to bottom.
fn push_chunks_to_b(stacks: &mut StackPair, factor_tenths: usize) {
    let chunk = (stacks.a().len().isqrt() * factor_tenths / 10).max(1);
    let mut pushed = 0;
    while let Some(&val) = stacks.a().front() {
        if val <= pushed {
            stacks.execute(Operation::Pb);
            stacks.execute(Operation::Rb);
            pushed += 1;
        } else if val <= pushed + chunk {
            stacks.execute(Operation::Pb);
            pushed += 1;
        } else {
            stacks.execute(Operation::Ra);
        }
    }
}

/// Push B back to A by repeatedly moving max to top.
fn push_back_to_a(stacks: &mut StackPair) {
    while !stacks.b().is_empty() {
        let max = stacks.b().max_pos();
        stacks.rotate_b_to_top(max);
        stacks.execute(Operation::Pa);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_random;

    #[test]
    fn random_inputs() {
        assert_sorts_random(&[100, 500], 10, sort_k_chunk);
    }
}
