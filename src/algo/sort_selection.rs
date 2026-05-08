use crate::stacks::{Operation, RotateExt, StackExt, StackPair};

use super::sort_three::sort_three;

/// Selection sort: push smallest to B one by one, sort_three remainder, push back.
pub fn sort_selection(stacks: &mut StackPair) {
    if stacks.a().len() <= 3 {
        sort_three(stacks);
        return;
    }
    let pushes_needed = stacks.a().len() - 3;
    for _ in 0..pushes_needed {
        let pos = stacks.a().min_pos();
        stacks.rotate_a_to_top(pos);
        stacks.execute(Operation::Pb);
    }
    sort_three(stacks);
    for _ in 0..pushes_needed {
        stacks.execute(Operation::Pa);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_all;

    #[test]
    fn all_permutations() {
        assert_sorts_all(1, 5, sort_selection);
    }
}
