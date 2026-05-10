use crate::stacks::Operation;
use Operation::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::LazyLock;

// Rule loading — parse embedded JSON once on first use

#[derive(Deserialize)]
struct CacheData {
    reductions: Vec<(Vec<String>, Vec<String>)>,
    annihilators: Vec<Vec<String>>,
}

struct RuleSet {
    by_len: HashMap<usize, HashMap<Vec<Operation>, Vec<Operation>>>,
    max_len: usize,
}

fn parse_ops(strings: &[String]) -> Vec<Operation> {
    strings
        .iter()
        .map(|s| Operation::from_str(s).unwrap())
        .collect()
}

static RULESET: LazyLock<RuleSet> = LazyLock::new(|| {
    let json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/superopt_cache.json"));
    let cache: CacheData = serde_json::from_str(json).expect("invalid superopt_cache.json");

    let mut max_len = 0;
    let mut by_len: HashMap<usize, HashMap<Vec<Operation>, Vec<Operation>>> = HashMap::new();

    for seq in &cache.annihilators {
        let from = parse_ops(seq);
        max_len = max_len.max(from.len());
        by_len.entry(from.len()).or_default().insert(from, vec![]);
    }
    for (from_s, to_s) in &cache.reductions {
        let from = parse_ops(from_s);
        let to = parse_ops(to_s);
        max_len = max_len.max(from.len());
        by_len.entry(from.len()).or_default().insert(from, to);
    }

    RuleSet { by_len, max_len }
});

// Operation classification (for normalization pass)

fn is_a_only(op: Operation) -> bool {
    matches!(op, Sa | Ra | Rra)
}

fn is_b_only(op: Operation) -> bool {
    matches!(op, Sb | Rb | Rrb)
}

fn is_barrier(op: Operation) -> bool {
    matches!(op, Ss | Rr | Rrr | Pa | Pb)
}

// Passes

/// Normalization: within blocks between barriers, reorder so all A-ops
/// come before B-ops. This exposes adjacent same-stack ops for the
/// peephole pass to cancel or merge.
fn pass_normalize(ops: &mut [Operation]) -> bool {
    let mut changed = false;
    let mut i = 0;

    while i < ops.len() {
        let block_start = i;
        while i < ops.len() && !is_barrier(ops[i]) {
            i += 1;
        }
        let block_end = i;

        if block_end - block_start >= 2 {
            let mut a_ops = Vec::new();
            let mut b_ops = Vec::new();

            for &op in &ops[block_start..block_end] {
                if is_a_only(op) {
                    a_ops.push(op);
                } else if is_b_only(op) {
                    b_ops.push(op);
                }
            }

            let new_len = a_ops.len() + b_ops.len();
            let old_block = &ops[block_start..block_end];

            if new_len == old_block.len() {
                // Block is pure A/B ops — check if reorder changes anything
                let mut reordered = Vec::with_capacity(new_len);
                reordered.extend(&a_ops);
                reordered.extend(&b_ops);

                if reordered != old_block {
                    ops[block_start..block_end].copy_from_slice(&reordered);
                    changed = true;
                }
            }
        }

        // Skip barrier
        if i < ops.len() {
            i += 1;
        }
    }

    changed
}

/// Peephole: scan with variable-width windows, apply reduction rules.
/// Tries longest windows first (greedy). Steps back on match for cascading.
fn pass_peephole(ops: &mut Vec<Operation>) -> bool {
    let ruleset = &*RULESET;
    let mut changed = false;
    let mut i = 0;

    while i < ops.len() {
        let mut matched = false;

        // Try longest windows first
        let max_w = ruleset.max_len.min(ops.len() - i);
        for window_len in (2..=max_w).rev() {
            let window = &ops[i..i + window_len];
            if let Some(rules_for_len) = ruleset.by_len.get(&window_len)
                && let Some(replacement) = rules_for_len.get(window)
            {
                let replacement = replacement.clone();
                ops.splice(i..i + window_len, replacement);
                // Step back to catch cascading reductions
                i = i.saturating_sub(ruleset.max_len);
                changed = true;
                matched = true;
                break;
            }
        }

        if !matched {
            i += 1;
        }
    }

    changed
}

// ====================================================================
// Main entry point
// ====================================================================

/// Optimize an operation sequence using generated peephole rules and
/// commutativity-based normalization. Runs in a fixed-point loop.
pub fn optimize(mut ops: Vec<Operation>) -> Vec<Operation> {
    loop {
        let mut changed = false;
        changed |= pass_normalize(&mut ops);
        changed |= pass_peephole(&mut ops);
        if !changed {
            break;
        }
    }

    ops
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

    fn assert_optimizes_to(input: &[Operation], expected: &[Operation]) {
        assert_eq!(
            optimize(input.to_vec()),
            expected,
            "optimize({input:?})"
        );
    }

    // Cancellations (annihilators)

    #[test]
    fn cancellations() {
        let pairs = [
            (Sa, Sa),
            (Sb, Sb),
            (Ss, Ss),
            (Pa, Pb),
            (Pb, Pa),
            (Ra, Rra),
            (Rra, Ra),
            (Rb, Rrb),
            (Rrb, Rb),
            (Rr, Rrr),
            (Rrr, Rr),
        ];
        for (a, b) in pairs {
            assert_identity(&[a, b]);
            assert_optimizes_to(&[a, b], &[]);
        }
    }

    // Pair rewrites (2 → 1)

    #[test]
    fn pair_rewrites() {
        let rules = [
            (Sa, Sb, Ss),
            (Sb, Sa, Ss),
            (Ra, Rb, Rr),
            (Rb, Ra, Rr),
            (Rra, Rrb, Rrr),
            (Rrb, Rra, Rrr),
            (Ss, Sa, Sb),
            (Sa, Ss, Sb),
            (Ss, Sb, Sa),
            (Sb, Ss, Sa),
            (Rr, Rra, Rb),
            (Rra, Rr, Rb),
            (Rr, Rrb, Ra),
            (Rrb, Rr, Ra),
            (Rrr, Ra, Rrb),
            (Ra, Rrr, Rrb),
            (Rrr, Rb, Rra),
            (Rb, Rrr, Rra),
        ];
        for (a, b, result) in rules {
            assert_equivalent(&[a, b], &[result]);
            assert_optimizes_to(&[a, b], &[result]);
        }
    }

    // Commutativity-based optimization (normalization + peephole)

    #[test]
    fn cancel_across_commuting() {
        assert_equivalent(&[Ra, Sb, Rra], &[Sb]);
        assert_optimizes_to(&[Ra, Sb, Rra], &[Sb]);
    }

    #[test]
    fn merge_across_commuting() {
        let result = optimize(vec![Ra, Sb, Rb]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn tier4_worked_example() {
        let ops = [Ra, Sb, Rra, Rb, Pb];
        let base = make_stacks();
        let opt = optimize(ops.to_vec());
        assert_eq!(opt.len(), 3);
        assert_eq!(run_ops(&base, &ops), run_ops(&base, &opt));
    }

    // Triple rewrites (3 → 2)

    #[test]
    fn triple_rewrites() {
        let rules = [
            ((Ra, Pb, Rra), (Sa, Pb)),
            ((Rb, Pa, Rrb), (Sb, Pa)),
            ((Ra, Pa, Rra), (Pa, Sa)),
            ((Rb, Pb, Rrb), (Pb, Sb)),
        ];
        for ((a, b, c), (r1, r2)) in rules {
            assert_equivalent(&[a, b, c], &[r1, r2]);
            assert_optimizes_to(&[a, b, c], &[r1, r2]);
        }
    }

    // Cascading / fixed-point

    #[test]
    fn cascade_merge_then_decompose() {
        assert_equivalent(&[Ra, Rb, Rra], &[Rb]);
        assert_optimizes_to(&[Ra, Rb, Rra], &[Rb]);
    }

    #[test]
    fn cascade_multiple_cancellations() {
        assert_optimizes_to(&[Sa, Sa, Ra, Rra], &[]);
    }

    // Edge cases

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
    fn complex_sequence_preserves_semantics() {
        let ops = [Ra, Sb, Rra, Rb, Sa, Sa, Pb, Rr, Rrr, Ra, Pb, Rra];
        let base = make_stacks();
        let opt = optimize(ops.to_vec());
        assert_eq!(run_ops(&base, &ops), run_ops(&base, &opt));
        assert!(opt.len() <= ops.len());
    }
}
