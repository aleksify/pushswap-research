use crate::stacks::{Log, Operation};
use Operation::*;

/// Tier 1 — adjacent pairs that cancel to nothing.
///    a     b
const CANCELLATIONS: &[(Operation, Operation)] = &[
    (Sa,  Sa ),
    (Sb,  Sb ),
    (Ss,  Ss ),
    (Pa,  Pb ),  (Pb,  Pa ),
    (Ra,  Rra),  (Rra, Ra ),
    (Rb,  Rrb),  (Rrb, Rb ),
    (Rr,  Rrr),  (Rrr, Rr ),
];

/// Tiers 2+3 — adjacent pairs that rewrite to a single op.
///    a     b      result
const PAIR_REWRITES: &[(Operation, Operation, Operation)] = &[
    // Tier 2: merge A-only + B-only → combined
    (Sa,  Sb,  Ss ),  (Sb,  Sa,  Ss ),
    (Ra,  Rb,  Rr ),  (Rb,  Ra,  Rr ),
    (Rra, Rrb, Rrr),  (Rrb, Rra, Rrr),
    // Tier 3: combined + half → other half
    (Ss,  Sa,  Sb ),  (Sa,  Ss,  Sb ),
    (Ss,  Sb,  Sa ),  (Sb,  Ss,  Sa ),
    (Rr,  Rra, Rb ),  (Rra, Rr,  Rb ),
    (Rr,  Rrb, Ra ),  (Rrb, Rr,  Ra ),
    (Rrr, Ra,  Rrb),  (Ra,  Rrr, Rrb),
    (Rrr, Rb,  Rra),  (Rb,  Rrr, Rra),
];

/// Tier 5 — triples that rewrite to pairs.
///    a    b    c          r1   r2
type Triple = ((Operation, Operation, Operation), (Operation, Operation));
const TRIPLE_REWRITES: &[Triple] = &[
    ((Ra, Pb, Rra),  (Sa, Pb)),
    ((Rb, Pa, Rrb),  (Sb, Pa)),
    ((Ra, Pa, Rra),  (Pa, Sa)),
    ((Rb, Pb, Rrb),  (Pb, Sb)),
];

/// Tier 4 — which A/B op pairs can be zipped into combined ops.
///    a     b      combined
const MERGE_PAIRS: &[(Operation, Operation, Operation)] = &[
    (Sa,  Sb,  Ss ),
    (Ra,  Rb,  Rr ),
    (Rra, Rrb, Rrr),
];

// ====================================================================
// Rule lookups
// ====================================================================

fn cancels(a: Operation, b: Operation) -> bool {
    CANCELLATIONS.iter().any(|&(x, y)| x == a && y == b)
}

fn pair_rewrite(a: Operation, b: Operation) -> Option<Operation> {
    PAIR_REWRITES
        .iter()
        .find(|&&(x, y, _)| x == a && y == b)
        .map(|&(_, _, result)| result)
}

fn triple_rewrite(
    a: Operation,
    b: Operation,
    c: Operation,
) -> Option<(Operation, Operation)> {
    TRIPLE_REWRITES
        .iter()
        .find(|&&((x, y, z), _)| x == a && y == b && z == c)
        .map(|&(_, result)| result)
}

// ====================================================================
// Operation classification
// ====================================================================

fn is_a_only(op: Operation) -> bool {
    matches!(op, Sa | Ra | Rra)
}

fn is_b_only(op: Operation) -> bool {
    matches!(op, Sb | Rb | Rrb)
}

fn is_barrier(op: Operation) -> bool {
    matches!(op, Ss | Rr | Rrr | Pa | Pb)
}

// ====================================================================
// Navigation helpers
// ====================================================================

fn next_exec(logs: &[Log], from: usize) -> Option<usize> {
    (from..logs.len()).find(|&i| matches!(logs[i], Log::Execute(_)))
}

fn prev_exec(logs: &[Log], before: usize) -> Option<usize> {
    (0..before).rev().find(|&i| matches!(logs[i], Log::Execute(_)))
}

fn op_at(logs: &[Log], idx: usize) -> Operation {
    match logs[idx] {
        Log::Execute(op) | Log::Ignore(op) => op,
    }
}

// ====================================================================
// Tier 4 helpers
// ====================================================================

/// Reduce a single-stack op subsequence by canceling adjacent inverse pairs.
/// Uses a stack: push each op, pop if top cancels with incoming op.
fn reduce_pure(ops: &[Operation]) -> Vec<Operation> {
    let mut stack: Vec<Operation> = Vec::with_capacity(ops.len());
    for &op in ops {
        if let Some(&top) = stack.last()
            && cancels(top, op)
        {
            stack.pop();
            continue;
        }
        stack.push(op);
    }
    stack
}

/// Merge matching A/B op pairs into combined ops (Tier 2 zip).
/// E.g. [ra,ra] + [rb,rb] → [rr,rr]; [ra,ra,ra] + [rb] → [rr,ra,ra].
fn zip_tier2_merge(a_ops: &[Operation], b_ops: &[Operation]) -> Vec<Operation> {
    let mut result = Vec::new();
    let mut b_remaining: Vec<Operation> = b_ops.to_vec();

    // For each merge rule, pair up matching A and B ops
    for &(a_match, b_match, combined) in MERGE_PAIRS {
        let a_count = a_ops.iter().filter(|&&op| op == a_match).count();
        let b_count = b_remaining.iter().filter(|&&op| op == b_match).count();
        let pairs = a_count.min(b_count);
        result.extend(std::iter::repeat_n(combined, pairs));
        // Remove paired B ops
        let mut removed = 0;
        b_remaining.retain(|&op| {
            if op == b_match && removed < pairs {
                removed += 1;
                false
            } else {
                true
            }
        });
    }

    // Unpaired A ops
    for &(a_match, b_match, _) in MERGE_PAIRS {
        let a_count = a_ops.iter().filter(|&&op| op == a_match).count();
        let b_count = b_ops.iter().filter(|&&op| op == b_match).count();
        let pairs = a_count.min(b_count);
        result.extend(std::iter::repeat_n(a_match, a_count - pairs));
    }

    // Unpaired B ops
    result.extend(b_remaining);

    result
}

// ====================================================================
// Passes
// ====================================================================

/// Tiers 1-3: scan adjacent active op pairs and apply rewrite rules.
/// Steps back on rewrite to catch cascading reductions.
fn pass_adjacent(logs: &mut [Log]) -> bool {
    let mut changed = false;
    let Some(mut i) = next_exec(logs, 0) else {
        return false;
    };

    while let Some(j) = next_exec(logs, i + 1) {
        let a = op_at(logs, i);
        let b = op_at(logs, j);

        if cancels(a, b) {
            logs[i] = Log::Ignore(a);
            logs[j] = Log::Ignore(b);
            changed = true;
            match prev_exec(logs, i) {
                Some(prev) => i = prev,
                None => match next_exec(logs, 0) {
                    Some(first) => i = first,
                    None => break,
                },
            }
        } else if let Some(replacement) = pair_rewrite(a, b) {
            logs[i] = Log::Execute(replacement);
            logs[j] = Log::Ignore(b);
            changed = true;
            if let Some(prev) = prev_exec(logs, i) {
                i = prev;
            }
        } else {
            i = j;
        }
    }

    changed
}

/// Tier 4: exploit A/B commutativity within blocks between barriers.
fn pass_tier4(logs: &mut [Log]) -> bool {
    let mut changed = false;

    let active: Vec<usize> = (0..logs.len())
        .filter(|&i| matches!(logs[i], Log::Execute(_)))
        .collect();

    if active.is_empty() {
        return false;
    }

    let mut blocks: Vec<Vec<usize>> = Vec::new();
    let mut current_block: Vec<usize> = Vec::new();

    for &idx in &active {
        let op = op_at(logs, idx);
        if is_barrier(op) {
            if current_block.len() >= 2 {
                blocks.push(std::mem::take(&mut current_block));
            } else {
                current_block.clear();
            }
        } else {
            current_block.push(idx);
        }
    }
    if current_block.len() >= 2 {
        blocks.push(current_block);
    }

    for block_indices in &blocks {
        let mut a_ops: Vec<Operation> = Vec::new();
        let mut b_ops: Vec<Operation> = Vec::new();

        for &idx in block_indices {
            let op = op_at(logs, idx);
            if is_a_only(op) {
                a_ops.push(op);
            } else if is_b_only(op) {
                b_ops.push(op);
            }
        }

        let a_reduced = reduce_pure(&a_ops);
        let b_reduced = reduce_pure(&b_ops);
        let merged = zip_tier2_merge(&a_reduced, &b_reduced);

        let mut block_changed = false;
        for (k, &new_op) in merged.iter().enumerate() {
            if op_at(logs, block_indices[k]) != new_op {
                block_changed = true;
            }
            logs[block_indices[k]] = Log::Execute(new_op);
        }
        for k in merged.len()..block_indices.len() {
            block_changed = true;
            let old_op = op_at(logs, block_indices[k]);
            logs[block_indices[k]] = Log::Ignore(old_op);
        }
        changed |= block_changed;
    }

    changed
}

/// Tier 5: scan windows of three active ops for rotation-around-push identities.
fn pass_tier5(logs: &mut [Log]) -> bool {
    let mut changed = false;
    let Some(mut i) = next_exec(logs, 0) else {
        return false;
    };

    while let Some(j) = next_exec(logs, i + 1) {
        let Some(k) = next_exec(logs, j + 1) else {
            break;
        };

        let a = op_at(logs, i);
        let b = op_at(logs, j);
        let c = op_at(logs, k);

        if let Some((new1, new2)) = triple_rewrite(a, b, c) {
            logs[i] = Log::Execute(new1);
            logs[j] = Log::Execute(new2);
            logs[k] = Log::Ignore(c);
            changed = true;
            if let Some(prev) = prev_exec(logs, i) {
                i = prev;
            }
        } else {
            i = j;
        }
    }

    changed
}

// ====================================================================
// Main entry point
// ====================================================================

/// Optimize a log sequence by replacing redundant ops with Ignore.
/// Runs Tiers 1-5 in a fixed-point loop until no more rewrites fire.
pub fn optimize(mut logs: Vec<Log>) -> Vec<Log> {
    loop {
        let mut changed = false;
        changed |= pass_adjacent(&mut logs);
        changed |= pass_tier4(&mut logs);
        changed |= pass_tier5(&mut logs);
        if !changed {
            break;
        }
    }
    logs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stacks::StackPair;
    use std::collections::VecDeque;

    fn make_stacks() -> StackPair {
        let mut s = StackPair::new(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        s.execute(Pb);
        s.execute(Pb);
        s.execute(Pb);
        s
    }

    fn snapshot(s: &StackPair) -> (VecDeque<usize>, VecDeque<usize>) {
        (s.a().clone(), s.b().clone())
    }

    fn run_ops(base: &StackPair, ops: &[Operation]) -> (VecDeque<usize>, VecDeque<usize>) {
        let mut s = base.clone();
        for &op in ops {
            s.execute(op);
        }
        snapshot(&s)
    }

    fn assert_identity(ops: &[Operation]) {
        let s = make_stacks();
        let before = snapshot(&s);
        let after = run_ops(&s, ops);
        assert_eq!(before, after, "{ops:?} should be identity");
    }

    fn assert_equivalent(lhs: &[Operation], rhs: &[Operation]) {
        let s = make_stacks();
        assert_eq!(
            run_ops(&s, lhs),
            run_ops(&s, rhs),
            "{lhs:?} should equal {rhs:?}"
        );
    }

    fn exec_ops(logs: &[Log]) -> Vec<Operation> {
        logs.iter()
            .filter_map(|l| match l {
                Log::Execute(op) => Some(*op),
                _ => None,
            })
            .collect()
    }

    fn make_logs(ops: &[Operation]) -> Vec<Log> {
        ops.iter().map(|&op| Log::Execute(op)).collect()
    }

    fn assert_optimizes_to(input: &[Operation], expected: &[Operation]) {
        assert_eq!(
            exec_ops(&optimize(make_logs(input))),
            expected,
            "optimize({input:?})"
        );
    }

    // ================================================================
    // Tier 1: canceling pairs are identity, optimizer eliminates them
    // ================================================================

    #[test]
    fn tier1_cancellations() {
        for &(a, b) in CANCELLATIONS {
            assert_identity(&[a, b]);
            assert_optimizes_to(&[a, b], &[]);
        }
    }

    // ================================================================
    // Tiers 2+3: pair rewrites equivalent, optimizer applies them
    // ================================================================

    #[test]
    fn tier2_and_tier3_pair_rewrites() {
        for &(a, b, result) in PAIR_REWRITES {
            assert_equivalent(&[a, b], &[result]);
            assert_optimizes_to(&[a, b], &[result]);
        }
    }

    // ================================================================
    // Tier 4: non-adjacent cancellation via commutativity
    // ================================================================

    #[test]
    fn tier4_cancel_across_commuting() {
        assert_equivalent(&[Ra, Sb, Rra], &[Sb]);
        assert_optimizes_to(&[Ra, Sb, Rra], &[Sb]);
    }

    #[test]
    fn tier4_merge_across_commuting() {
        let result = optimize(make_logs(&[Ra, Sb, Rb]));
        assert_eq!(exec_ops(&result).len(), 2);
    }

    #[test]
    fn tier4_worked_example() {
        // From spec: ra sb rra rb pb → sb rb pb (5 → 3)
        let ops = [Ra, Sb, Rra, Rb, Pb];
        let base = make_stacks();
        let opt = exec_ops(&optimize(make_logs(&ops)));
        assert_eq!(opt.len(), 3);
        assert_eq!(run_ops(&base, &ops), run_ops(&base, &opt));
    }

    // ================================================================
    // Tier 5: rotation-around-push identities
    // ================================================================

    #[test]
    fn tier5_rewrites() {
        for &((a, b, c), (r1, r2)) in TRIPLE_REWRITES {
            assert_equivalent(&[a, b, c], &[r1, r2]);
            assert_optimizes_to(&[a, b, c], &[r1, r2]);
        }
    }

    // ================================================================
    // Cascading / fixed-point
    // ================================================================

    #[test]
    fn cascade_merge_then_decompose() {
        assert_equivalent(&[Ra, Rb, Rra], &[Rb]);
        assert_optimizes_to(&[Ra, Rb, Rra], &[Rb]);
    }

    #[test]
    fn cascade_multiple_cancellations() {
        assert_optimizes_to(&[Sa, Sa, Ra, Rra], &[]);
    }

    // ================================================================
    // Edge cases
    // ================================================================

    #[test]
    fn empty_input() {
        assert!(optimize(vec![]).is_empty());
    }

    #[test]
    fn single_op_unchanged() {
        assert_optimizes_to(&[Ra], &[Ra]);
    }

    #[test]
    fn no_optimization_possible() {
        assert_optimizes_to(&[Ra, Pa, Sb], &[Ra, Pa, Sb]);
    }

    #[test]
    fn ignores_preserved() {
        let logs = vec![Log::Ignore(Sa), Log::Execute(Ra), Log::Execute(Rra)];
        let result = optimize(logs);
        assert_eq!(exec_ops(&result), vec![]);
        assert!(matches!(result[0], Log::Ignore(_)));
    }

    #[test]
    fn complex_sequence_preserves_semantics() {
        let ops = [Ra, Sb, Rra, Rb, Sa, Sa, Pb, Rr, Rrr, Ra, Pb, Rra];
        let base = make_stacks();
        let opt = exec_ops(&optimize(make_logs(&ops)));
        assert_eq!(run_ops(&base, &ops), run_ops(&base, &opt));
        assert!(opt.len() <= ops.len());
    }
}
