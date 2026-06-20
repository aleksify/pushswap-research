//! Reverse-solve: an orthogonal racing axis (idea N2).
//!
//! The configuration graph is undirected — every operation has a single-op
//! inverse — so sorting an arrangement `P` is the same group problem as sorting
//! its inverse permutation `P⁻¹`, only traversed the other way. Concretely: if a
//! sequence `O'` sorts `P⁻¹`, then reversing `O'` and inverting each op yields a
//! sequence that sorts `P`.
//!
//! Optimal lengths are identical in both directions, but a *heuristic* explores
//! different paths forward vs backward, so the reverse twin sometimes lands
//! shorter than the same algorithm run forward. It costs nothing but a second
//! racer.

use crate::stacks::{Operation, StackPair};

/// The single-op inverse of each operation. `Sa`/`Sb`/`Ss` are self-inverse;
/// pushes swap stacks; rotations flip direction.
pub fn invert_op(op: Operation) -> Operation {
    match op {
        Operation::Sa => Operation::Sa,
        Operation::Sb => Operation::Sb,
        Operation::Ss => Operation::Ss,
        Operation::Pa => Operation::Pb,
        Operation::Pb => Operation::Pa,
        Operation::Ra => Operation::Rra,
        Operation::Rb => Operation::Rrb,
        Operation::Rr => Operation::Rrr,
        Operation::Rra => Operation::Ra,
        Operation::Rrb => Operation::Rb,
        Operation::Rrr => Operation::Rr,
    }
}

/// Inverse permutation of a rank array (a permutation of `0..n`).
fn inverse_perm(ranked: &[usize]) -> Vec<usize> {
    let mut inv = vec![0usize; ranked.len()];
    for (i, &r) in ranked.iter().enumerate() {
        inv[r] = i;
    }
    inv
}

/// Solve `ranked` by running `algo` on the inverse permutation and
/// reverse-inverting the resulting op sequence (the "un-sort" framing).
///
/// Returns a [`StackPair`] holding the original input with the transformed log
/// applied. If the framing is valid the pair is sorted; correctness is also
/// guarded by the external `checker` in the race.
pub fn reverse_solve<F: Fn(&mut StackPair)>(algo: F, ranked: &[usize]) -> StackPair {
    let mut inv_stacks = StackPair::new(inverse_perm(ranked));
    algo(&mut inv_stacks);

    let ops: Vec<Operation> = inv_stacks
        .logs()
        .iter()
        .rev()
        .map(|&op| invert_op(op))
        .collect();

    let mut out = StackPair::new(ranked.to_vec());
    for op in ops {
        out.execute(op);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::Algorithm;

    fn check(algo: Algorithm, ranked: &[usize]) {
        let out = reverse_solve(algo.sort(), ranked);
        assert!(
            out.is_sorted(),
            "reverse_solve({}) failed to sort {ranked:?}",
            algo.name()
        );
    }

    #[test]
    fn reverse_solve_sorts_small() {
        let cases: &[&[usize]] = &[
            &[0, 1, 2],
            &[2, 1, 0],
            &[2, 0, 1],
            &[1, 2, 0],
            &[3, 1, 4, 0, 2],
            &[4, 3, 2, 1, 0],
        ];
        for algo in Algorithm::ALL {
            for c in cases {
                check(*algo, c);
            }
        }
    }

    #[test]
    fn reverse_solve_sorts_seeded_shuffles() {
        // Deterministic LCG shuffle, no external rng dependency.
        let mut state: u64 = 0x9E3779B97F4A7C15;
        let mut next = |bound: usize| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((state >> 33) as usize) % bound
        };
        for _ in 0..20 {
            let n = 5 + next(20);
            let mut v: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                v.swap(i, next(i + 1));
            }
            for algo in Algorithm::ALL {
                check(*algo, &v);
            }
        }
    }
}
