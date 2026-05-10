use push_swap::stacks::{Operation, StackPair};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::str::FromStr;

// ── State Engine ──────────────────────────────────────────────────────

/// Snapshot of both stacks, used as HashMap key for the oracle.
type State = (Vec<usize>, Vec<usize>);

/// Stack size for the canonical state. At least 3 so all 11 ops produce
/// distinct states (with only 2 elements, swap == rotate).
fn stack_size(n: usize) -> usize {
    n.max(3)
}

/// Build a canonical state with `sz` elements in each stack, all distinct.
/// Created by putting `[1..=2*sz]` into A then executing `pb` sz times.
fn canonical_state(n: usize) -> StackPair {
    let sz = stack_size(n);
    let values: Vec<usize> = (1..=2 * sz).collect();
    let mut sp = StackPair::new(values);
    for _ in 0..sz {
        sp.execute(Operation::Pb);
    }
    sp.set_logs(vec![]);
    sp
}

fn get_state(sp: &StackPair) -> State {
    (
        sp.a().iter().copied().collect(),
        sp.b().iter().copied().collect(),
    )
}

// ── Reducible Pattern Set ─────────────────────────────────────────────

/// Tracks operation sequences known to have a shorter equivalent.
/// Organized by pattern length for efficient suffix lookups during
/// recursive enumeration.
struct ReducibleSet {
    by_length: HashMap<usize, HashSet<Vec<Operation>>>,
}

impl ReducibleSet {
    fn new() -> Self {
        Self {
            by_length: HashMap::new(),
        }
    }

    fn add(&mut self, pattern: &[Operation]) {
        self.by_length
            .entry(pattern.len())
            .or_default()
            .insert(pattern.to_vec());
    }

    /// Check if any suffix of `seq` matches a known reducible pattern.
    /// Only suffixes needed: non-suffix windows were checked when those ops were added.
    fn has_reducible_suffix(&self, seq: &[Operation]) -> bool {
        for (&pat_len, patterns) in &self.by_length {
            if pat_len <= seq.len() {
                let suffix = &seq[seq.len() - pat_len..];
                if patterns.contains(suffix) {
                    return true;
                }
            }
        }
        false
    }
}

// ── Rules ─────────────────────────────────────────────────────────────

struct Rules {
    reductions: Vec<(Vec<Operation>, Vec<Operation>)>,
    annihilators: Vec<Vec<Operation>>,
    equivalences: Vec<(Vec<Operation>, Vec<Operation>)>,
}

impl Rules {
    fn new() -> Self {
        Self {
            reductions: Vec::new(),
            annihilators: Vec::new(),
            equivalences: Vec::new(),
        }
    }
}

// ── Search ────────────────────────────────────────────────────────────

/// Run one depth level: enumerate all sequences of `depth` operations,
/// compare each result against the oracle, and collect new rules.
/// New reducible patterns are buffered and added after the full depth
/// completes (rules from depth K prune depth K+1, not K itself).
fn search_depth(
    depth: usize,
    canonical: &StackPair,
    oracle: &mut HashMap<State, Vec<Operation>>,
    rules: &mut Rules,
    reducible: &mut ReducibleSet,
) {
    let mut current_ops: Vec<Operation> = Vec::with_capacity(depth);
    let mut new_reducible: Vec<Vec<Operation>> = Vec::new();

    enumerate(
        canonical,
        depth,
        &mut current_ops,
        oracle,
        rules,
        reducible,
        &mut new_reducible,
    );

    for pattern in new_reducible {
        reducible.add(&pattern);
    }
}

/// Recursive sequence builder. At each level, clones the parent StackPair
/// and applies one more operation — avoids replaying from scratch.
/// Pruning happens before cloning: if the current suffix matches a known
/// reducible pattern, the branch is skipped entirely.
fn enumerate(
    sp: &StackPair,
    target_depth: usize,
    current_ops: &mut Vec<Operation>,
    oracle: &mut HashMap<State, Vec<Operation>>,
    rules: &mut Rules,
    reducible: &ReducibleSet,
    new_reducible: &mut Vec<Vec<Operation>>,
) {
    if current_ops.len() == target_depth {
        let state = get_state(sp);

        if let Some(existing) = oracle.get(&state) {
            if existing.len() < current_ops.len() {
                // Strict reduction found
                if existing.is_empty() {
                    rules.annihilators.push(current_ops.clone());
                } else {
                    rules
                        .reductions
                        .push((current_ops.clone(), existing.clone()));
                }
                new_reducible.push(current_ops.clone());
            } else if existing.len() == current_ops.len() && *existing != *current_ops {
                // Same-length equivalence. Existing is canonical (enumerated first).
                rules
                    .equivalences
                    .push((existing.clone(), current_ops.clone()));
            }
        } else {
            oracle.insert(state, current_ops.clone());
        }
        return;
    }

    for &op in &Operation::ALL {
        current_ops.push(op);

        if !reducible.has_reducible_suffix(current_ops) {
            let mut sp_clone = sp.clone();
            sp_clone.execute(op);
            sp_clone.set_logs(vec![]);

            enumerate(
                &sp_clone,
                target_depth,
                current_ops,
                oracle,
                rules,
                reducible,
                new_reducible,
            );
        }

        current_ops.pop();
    }
}

// ── Fuzz Verifier ─────────────────────────────────────────────────────

/// Build a StackPair with `a_size` elements in A and `b_size` in B.
fn make_config(a_size: usize, b_size: usize) -> StackPair {
    let total = a_size + b_size;
    if total == 0 {
        return StackPair::new(vec![]);
    }
    let mut sp = StackPair::new((1..=total).collect());
    for _ in 0..b_size {
        sp.execute(Operation::Pb);
    }
    sp.set_logs(vec![]);
    sp
}

/// Test that `lhs` and `rhs` produce identical stacks across 1,000 random
/// configurations. Both stacks start with enough elements that no operation
/// in either sequence is a no-op (min 2 per stack for swaps/rotates, plus
/// enough for the longest run of pushes in one direction).
fn verify_rule(lhs: &[Operation], rhs: &[Operation], n: usize) -> bool {
    let mut rng = rand::rng();
    let max_total = 2 * n + 10;
    let min_per_stack = lhs.len().max(rhs.len()).max(2);

    for _ in 0..1000 {
        let total = rng.random_range(2 * min_per_stack..=max_total);
        let a_size = rng.random_range(min_per_stack..=total - min_per_stack);
        let b_size = total - a_size;

        let base = make_config(a_size, b_size);

        let mut sp_lhs = base.clone();
        for &op in lhs {
            sp_lhs.execute(op);
        }

        let mut sp_rhs = base;
        for &op in rhs {
            sp_rhs.execute(op);
        }

        if sp_lhs.a() != sp_rhs.a() || sp_lhs.b() != sp_rhs.b() {
            return false;
        }
    }
    true
}

// ── Persistence ───────────────────────────────────────────────────────
//
// Operations are serialized as strings via Display/FromStr since we can't
// derive Serialize on a foreign type. The cache stores the full oracle
// (state → shortest sequence) plus all discovered rules.

const CACHE_FILE: &str = "superopt_cache.json";

#[derive(Serialize, Deserialize)]
struct OracleEntry {
    state_a: Vec<usize>,
    state_b: Vec<usize>,
    ops: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct CacheData {
    canonical_n: usize,
    max_depth_explored: usize,
    oracle: Vec<OracleEntry>,
    reductions: Vec<(Vec<String>, Vec<String>)>,
    annihilators: Vec<Vec<String>>,
    equivalences: Vec<(Vec<String>, Vec<String>)>,
}

fn ops_to_strings(ops: &[Operation]) -> Vec<String> {
    ops.iter().map(|op| op.to_string()).collect()
}

fn strings_to_ops(strings: &[String]) -> Vec<Operation> {
    strings
        .iter()
        .map(|s| Operation::from_str(s).unwrap())
        .collect()
}

fn save_cache(
    n: usize,
    max_depth: usize,
    oracle: &HashMap<State, Vec<Operation>>,
    rules: &Rules,
) {
    let oracle_entries: Vec<OracleEntry> = oracle
        .iter()
        .map(|((sa, sb), ops)| OracleEntry {
            state_a: sa.clone(),
            state_b: sb.clone(),
            ops: ops_to_strings(ops),
        })
        .collect();

    let data = CacheData {
        canonical_n: n,
        max_depth_explored: max_depth,
        oracle: oracle_entries,
        reductions: rules
            .reductions
            .iter()
            .map(|(from, to)| (ops_to_strings(from), ops_to_strings(to)))
            .collect(),
        annihilators: rules
            .annihilators
            .iter()
            .map(|seq| ops_to_strings(seq))
            .collect(),
        equivalences: rules
            .equivalences
            .iter()
            .map(|(canon, equiv)| (ops_to_strings(canon), ops_to_strings(equiv)))
            .collect(),
    };

    let json = serde_json::to_string(&data).expect("failed to serialize cache");
    fs::write(CACHE_FILE, json).expect("failed to write cache file");
}

fn load_cache() -> Option<CacheData> {
    let json = fs::read_to_string(CACHE_FILE).ok()?;
    serde_json::from_str(&json).ok()
}

/// Reconstruct oracle, rules, and reducible set from cached data so
/// search can resume from `max_depth_explored + 1`.
fn rebuild_from_cache(
    cache: &CacheData,
) -> (HashMap<State, Vec<Operation>>, Rules, ReducibleSet) {
    let mut oracle = HashMap::new();
    for entry in &cache.oracle {
        let state = (entry.state_a.clone(), entry.state_b.clone());
        let ops = strings_to_ops(&entry.ops);
        oracle.insert(state, ops);
    }

    let mut rules = Rules::new();
    for (from, to) in &cache.reductions {
        rules
            .reductions
            .push((strings_to_ops(from), strings_to_ops(to)));
    }
    for seq in &cache.annihilators {
        rules.annihilators.push(strings_to_ops(seq));
    }
    for (canon, equiv) in &cache.equivalences {
        rules
            .equivalences
            .push((strings_to_ops(canon), strings_to_ops(equiv)));
    }

    // Rebuild reducible set from known reductions + annihilators
    let mut reducible = ReducibleSet::new();
    for (from, _) in &rules.reductions {
        reducible.add(from);
    }
    for seq in &rules.annihilators {
        reducible.add(seq);
    }

    (oracle, rules, reducible)
}

// ── Output Formatter ──────────────────────────────────────────────────

fn fmt_ops(ops: &[Operation]) -> String {
    ops.iter()
        .map(|op| op.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn print_rules(rules: &Rules) {
    println!("## Strict Reductions (N → shorter)\n");
    println!("| From | To |");
    println!("|------|----|");
    for (from, to) in &rules.reductions {
        println!("| {} | {} |", fmt_ops(from), fmt_ops(to));
    }

    println!("\n## Annihilators (N → empty)\n");
    println!("| Sequence |");
    println!("|----------|");
    for seq in &rules.annihilators {
        println!("| {} |", fmt_ops(seq));
    }

    println!("\n## Equivalences (same length)\n");
    println!("| Canonical | Equivalent |");
    println!("|-----------|------------|");
    for (canon, equiv) in &rules.equivalences {
        println!("| {} | {} |", fmt_ops(canon), fmt_ops(equiv));
    }
}

// ── Main ──────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: superopt <max_depth>");
        std::process::exit(1);
    }
    let n: usize = args[1].parse().unwrap_or_else(|_| {
        eprintln!("Error: max_depth must be a positive integer");
        std::process::exit(1);
    });
    if n < 2 {
        eprintln!("Error: max_depth must be >= 2");
        std::process::exit(1);
    }

    // Load cache
    let sz = stack_size(n);

    let (mut oracle, mut rules, mut reducible, start_depth) = match load_cache() {
        Some(cache) if cache.canonical_n == sz => {
            if cache.max_depth_explored >= n {
                // Already fully explored — just print
                let (_, rules, _) = rebuild_from_cache(&cache);
                print_rules(&rules);
                return;
            }
            let (oracle, rules, reducible) = rebuild_from_cache(&cache);
            let start = cache.max_depth_explored + 1;
            eprintln!(
                "Resuming from cached depth {} (depths 1..={})",
                start, cache.max_depth_explored
            );
            (oracle, rules, reducible, start)
        }
        _ => {
            // Fresh start
            let canonical = canonical_state(n);
            let initial_state = get_state(&canonical);
            let mut oracle = HashMap::new();
            oracle.insert(initial_state, vec![]);
            (oracle, Rules::new(), ReducibleSet::new(), 1)
        }
    };

    let canonical = canonical_state(n);

    for depth in start_depth..=n {
        let reds_before = rules.reductions.len() + rules.annihilators.len();
        let equivs_before = rules.equivalences.len();
        eprintln!(
            "Depth {depth}: searching (oracle size: {})...",
            oracle.len()
        );

        search_depth(depth, &canonical, &mut oracle, &mut rules, &mut reducible);

        let new_reds = (rules.reductions.len() + rules.annihilators.len()) - reds_before;
        let new_equivs = rules.equivalences.len() - equivs_before;
        eprintln!(
            "Depth {depth}: done. +{new_reds} reductions, +{new_equivs} equivalences, oracle size: {}",
            oracle.len()
        );

        save_cache(sz, depth, &oracle, &rules);
    }

    // Fuzz-verify all rules, drop failures
    eprintln!("Verifying {} reductions...", rules.reductions.len());
    rules
        .reductions
        .retain(|(from, to)| verify_rule(from, to, n));

    eprintln!("Verifying {} annihilators...", rules.annihilators.len());
    rules
        .annihilators
        .retain(|seq| verify_rule(seq, &[], n));

    eprintln!("Verifying {} equivalences...", rules.equivalences.len());
    rules
        .equivalences
        .retain(|(canon, equiv)| verify_rule(canon, equiv, n));

    // Save verified rules
    save_cache(sz, n, &oracle, &rules);

    print_rules(&rules);
}
