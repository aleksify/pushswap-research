//! Binary LSD radix sort.
//!
//! # Classic radix sort
//!
//! For each bit position k from least to most significant: stable-partition
//! elements into "bit k = 0" and "bit k = 1" buckets. After processing all
//! bits, sequence is sorted.
//!
//! # Push_swap encoding
//!
//! Values are rank-normalized to `0..n-1`, so max value is `n-1` and we
//! need `⌈log₂(n)⌉` passes.
//!
//! Each pass walks A's top `n` elements once:
//! - bit k is 1 → `ra` (element cycles to bottom of A; relative order of
//!   1-bit elements preserved)
//! - bit k is 0 → `pb` (element pushed onto top of B; 0-bit elements end
//!   up in B in *reversed* relative order)
//!
//! Then `pa` until B is empty. This re-reverses the 0-bit elements, so
//! they land on top of A in their original relative order, with the 1-bit
//! elements below. Result: A is now stable-partitioned by bit k.
//!
//! # Op count
//!
//! Each pass: exactly `n` rotate-or-push ops + `(count of 0-bit elements)`
//! `pa` ops. Upper bound `2n` per pass, total `~2n · log₂(n)`.
//! Asymptotically worse than turk/k_chunk/quick3 for n ≥ ~50, but the
//! implementation is trivial.

use crate::stacks::{Operation, StackPair};

sort_name!();

pub fn sort_radix(stacks: &mut StackPair) {
    let n = stacks.a().len();
    if n <= 1 || stacks.is_sorted() {
        return;
    }
    let max_bits = (usize::BITS - (n - 1).leading_zeros()) as usize;
    for bit in 0..max_bits {
        for _ in 0..n {
            let top = stacks.a()[0];
            if (top >> bit) & 1 == 1 {
                stacks.execute(Operation::Ra);
            } else {
                stacks.execute(Operation::Pb);
            }
        }
        while !stacks.b().is_empty() {
            stacks.execute(Operation::Pa);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_random;

    #[test]
    fn random_inputs() {
        assert_sorts_random(&[100, 500], 10, sort_radix);
    }
}
