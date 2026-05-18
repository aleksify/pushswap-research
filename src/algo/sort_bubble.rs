//! **Not included in the binary** — performance is abysmal (O(n²) with large
//! constants; every other algorithm in the repo beats this for n > ~10). The
//! file is kept here only as a reference implementation of the classical
//! algorithm; the module is not wired into `algo.rs`.
//!
//! Cyclic bubble sort.
//!
//! One pass: `n-1` rounds of "compare A[0] vs A[1], `sa` if out of order,
//! `ra`", then one trailing `ra` to skip the wraparound boundary (which would
//! otherwise undo the legitimate max→min step at the seam). Total `n` ra's
//! per pass means stack framing returns to its starting rotation after each
//! pass. Loop until a pass completes with no `sa`.
//!
//! O(n²) ops. Educational baseline — every other algorithm in the repo beats
//! this for n > ~10. Included for the "classical sorts" set.

use crate::stacks::{Operation, StackPair};

sort_name!();

pub fn sort_bubble(stacks: &mut StackPair) {
    let n = stacks.a().len();
    if n <= 1 {
        return;
    }
    loop {
        let mut swapped = false;
        for _ in 0..n - 1 {
            if stacks.a()[0] > stacks.a()[1] {
                stacks.execute(Operation::Sa);
                swapped = true;
            }
            stacks.execute(Operation::Ra);
        }
        stacks.execute(Operation::Ra);
        if !swapped {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_random;

    #[test]
    fn random_inputs() {
        assert_sorts_random(&[100, 500], 10, sort_bubble);
    }
}
