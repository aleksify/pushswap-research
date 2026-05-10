use push_swap::stacks::{Operation, StackPair};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::str::FromStr;

/// Maximum supported search depth. Packed representations are sized for this.
const N: usize = 10;

// ── Packed Types ─────────────────────────────────────────────────────

/// A stack packed into split registers: lo (u128) + hi (u64) + len (u8).
///
/// 6-bit elements (values 0..63, sufficient for 2*stack_size=42 at N=10).
/// - `lo`: elements 0..20 (21 slots × 6 bits = 126 bits ≤ 128)
/// - `hi`: elements 21..30 (10 slots × 6 bits = 60 bits ≤ 64)
/// - Max 31 elements (stack_size 21 + depth 10).
///
/// elem_0 = top of stack.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PackedStack {
    lo: u128,
    hi: u64,
    len: u8,
}

impl PackedStack {
    const ELEM_BITS: u32 = 6;
    const ELEM_MASK_128: u128 = 0x3F;
    const ELEM_MASK_64: u64 = 0x3F;
    const LO_SLOTS: u32 = 21;
    const LO_MASK: u128 = (1u128 << (21 * 6)) - 1;

    fn len(self) -> u32 {
        self.len as u32
    }

    fn elem(self, i: u32) -> u64 {
        if i < Self::LO_SLOTS {
            ((self.lo >> (i * Self::ELEM_BITS)) & Self::ELEM_MASK_128) as u64
        } else {
            let j = i - Self::LO_SLOTS;
            (self.hi >> (j * Self::ELEM_BITS)) & Self::ELEM_MASK_64
        }
    }

    fn set_elem(self, i: u32, val: u64) -> Self {
        if i < Self::LO_SLOTS {
            let shift = i * Self::ELEM_BITS;
            Self {
                lo: (self.lo & !(Self::ELEM_MASK_128 << shift))
                    | ((val as u128 & Self::ELEM_MASK_128) << shift),
                ..self
            }
        } else {
            let j = i - Self::LO_SLOTS;
            let shift = j * Self::ELEM_BITS;
            Self {
                hi: (self.hi & !(Self::ELEM_MASK_64 << shift))
                    | ((val & Self::ELEM_MASK_64) << shift),
                ..self
            }
        }
    }

    fn from_slice(elems: &[usize]) -> Self {
        let mut s = Self {
            lo: 0,
            hi: 0,
            len: elems.len() as u8,
        };
        for (i, &val) in elems.iter().enumerate() {
            s = s.set_elem(i as u32, val as u64);
        }
        s
    }

    fn to_vec(self) -> Vec<usize> {
        (0..self.len()).map(|i| self.elem(i) as usize).collect()
    }

    fn swap_top(self) -> Self {
        if self.len() < 2 {
            return self;
        }
        let e0 = self.elem(0);
        let e1 = self.elem(1);
        let mask = Self::ELEM_MASK_128 | (Self::ELEM_MASK_128 << Self::ELEM_BITS);
        Self {
            lo: (self.lo & !mask) | (e1 as u128) | ((e0 as u128) << Self::ELEM_BITS),
            ..self
        }
    }

    fn pop_top(self) -> (Self, u64) {
        debug_assert!(self.len() > 0);
        let val = self.elem(0);
        let carry = self.hi & Self::ELEM_MASK_64;
        let new_lo = ((self.lo >> Self::ELEM_BITS)
            | ((carry as u128) << ((Self::LO_SLOTS - 1) * Self::ELEM_BITS)))
            & Self::LO_MASK;
        let new_hi = self.hi >> Self::ELEM_BITS;
        (
            Self {
                lo: new_lo,
                hi: new_hi,
                len: self.len - 1,
            },
            val,
        )
    }

    fn push_top(self, val: u64) -> Self {
        let overflow =
            ((self.lo >> ((Self::LO_SLOTS - 1) * Self::ELEM_BITS)) & Self::ELEM_MASK_128) as u64;
        let new_lo = ((self.lo << Self::ELEM_BITS) & Self::LO_MASK)
            | ((val as u128) & Self::ELEM_MASK_128);
        let new_hi = (self.hi << Self::ELEM_BITS) | (overflow & Self::ELEM_MASK_64);
        Self {
            lo: new_lo,
            hi: new_hi,
            len: self.len + 1,
        }
    }

    fn rotate(self) -> Self {
        let l = self.len();
        if l <= 1 {
            return self;
        }
        let top = self.elem(0);
        let carry = self.hi & Self::ELEM_MASK_64;
        let new_lo = ((self.lo >> Self::ELEM_BITS)
            | ((carry as u128) << ((Self::LO_SLOTS - 1) * Self::ELEM_BITS)))
            & Self::LO_MASK;
        let new_hi = self.hi >> Self::ELEM_BITS;
        Self {
            lo: new_lo,
            hi: new_hi,
            len: self.len,
        }
        .set_elem(l - 1, top)
    }

    fn reverse_rotate(self) -> Self {
        let l = self.len();
        if l <= 1 {
            return self;
        }
        let bottom = self.elem(l - 1);
        let cleared = self.set_elem(l - 1, 0);
        let overflow =
            ((cleared.lo >> ((Self::LO_SLOTS - 1) * Self::ELEM_BITS)) & Self::ELEM_MASK_128)
                as u64;
        let new_lo = ((cleared.lo << Self::ELEM_BITS) & Self::LO_MASK)
            | ((bottom as u128) & Self::ELEM_MASK_128);
        let new_hi = (cleared.hi << Self::ELEM_BITS) | (overflow & Self::ELEM_MASK_64);
        Self {
            lo: new_lo,
            hi: new_hi,
            len: self.len,
        }
    }
}

/// Packed state of both stacks. 32 bytes, Copy, zero-alloc.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FastState {
    a: PackedStack,
    b: PackedStack,
}

impl FastState {
    fn from_stack_pair(sp: &StackPair) -> Self {
        let a: Vec<usize> = sp.a().iter().copied().collect();
        let b: Vec<usize> = sp.b().iter().copied().collect();
        Self {
            a: PackedStack::from_slice(&a),
            b: PackedStack::from_slice(&b),
        }
    }

    fn execute(self, op: Operation) -> Self {
        use Operation::*;
        match op {
            Sa => Self {
                a: self.a.swap_top(),
                b: self.b,
            },
            Sb => Self {
                a: self.a,
                b: self.b.swap_top(),
            },
            Ss => Self {
                a: self.a.swap_top(),
                b: self.b.swap_top(),
            },
            Pa => {
                if self.b.len() == 0 {
                    return self;
                }
                let (b, val) = self.b.pop_top();
                Self {
                    a: self.a.push_top(val),
                    b,
                }
            }
            Pb => {
                if self.a.len() == 0 {
                    return self;
                }
                let (a, val) = self.a.pop_top();
                Self {
                    a,
                    b: self.b.push_top(val),
                }
            }
            Ra => Self {
                a: self.a.rotate(),
                b: self.b,
            },
            Rb => Self {
                a: self.a,
                b: self.b.rotate(),
            },
            Rr => Self {
                a: self.a.rotate(),
                b: self.b.rotate(),
            },
            Rra => Self {
                a: self.a.reverse_rotate(),
                b: self.b,
            },
            Rrb => Self {
                a: self.a,
                b: self.b.reverse_rotate(),
            },
            Rrr => Self {
                a: self.a.reverse_rotate(),
                b: self.b.reverse_rotate(),
            },
        }
    }
}

/// Up to 10 operations packed into a u64.
///
/// Layout: `[len:4][op_0:4][op_1:4]...[op_{len-1}:4]`
/// 11 ops need 4 bits each. 10 ops × 4 + 4 = 44 bits ≤ 64.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PackedSequence(u64);

impl PackedSequence {
    const OP_BITS: u32 = 4;
    const LEN_BITS: u32 = 4;
    const LEN_MASK: u64 = 0xF;
    const OP_MASK: u64 = 0xF;

    fn empty() -> Self {
        Self(0)
    }

    fn len(self) -> u8 {
        (self.0 & Self::LEN_MASK) as u8
    }

    fn get(self, i: u8) -> Operation {
        let shift = Self::LEN_BITS + (i as u32) * Self::OP_BITS;
        let val = ((self.0 >> shift) & Self::OP_MASK) as usize;
        Operation::ALL[val]
    }

    fn push(self, op: Operation) -> Self {
        let l = self.len() as u32;
        let shift = Self::LEN_BITS + l * Self::OP_BITS;
        Self((self.0 & !Self::LEN_MASK) | ((l + 1) as u64) | ((op as u64) << shift))
    }

    fn suffix(self, k: u8) -> Self {
        let start = (self.len() - k) as u32;
        let src_bit = Self::LEN_BITS + start * Self::OP_BITS;
        let ops_mask = (1u64 << (k as u32 * Self::OP_BITS)) - 1;
        let ops_bits = (self.0 >> src_bit) & ops_mask;
        Self((ops_bits << Self::LEN_BITS) | k as u64)
    }

    fn from_ops(ops: &[Operation]) -> Self {
        let mut p = Self(ops.len() as u64);
        for (i, &op) in ops.iter().enumerate() {
            let shift = Self::LEN_BITS + (i as u32) * Self::OP_BITS;
            p.0 |= (op as u64) << shift;
        }
        p
    }

    fn to_vec(self) -> Vec<Operation> {
        (0..self.len()).map(|i| self.get(i)).collect()
    }
}

// ── State Engine ─────────────────────────────────────────────────────

/// Stack size for the canonical state. At least 3 so all 11 ops produce
/// distinct states (with only 2 elements, swap == rotate).
fn stack_size(n: usize) -> usize {
    (2 * n + 1).max(3)
}

/// Build a canonical state via StackPair (one-time, correctness over speed).
fn canonical_state(n: usize) -> FastState {
    let sz = stack_size(n);
    let values: Vec<usize> = (1..=2 * sz).collect();
    let mut sp = StackPair::new(values);
    for _ in 0..sz {
        sp.execute(Operation::Pb);
    }
    FastState::from_stack_pair(&sp)
}

// ── Reducible Pattern Set ────────────────────────────────────────────

struct ReducibleSet {
    by_length: Vec<HashSet<PackedSequence>>,
}

impl ReducibleSet {
    fn new() -> Self {
        Self {
            by_length: vec![HashSet::new(); N + 1],
        }
    }

    fn add(&mut self, pattern: PackedSequence) {
        self.by_length[pattern.len() as usize].insert(pattern);
    }

    fn has_reducible_suffix(&self, seq: PackedSequence) -> bool {
        let l = seq.len();
        for pat_len in 2..=l {
            let set = &self.by_length[pat_len as usize];
            if !set.is_empty() && set.contains(&seq.suffix(pat_len)) {
                return true;
            }
        }
        false
    }
}

// ── Rules ────────────────────────────────────────────────────────────

struct Rules {
    reductions: Vec<(PackedSequence, PackedSequence)>,
    annihilators: Vec<PackedSequence>,
}

impl Rules {
    fn new() -> Self {
        Self {
            reductions: Vec::new(),
            annihilators: Vec::new(),
        }
    }
}

// ── Search ───────────────────────────────────────────────────────────

/// Rebuild BFS frontier from oracle. FastState is the key, so no replay needed.
fn rebuild_frontier(
    oracle: &HashMap<FastState, PackedSequence>,
    depth: usize,
) -> Vec<(FastState, PackedSequence)> {
    oracle
        .iter()
        .filter(|(_, ops)| ops.len() as usize == depth)
        .map(|(&state, &ops)| (state, ops))
        .collect()
}

/// BFS search from `start_depth` through `max_depth`.
fn search_bfs(
    max_depth: usize,
    start_depth: usize,
    canonical: FastState,
    n: usize,
    oracle: &mut HashMap<FastState, PackedSequence>,
    rules: &mut Rules,
    reducible: &mut ReducibleSet,
) {
    let mut frontier = if start_depth == 1 {
        vec![(canonical, PackedSequence::empty())]
    } else {
        rebuild_frontier(oracle, start_depth - 1)
    };

    for depth in start_depth..=max_depth {
        let reds_before = rules.reductions.len() + rules.annihilators.len();
        eprintln!(
            "Depth {depth}: searching (oracle: {}, frontier: {})...",
            oracle.len(),
            frontier.len()
        );

        let mut next_frontier = Vec::new();
        let mut new_reducible: Vec<PackedSequence> = Vec::new();

        for &(state, ops) in &frontier {
            for &op in &Operation::ALL {
                let new_ops = ops.push(op);

                if reducible.has_reducible_suffix(new_ops) {
                    continue;
                }

                let new_state = state.execute(op);

                if let Some(&existing) = oracle.get(&new_state) {
                    if existing.len() < new_ops.len() {
                        if existing.len() == 0 {
                            rules.annihilators.push(new_ops);
                        } else {
                            rules.reductions.push((new_ops, existing));
                        }
                        new_reducible.push(new_ops);
                    }
                } else {
                    oracle.insert(new_state, new_ops);
                    next_frontier.push((new_state, new_ops));
                }
            }
        }

        for pattern in new_reducible {
            reducible.add(pattern);
        }

        let new_reds = (rules.reductions.len() + rules.annihilators.len()) - reds_before;
        eprintln!(
            "Depth {depth}: done. +{new_reds} reductions, oracle: {}",
            oracle.len()
        );

        save_cache(stack_size(n), depth, oracle, rules);
        frontier = next_frontier;
    }
}

// ── Fuzz Verifier ────────────────────────────────────────────────────

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

fn verify_rule(lhs: PackedSequence, rhs: PackedSequence, n: usize) -> bool {
    let lhs_ops = lhs.to_vec();
    let rhs_ops = rhs.to_vec();
    let mut rng = rand::rng();
    let max_total = 2 * n + 10;
    let min_per_stack = lhs_ops.len().max(rhs_ops.len()).max(2);

    for _ in 0..1000 {
        let total = rng.random_range(2 * min_per_stack..=max_total);
        let a_size = rng.random_range(min_per_stack..=total - min_per_stack);
        let b_size = total - a_size;

        let base = make_config(a_size, b_size);

        let mut sp_lhs = base.clone();
        for &op in &lhs_ops {
            sp_lhs.execute(op);
        }

        let mut sp_rhs = base;
        for &op in &rhs_ops {
            sp_rhs.execute(op);
        }

        if sp_lhs.a() != sp_rhs.a() || sp_lhs.b() != sp_rhs.b() {
            return false;
        }
    }
    true
}

// ── Persistence ──────────────────────────────────────────────────────

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
    oracle: &HashMap<FastState, PackedSequence>,
    rules: &Rules,
) {
    let oracle_entries: Vec<OracleEntry> = oracle
        .iter()
        .map(|(state, ops)| OracleEntry {
            state_a: state.a.to_vec(),
            state_b: state.b.to_vec(),
            ops: ops_to_strings(&ops.to_vec()),
        })
        .collect();

    let data = CacheData {
        canonical_n: n,
        max_depth_explored: max_depth,
        oracle: oracle_entries,
        reductions: rules
            .reductions
            .iter()
            .map(|(from, to)| (ops_to_strings(&from.to_vec()), ops_to_strings(&to.to_vec())))
            .collect(),
        annihilators: rules
            .annihilators
            .iter()
            .map(|seq| ops_to_strings(&seq.to_vec()))
            .collect(),
    };

    let json = serde_json::to_string(&data).expect("failed to serialize cache");
    fs::write(CACHE_FILE, json).expect("failed to write cache file");
}

fn load_cache() -> Option<CacheData> {
    let json = fs::read_to_string(CACHE_FILE).ok()?;
    serde_json::from_str(&json).ok()
}

fn rebuild_from_cache(
    cache: &CacheData,
) -> (HashMap<FastState, PackedSequence>, Rules, ReducibleSet) {
    let mut oracle = HashMap::new();
    for entry in &cache.oracle {
        let state = FastState {
            a: PackedStack::from_slice(&entry.state_a),
            b: PackedStack::from_slice(&entry.state_b),
        };
        let ops = PackedSequence::from_ops(&strings_to_ops(&entry.ops));
        oracle.insert(state, ops);
    }

    let mut rules = Rules::new();
    for (from, to) in &cache.reductions {
        rules
            .reductions
            .push((PackedSequence::from_ops(&strings_to_ops(from)), PackedSequence::from_ops(&strings_to_ops(to))));
    }
    for seq in &cache.annihilators {
        rules.annihilators.push(PackedSequence::from_ops(&strings_to_ops(seq)));
    }

    let mut reducible = ReducibleSet::new();
    for &(from, _) in &rules.reductions {
        reducible.add(from);
    }
    for &seq in &rules.annihilators {
        reducible.add(seq);
    }

    (oracle, rules, reducible)
}

// ── Output Formatter ─────────────────────────────────────────────────

fn fmt_ops(seq: PackedSequence) -> String {
    seq.to_vec()
        .iter()
        .map(|op| op.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn print_rules(rules: &Rules) {
    println!("## Strict Reductions (N → shorter)\n");
    println!("| From | To |");
    println!("|------|----|");
    for &(from, to) in &rules.reductions {
        println!("| {} | {} |", fmt_ops(from), fmt_ops(to));
    }

    println!("\n## Annihilators (N → empty)\n");
    println!("| Sequence |");
    println!("|----------|");
    for &seq in &rules.annihilators {
        println!("| {} |", fmt_ops(seq));
    }
}

// ── Main ─────────────────────────────────────────────────────────────

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
    if n > N {
        eprintln!("Error: max_depth must be <= {N} (packed representation limit)");
        std::process::exit(1);
    }

    let sz = stack_size(n);
    let canonical = canonical_state(n);

    let (mut oracle, mut rules, mut reducible, start_depth) = match load_cache() {
        Some(cache) if cache.canonical_n == sz => {
            if cache.max_depth_explored >= n {
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
            let mut oracle = HashMap::new();
            oracle.insert(canonical, PackedSequence::empty());
            (oracle, Rules::new(), ReducibleSet::new(), 1)
        }
    };

    search_bfs(
        n,
        start_depth,
        canonical,
        n,
        &mut oracle,
        &mut rules,
        &mut reducible,
    );

    // Fuzz-verify all discovered rules
    let total_rules = rules.reductions.len() + rules.annihilators.len();
    eprintln!("Fuzz-verifying {total_rules} rules...");
    let mut fail_count = 0;
    for &(from, to) in &rules.reductions {
        if !verify_rule(from, to, n) {
            eprintln!("  FUZZ FAIL: {} → {}", fmt_ops(from), fmt_ops(to));
            fail_count += 1;
        }
    }
    for &seq in &rules.annihilators {
        if !verify_rule(seq, PackedSequence::empty(), n) {
            eprintln!("  FUZZ FAIL: {} → ∅", fmt_ops(seq));
            fail_count += 1;
        }
    }
    if fail_count > 0 {
        eprintln!("{fail_count} rules FAILED fuzz verification!");
    } else {
        eprintln!("All rules passed fuzz verification.");
    }

    save_cache(sz, n, &oracle, &rules);

    if n <= 4 {
        print_rules(&rules);
    } else {
        eprintln!(
            "Done. {} reductions, {} annihilators. Saved to {CACHE_FILE}.",
            rules.reductions.len(),
            rules.annihilators.len()
        );
    }
}
