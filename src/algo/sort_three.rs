use crate::stacks::{Operation, StackPair};

fn swap_if_needed(stacks: &mut StackPair) {
    if stacks.a().len() >= 2 && stacks.a()[0] > stacks.a()[1] {
        stacks.execute(Operation::Sa);
    }
}

/// Sort 1-3 elements on stack A using swaps and rotations.
pub fn sort_three(stacks: &mut StackPair) {
    if stacks.a().len() <= 1 {
        return;
    }
    if stacks.a().len() == 2 {
        swap_if_needed(stacks);
        return;
    }
    let top = stacks.a()[0];
    let mid = stacks.a()[1];
    let bot = stacks.a()[2];
    if top > mid && top > bot {
        stacks.execute(Operation::Ra);
    } else if mid > top && mid > bot {
        stacks.execute(Operation::Rra);
    }
    swap_if_needed(stacks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_all;

    #[test]
    fn all_permutations() {
        assert_sorts_all(1, 3, sort_three);
    }
}
