use crate::stacks::{Operation, RotateExt, StackExt, StackPair};

/// Chunk sort: split into sqrt-sized ranges, push to B, pop max back.
pub fn sort_chunk(stacks: &mut StackPair) {
    push_chunks_to_b(stacks);
    push_back_to_a(stacks);
}

/// Push A to B in chunks sized by sqrt(n), smaller values rotate to bottom.
fn push_chunks_to_b(stacks: &mut StackPair) {
    let chunk = stacks.a().len().isqrt() * 14 / 10;
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
        assert_sorts_random(&[100, 500], 10, sort_chunk);
    }
}
