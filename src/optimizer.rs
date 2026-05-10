use crate::stacks::{Log, Operation};
use Operation::*;

// ---- Operation classification ----

fn is_a_only(op: Operation) -> bool {
    matches!(op, Sa | Ra | Rra)
}

fn is_b_only(op: Operation) -> bool {
    matches!(op, Sb | Rb | Rrb)
}

fn is_barrier(op: Operation) -> bool {
    matches!(
        op,
        Ss | Rr | Rrr | Pa | Pb
    )
}

// ---- Navigation helpers ----

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

// ---- Tier 1: Adjacent cancellation (length-2 → 0) ----

fn tier1_cancel(a: Operation, b: Operation) -> bool {

    matches!(
        (a, b),
        (Sa, Sa)
            | (Sb, Sb)
            | (Ss, Ss)
            | (Pa, Pb)
            | (Pb, Pa)
            | (Ra, Rra)
            | (Rra, Ra)
            | (Rb, Rrb)
            | (Rrb, Rb)
            | (Rr, Rrr)
            | (Rrr, Rr)
    )
}

// ---- Tier 2: Adjacent merge (length-2 → 1) ----

fn tier2_merge(a: Operation, b: Operation) -> Option<Operation> {

    match (a, b) {
        (Sa, Sb) | (Sb, Sa) => Some(Ss),
        (Ra, Rb) | (Rb, Ra) => Some(Rr),
        (Rra, Rrb) | (Rrb, Rra) => Some(Rrr),
        _ => None,
    }
}

// ---- Tier 3: Decomposition rewrite (length-2 → 1) ----

fn tier3_decompose(a: Operation, b: Operation) -> Option<Operation> {

    match (a, b) {
        (Ss, Sa) | (Sa, Ss) => Some(Sb),
        (Ss, Sb) | (Sb, Ss) => Some(Sa),
        (Rr, Rra) | (Rra, Rr) => Some(Rb),
        (Rr, Rrb) | (Rrb, Rr) => Some(Ra),
        (Rrr, Ra) | (Ra, Rrr) => Some(Rrb),
        (Rrr, Rb) | (Rb, Rrr) => Some(Rra),
        _ => None,
    }
}

// ---- Tier 5: Length-3 algebraic rewrites (length-3 → 2) ----

fn tier5_rewrite(a: Operation, b: Operation, c: Operation) -> Option<(Operation, Operation)> {

    match (a, b, c) {
        (Ra, Pb, Rra) => Some((Sa, Pb)),
        (Rb, Pa, Rrb) => Some((Sb, Pa)),
        (Ra, Pa, Rra) => Some((Pa, Sa)),
        (Rb, Pb, Rrb) => Some((Pb, Sb)),
        _ => None,
    }
}

// ---- Tier 4 helpers ----

/// Reduce a single-stack op subsequence by canceling adjacent inverse pairs.
/// Uses a stack: push each op, pop if top cancels with incoming op.
fn reduce_pure(ops: &[Operation]) -> Vec<Operation> {
    let mut stack: Vec<Operation> = Vec::with_capacity(ops.len());
    for &op in ops {
        if let Some(&top) = stack.last()
            && tier1_cancel(top, op)
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


    let a_sa = a_ops.iter().filter(|&&op| op == Sa).count();
    let a_ra = a_ops.iter().filter(|&&op| op == Ra).count();
    let a_rra = a_ops.iter().filter(|&&op| op == Rra).count();

    let b_sb = b_ops.iter().filter(|&&op| op == Sb).count();
    let b_rb = b_ops.iter().filter(|&&op| op == Rb).count();
    let b_rrb = b_ops.iter().filter(|&&op| op == Rrb).count();

    let merge_ss = a_sa.min(b_sb);
    let merge_rr = a_ra.min(b_rb);
    let merge_rrr = a_rra.min(b_rrb);

    let mut result = Vec::new();
    result.extend(std::iter::repeat_n(Ss, merge_ss));
    result.extend(std::iter::repeat_n(Rr, merge_rr));
    result.extend(std::iter::repeat_n(Rrr, merge_rrr));
    result.extend(std::iter::repeat_n(Sa, a_sa - merge_ss));
    result.extend(std::iter::repeat_n(Ra, a_ra - merge_rr));
    result.extend(std::iter::repeat_n(Rra, a_rra - merge_rrr));
    result.extend(std::iter::repeat_n(Sb, b_sb - merge_ss));
    result.extend(std::iter::repeat_n(Rb, b_rb - merge_rr));
    result.extend(std::iter::repeat_n(Rrb, b_rrb - merge_rrr));

    result
}

// ---- Pass: Tiers 1-3 (adjacent peephole) ----

/// Scan adjacent active op pairs and apply Tier 1/2/3 rewrite rules.
/// Steps back on rewrite to catch cascading reductions.
fn pass_adjacent(logs: &mut [Log]) -> bool {
    let mut changed = false;
    let Some(mut i) = next_exec(logs, 0) else {
        return false;
    };

    while let Some(j) = next_exec(logs, i + 1) {
        let a = op_at(logs, i);
        let b = op_at(logs, j);

        if tier1_cancel(a, b) {
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
        } else if let Some(replacement) = tier2_merge(a, b).or_else(|| tier3_decompose(a, b)) {
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

// ---- Pass: Tier 4 (commutativity-aware sliding window) ----

/// Exploit A/B commutativity: within blocks between barriers,
/// partition into A-only and B-only subsequences, reduce each,
/// then merge matching pairs into combined ops.
fn pass_tier4(logs: &mut [Log]) -> bool {
    let mut changed = false;

    let active: Vec<usize> = (0..logs.len())
        .filter(|&i| matches!(logs[i], Log::Execute(_)))
        .collect();

    if active.is_empty() {
        return false;
    }

    // Split active indices into blocks at barriers
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

        // Write back and detect changes inline
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

// ---- Pass: Tier 5 (length-3 algebraic rewrites) ----

/// Scan windows of three active ops for rotation-around-push identities.
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

        if let Some((new1, new2)) = tier5_rewrite(a, b, c) {
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

// ---- Main entry point ----

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


    /// StackPair with 10 elements, 3 pushed to B — both stacks populated.
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
        let pairs: &[&[Operation]] = &[
            &[Sa, Sa],
            &[Sb, Sb],
            &[Ss, Ss],
            &[Pa, Pb],
            &[Pb, Pa],
            &[Ra, Rra],
            &[Rra, Ra],
            &[Rb, Rrb],
            &[Rrb, Rb],
            &[Rr, Rrr],
            &[Rrr, Rr],
        ];
        for ops in pairs {
            assert_identity(ops);
            assert_optimizes_to(ops, &[]);
        }
    }

    // ================================================================
    // Tier 2: A+B pairs equivalent to combined op, optimizer merges
    // ================================================================

    #[test]
    fn tier2_merges() {
        let rules: &[(&[Operation], Operation)] = &[
            (&[Sa, Sb], Ss),
            (&[Sb, Sa], Ss),
            (&[Ra, Rb], Rr),
            (&[Rb, Ra], Rr),
            (&[Rra, Rrb], Rrr),
            (&[Rrb, Rra], Rrr),
        ];
        for &(input, expected) in rules {
            assert_equivalent(input, &[expected]);
            assert_optimizes_to(input, &[expected]);
        }
    }

    // ================================================================
    // Tier 3: combined + half → other half, optimizer decomposes
    // ================================================================

    #[test]
    fn tier3_decompositions() {
        let rules: &[(&[Operation], Operation)] = &[
            (&[Ss, Sa], Sb),
            (&[Sa, Ss], Sb),
            (&[Ss, Sb], Sa),
            (&[Sb, Ss], Sa),
            (&[Rr, Rra], Rb),
            (&[Rra, Rr], Rb),
            (&[Rr, Rrb], Ra),
            (&[Rrb, Rr], Ra),
            (&[Rrr, Ra], Rrb),
            (&[Ra, Rrr], Rrb),
            (&[Rrr, Rb], Rra),
            (&[Rb, Rrr], Rra),
        ];
        for &(input, expected) in rules {
            assert_equivalent(input, &[expected]);
            assert_optimizes_to(input, &[expected]);
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
        // A=[ra], B=[sb,rb]. Merge ra+rb → rr. Left: sb.
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
        let rules: &[(&[Operation], &[Operation])] = &[
            (&[Ra, Pb, Rra], &[Sa, Pb]),
            (&[Rb, Pa, Rrb], &[Sb, Pa]),
            (&[Ra, Pa, Rra], &[Pa, Sa]),
            (&[Rb, Pb, Rrb], &[Pb, Sb]),
        ];
        for &(input, expected) in rules {
            assert_equivalent(input, expected);
            assert_optimizes_to(input, expected);
        }
    }

    // ================================================================
    // Cascading / fixed-point
    // ================================================================

    #[test]
    fn cascade_merge_then_decompose() {
        // ra rb rra → merge(ra+rb)=rr, then tier3(rr+rra)=rb
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
