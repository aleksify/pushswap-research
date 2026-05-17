use crate::stacks::{Operation, StackPair};

sort_name!();

/// Classic recursive quicksort. Median pivot, partition into B, recurse.
pub fn sort_quick2(stacks: &mut StackPair) {
    let n = stacks.a().len();
    quick_a(stacks, n);
}

fn median_top_a(stacks: &StackPair, n: usize) -> usize {
    let mut vals: Vec<usize> = (0..n).map(|i| stacks.a()[i]).collect();
    vals.sort_unstable();
    vals[n / 2]
}

fn median_top_b(stacks: &StackPair, n: usize) -> usize {
    let mut vals: Vec<usize> = (0..n).map(|i| stacks.b()[i]).collect();
    vals.sort_unstable();
    vals[n / 2]
}

/// Sort top `n` of A in place. Leaves elements below position `n` untouched.
fn quick_a(stacks: &mut StackPair, n: usize) {
    if n <= 1 {
        return;
    }
    if n == 2 {
        if stacks.a()[0] > stacks.a()[1] {
            stacks.execute(Operation::Sa);
        }
        return;
    }
    let m = stacks.a().len();
    let pivot = median_top_a(stacks, n);
    let mut pushed = 0;
    for _ in 0..n {
        if stacks.a()[0] < pivot {
            stacks.execute(Operation::Pb);
            pushed += 1;
        } else {
            stacks.execute(Operation::Ra);
        }
    }
    let high = n - pushed;
    let low = pushed;
    let fwd = m - n;
    if fwd <= high {
        for _ in 0..fwd {
            stacks.execute(Operation::Ra);
        }
    } else {
        for _ in 0..high {
            stacks.execute(Operation::Rra);
        }
    }
    quick_a(stacks, high);
    quick_b_to_a(stacks, low);
}

/// Move top `n` of B onto A so the new top `n` of A is sorted ascending.
/// Assumes every value in top `n` of B is smaller than the current top of A.
fn quick_b_to_a(stacks: &mut StackPair, n: usize) {
    if n == 0 {
        return;
    }
    if n == 1 {
        stacks.execute(Operation::Pa);
        return;
    }
    if n == 2 {
        if stacks.b()[0] < stacks.b()[1] {
            stacks.execute(Operation::Sb);
        }
        stacks.execute(Operation::Pa);
        stacks.execute(Operation::Pa);
        return;
    }
    let mb = stacks.b().len();
    let pivot = median_top_b(stacks, n);
    let mut popped = 0;
    for _ in 0..n {
        if stacks.b()[0] >= pivot {
            stacks.execute(Operation::Pa);
            popped += 1;
        } else {
            stacks.execute(Operation::Rb);
        }
    }
    let remaining = n - popped;
    let fwd = mb - n;
    if fwd <= remaining {
        for _ in 0..fwd {
            stacks.execute(Operation::Rb);
        }
    } else {
        for _ in 0..remaining {
            stacks.execute(Operation::Rrb);
        }
    }
    quick_a(stacks, popped);
    quick_b_to_a(stacks, remaining);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::test_utils::assert_sorts_random;

    #[test]
    fn random_inputs() {
        assert_sorts_random(&[100, 500], 10, sort_quick2);
    }
}
