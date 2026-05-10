use crate::stacks::{Log, Operation};

// ---- Operation classification ----

fn is_a_only(op: Operation) -> bool {
    matches!(op, Operation::Sa | Operation::Ra | Operation::Rra)
}

fn is_b_only(op: Operation) -> bool {
    matches!(op, Operation::Sb | Operation::Rb | Operation::Rrb)
}

fn is_barrier(op: Operation) -> bool {
    matches!(
        op,
        Operation::Ss | Operation::Rr | Operation::Rrr | Operation::Pa | Operation::Pb
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
    use Operation::*;
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
    use Operation::*;
    match (a, b) {
        (Sa, Sb) | (Sb, Sa) => Some(Ss),
        (Ra, Rb) | (Rb, Ra) => Some(Rr),
        (Rra, Rrb) | (Rrb, Rra) => Some(Rrr),
        _ => None,
    }
}

// ---- Tier 3: Decomposition rewrite (length-2 → 1) ----

fn tier3_decompose(a: Operation, b: Operation) -> Option<Operation> {
    use Operation::*;
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
    use Operation::*;
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
    use Operation::*;

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
        } else if let Some(merged) = tier2_merge(a, b) {
            logs[i] = Log::Execute(merged);
            logs[j] = Log::Ignore(b);
            changed = true;
            if let Some(prev) = prev_exec(logs, i) {
                i = prev;
            }
        } else if let Some(result) = tier3_decompose(a, b) {
            logs[i] = Log::Execute(result);
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

        let orig_ops: Vec<Operation> =
            block_indices.iter().map(|&idx| op_at(logs, idx)).collect();

        if merged.len() != orig_ops.len() || merged != orig_ops {
            changed = true;
            for (k, &new_op) in merged.iter().enumerate() {
                logs[block_indices[k]] = Log::Execute(new_op);
            }
            for k in merged.len()..block_indices.len() {
                let old_op = op_at(logs, block_indices[k]);
                logs[block_indices[k]] = Log::Ignore(old_op);
            }
        }
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
pub fn optimize(logs: Vec<Log>) -> Vec<Log> {
    let mut logs = logs;
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
    use Operation::*;

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

    /// Assert that applying `ops` leaves stacks unchanged (identity).
    fn assert_identity(ops: &[Operation]) {
        let s = make_stacks();
        let before = snapshot(&s);
        let after = run_ops(&s, ops);
        assert_eq!(before, after, "ops {ops:?} should be identity");
    }

    /// Assert that two op sequences produce identical stack states.
    fn assert_equivalent(ops_a: &[Operation], ops_b: &[Operation]) {
        let s = make_stacks();
        let result_a = run_ops(&s, ops_a);
        let result_b = run_ops(&s, ops_b);
        assert_eq!(
            result_a, result_b,
            "ops {ops_a:?} and {ops_b:?} should be equivalent"
        );
    }

    fn exec_count(logs: &[Log]) -> usize {
        logs.iter()
            .filter(|l| matches!(l, Log::Execute(_)))
            .count()
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

    // ================================================================
    // Tier 1: each canceling pair is identity on real stacks
    // ================================================================

    #[test]
    fn tier1_sa_sa_identity() {
        assert_identity(&[Sa, Sa]);
    }
    #[test]
    fn tier1_sb_sb_identity() {
        assert_identity(&[Sb, Sb]);
    }
    #[test]
    fn tier1_ss_ss_identity() {
        assert_identity(&[Ss, Ss]);
    }
    #[test]
    fn tier1_pa_pb_identity() {
        assert_identity(&[Pa, Pb]);
    }
    #[test]
    fn tier1_pb_pa_identity() {
        assert_identity(&[Pb, Pa]);
    }
    #[test]
    fn tier1_ra_rra_identity() {
        assert_identity(&[Ra, Rra]);
    }
    #[test]
    fn tier1_rra_ra_identity() {
        assert_identity(&[Rra, Ra]);
    }
    #[test]
    fn tier1_rb_rrb_identity() {
        assert_identity(&[Rb, Rrb]);
    }
    #[test]
    fn tier1_rrb_rb_identity() {
        assert_identity(&[Rrb, Rb]);
    }
    #[test]
    fn tier1_rr_rrr_identity() {
        assert_identity(&[Rr, Rrr]);
    }
    #[test]
    fn tier1_rrr_rr_identity() {
        assert_identity(&[Rrr, Rr]);
    }

    // Tier 1 optimizer: canceled pairs produce 0 execute ops
    #[test]
    fn tier1_optimizer_sa_sa() {
        assert_eq!(exec_count(&optimize(make_logs(&[Sa, Sa]))), 0);
    }
    #[test]
    fn tier1_optimizer_ra_rra() {
        assert_eq!(exec_count(&optimize(make_logs(&[Ra, Rra]))), 0);
    }
    #[test]
    fn tier1_optimizer_pa_pb() {
        assert_eq!(exec_count(&optimize(make_logs(&[Pa, Pb]))), 0);
    }
    #[test]
    fn tier1_optimizer_rr_rrr() {
        assert_eq!(exec_count(&optimize(make_logs(&[Rr, Rrr]))), 0);
    }

    // ================================================================
    // Tier 2: each merging pair is equivalent to the combined op
    // ================================================================

    #[test]
    fn tier2_sa_sb_equiv_ss() {
        assert_equivalent(&[Sa, Sb], &[Ss]);
    }
    #[test]
    fn tier2_sb_sa_equiv_ss() {
        assert_equivalent(&[Sb, Sa], &[Ss]);
    }
    #[test]
    fn tier2_ra_rb_equiv_rr() {
        assert_equivalent(&[Ra, Rb], &[Rr]);
    }
    #[test]
    fn tier2_rb_ra_equiv_rr() {
        assert_equivalent(&[Rb, Ra], &[Rr]);
    }
    #[test]
    fn tier2_rra_rrb_equiv_rrr() {
        assert_equivalent(&[Rra, Rrb], &[Rrr]);
    }
    #[test]
    fn tier2_rrb_rra_equiv_rrr() {
        assert_equivalent(&[Rrb, Rra], &[Rrr]);
    }

    // Tier 2 optimizer: pairs merge to single combined op
    #[test]
    fn tier2_optimizer_sa_sb() {
        assert_eq!(exec_ops(&optimize(make_logs(&[Sa, Sb]))), vec![Ss]);
    }
    #[test]
    fn tier2_optimizer_ra_rb() {
        assert_eq!(exec_ops(&optimize(make_logs(&[Ra, Rb]))), vec![Rr]);
    }
    #[test]
    fn tier2_optimizer_rra_rrb() {
        assert_eq!(exec_ops(&optimize(make_logs(&[Rra, Rrb]))), vec![Rrr]);
    }

    // ================================================================
    // Tier 3: each decomposition is equivalent to its replacement
    // ================================================================

    #[test]
    fn tier3_ss_sa_equiv_sb() {
        assert_equivalent(&[Ss, Sa], &[Sb]);
    }
    #[test]
    fn tier3_sa_ss_equiv_sb() {
        assert_equivalent(&[Sa, Ss], &[Sb]);
    }
    #[test]
    fn tier3_ss_sb_equiv_sa() {
        assert_equivalent(&[Ss, Sb], &[Sa]);
    }
    #[test]
    fn tier3_sb_ss_equiv_sa() {
        assert_equivalent(&[Sb, Ss], &[Sa]);
    }
    #[test]
    fn tier3_rr_rra_equiv_rb() {
        assert_equivalent(&[Rr, Rra], &[Rb]);
    }
    #[test]
    fn tier3_rra_rr_equiv_rb() {
        assert_equivalent(&[Rra, Rr], &[Rb]);
    }
    #[test]
    fn tier3_rr_rrb_equiv_ra() {
        assert_equivalent(&[Rr, Rrb], &[Ra]);
    }
    #[test]
    fn tier3_rrb_rr_equiv_ra() {
        assert_equivalent(&[Rrb, Rr], &[Ra]);
    }
    #[test]
    fn tier3_rrr_ra_equiv_rrb() {
        assert_equivalent(&[Rrr, Ra], &[Rrb]);
    }
    #[test]
    fn tier3_ra_rrr_equiv_rrb() {
        assert_equivalent(&[Ra, Rrr], &[Rrb]);
    }
    #[test]
    fn tier3_rrr_rb_equiv_rra() {
        assert_equivalent(&[Rrr, Rb], &[Rra]);
    }
    #[test]
    fn tier3_rb_rrr_equiv_rra() {
        assert_equivalent(&[Rb, Rrr], &[Rra]);
    }

    // Tier 3 optimizer
    #[test]
    fn tier3_optimizer_ss_sa() {
        assert_eq!(exec_ops(&optimize(make_logs(&[Ss, Sa]))), vec![Sb]);
    }
    #[test]
    fn tier3_optimizer_rr_rra() {
        assert_eq!(exec_ops(&optimize(make_logs(&[Rr, Rra]))), vec![Rb]);
    }
    #[test]
    fn tier3_optimizer_rrr_ra() {
        assert_eq!(exec_ops(&optimize(make_logs(&[Rrr, Ra]))), vec![Rrb]);
    }

    // ================================================================
    // Tier 4: non-adjacent cancellation via commutativity
    // ================================================================

    #[test]
    fn tier4_ra_sb_rra_equiv_sb() {
        assert_equivalent(&[Ra, Sb, Rra], &[Sb]);
    }

    #[test]
    fn tier4_optimizer_cancel_across_commuting() {
        let result = optimize(make_logs(&[Ra, Sb, Rra]));
        assert_eq!(exec_ops(&result), vec![Sb]);
    }

    #[test]
    fn tier4_optimizer_merge_across_commuting() {
        // ra sb rb → block [ra, sb, rb]. A=[ra], B=[sb,rb].
        // No cancel. Merge ra+rb → rr. Left: sb. Result: [rr, sb].
        let result = optimize(make_logs(&[Ra, Sb, Rb]));
        assert_eq!(exec_count(&result), 2);
    }

    #[test]
    fn tier4_worked_example() {
        // From spec: ra sb rra rb pb → sb rb pb (5 → 3)
        let result = optimize(make_logs(&[Ra, Sb, Rra, Rb, Pb]));
        assert_eq!(exec_count(&result), 3);
    }

    #[test]
    fn tier4_preserves_semantics() {
        let ops = [Ra, Sb, Rra, Rb, Pb];
        let base = make_stacks();
        let original = run_ops(&base, &ops);
        let opt_ops = exec_ops(&optimize(make_logs(&ops)));
        let optimized = run_ops(&base, &opt_ops);
        assert_eq!(original, optimized);
    }

    // ================================================================
    // Tier 5: rotation-around-push identities
    // ================================================================

    #[test]
    fn tier5_ra_pb_rra_equiv_sa_pb() {
        assert_equivalent(&[Ra, Pb, Rra], &[Sa, Pb]);
    }
    #[test]
    fn tier5_rb_pa_rrb_equiv_sb_pa() {
        assert_equivalent(&[Rb, Pa, Rrb], &[Sb, Pa]);
    }
    #[test]
    fn tier5_ra_pa_rra_equiv_pa_sa() {
        assert_equivalent(&[Ra, Pa, Rra], &[Pa, Sa]);
    }
    #[test]
    fn tier5_rb_pb_rrb_equiv_pb_sb() {
        assert_equivalent(&[Rb, Pb, Rrb], &[Pb, Sb]);
    }

    #[test]
    fn tier5_optimizer_ra_pb_rra() {
        let result = optimize(make_logs(&[Ra, Pb, Rra]));
        assert_eq!(exec_count(&result), 2);
    }

    #[test]
    fn tier5_preserves_semantics() {
        let ops = [Ra, Pb, Rra];
        let base = make_stacks();
        let original = run_ops(&base, &ops);
        let opt_ops = exec_ops(&optimize(make_logs(&ops)));
        let optimized = run_ops(&base, &opt_ops);
        assert_eq!(original, optimized);
    }

    // ================================================================
    // Cascading / fixed-point
    // ================================================================

    #[test]
    fn cascade_merge_then_decompose() {
        // ra rb rra → merge(ra+rb)=rr, then tier3(rr+rra)=rb
        let result = optimize(make_logs(&[Ra, Rb, Rra]));
        assert_eq!(exec_ops(&result), vec![Rb]);
    }

    #[test]
    fn cascade_merge_then_decompose_equiv() {
        assert_equivalent(&[Ra, Rb, Rra], &[Rb]);
    }

    #[test]
    fn cascade_multiple_cancellations() {
        let result = optimize(make_logs(&[Sa, Sa, Ra, Rra]));
        assert_eq!(exec_count(&result), 0);
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
        assert_eq!(exec_ops(&optimize(make_logs(&[Ra]))), vec![Ra]);
    }

    #[test]
    fn no_optimization_possible() {
        let result = optimize(make_logs(&[Ra, Pa, Sb]));
        assert_eq!(exec_count(&result), 3);
    }

    #[test]
    fn ignores_preserved() {
        let logs = vec![Log::Ignore(Sa), Log::Execute(Ra), Log::Execute(Rra)];
        let result = optimize(logs);
        // Ra+Rra cancel, Ignore(Sa) preserved
        assert_eq!(exec_count(&result), 0);
        assert!(matches!(result[0], Log::Ignore(_)));
    }

    #[test]
    fn complex_sequence_preserves_semantics() {
        let ops = [Ra, Sb, Rra, Rb, Sa, Sa, Pb, Rr, Rrr, Ra, Pb, Rra];
        let base = make_stacks();
        let original = run_ops(&base, &ops);
        let opt_ops = exec_ops(&optimize(make_logs(&ops)));
        let optimized = run_ops(&base, &opt_ops);
        assert_eq!(original, optimized);
        assert!(opt_ops.len() <= ops.len());
    }
}
