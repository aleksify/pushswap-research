//! Vanilla chunk sort.
//!
//! Partition rank space `0..n` into `√n` equal-size buckets. Walk A; if the
//! top falls in the next-unfinished bucket, `pb`; else `ra`. Repeat per
//! bucket low-to-high. Then pull max of B back to A until B empty.

use crate::stacks::{Operation, RotateExt, StackExt, StackPair};

sort_name!();

pub fn sort_chunk(stacks: &mut StackPair) {
    let n = stacks.a().len();
    if n <= 1 || stacks.is_sorted() {
        return;
    }
    let chunk = n.isqrt().max(1);
    let k = n.div_ceil(chunk);

    for c in 0..k {
        let lo = c * chunk;
        let hi = ((c + 1) * chunk).min(n);
        while stacks.a().iter().any(|&v| v >= lo && v < hi) {
            let top = stacks.a()[0];
            if top >= lo && top < hi {
                stacks.execute(Operation::Pb);
            } else {
                stacks.execute(Operation::Ra);
            }
        }
    }

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
        assert_sorts_random(&[100, 500], 10, sort_chunk);
    }
}
