use crate::stacks::{Operation, RotateExt, StackExt, StackPair};

/// Insertion sort: push unsorted to B, insert each back at correct position.
pub fn sort_insert(stacks: &mut StackPair) {
    push_unsorted(stacks);
    while !stacks.b().is_empty() {
        let val = stacks.b()[0];
        let pos = stacks.a().min_above_pos(val);
        stacks.rotate_a_to_top(pos);
        stacks.execute(Operation::Pa);
    }
    let min = stacks.a().min_pos();
    stacks.rotate_a_to_top(min);
}

/// Push elements that break ascending order to B, keep sorted tail in A.
fn push_unsorted(stacks: &mut StackPair) {
    let n = stacks.a().len();
    let mut max_seen = stacks.a()[0];
    for _ in 0..n {
        if stacks.a()[0] >= max_seen {
            max_seen = stacks.a()[0];
            stacks.execute(Operation::Ra);
        } else {
            stacks.execute(Operation::Pb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_random;

    #[test]
    fn random_inputs() {
        assert_sorts_random(&[100, 500], 10, sort_insert);
    }
}
