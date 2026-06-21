//! N1 re-optimizer: exact shortest-path between two concrete (A,B) states via
//! bidirectional BFS. The op graph is undirected (every op has a single-op
//! inverse), so both frontiers expand with the same 11 ops; we meet in the
//! middle and reconstruct. Used to replace a sub-span of a solver's output
//! with the provably-shortest sequence achieving the identical state
//! transition — correct by construction (the spliced segment is verified to
//! reproduce the end state).

use crate::stacks::Operation;
use std::collections::HashMap;

/// A concrete configuration: `a` (top at index 0) and `b` (top at index 0).
type State = (Vec<usize>, Vec<usize>);

/// Absolute cap on the bidirectional search depth. A window whose optimal path
/// exceeds this is left unchanged. Keeps each side's frontier ~`b^(CAP/2)`.
const REOPT_MAX_DEPTH: usize = 18;

/// Hard node budget per window: abandon the search if the two visited maps
/// together exceed this. Anti-hang safety; never affects correctness. Tight on
/// purpose — improvable windows meet at low depth, so a small budget keeps
/// non-improvable windows cheap to reject.
const MAX_STATES: usize = 40_000;

/// Functionally apply `op`, returning the new state, or `None` if it's a no-op
/// (no stack changed) — matching `StackPair::execute`'s success semantics.
fn apply(op: Operation, a: &[usize], b: &[usize]) -> Option<State> {
    use Operation::*;
    let mut a = a.to_vec();
    let mut b = b.to_vec();
    let changed = match op {
        Sa => swap(&mut a),
        Sb => swap(&mut b),
        Ss => swap(&mut a) | swap(&mut b),
        Pa => push(&mut a, &mut b),
        Pb => push(&mut b, &mut a),
        Ra => rot(&mut a),
        Rb => rot(&mut b),
        Rr => rot(&mut a) | rot(&mut b),
        Rra => rrot(&mut a),
        Rrb => rrot(&mut b),
        Rrr => rrot(&mut a) | rrot(&mut b),
    };
    if changed { Some((a, b)) } else { None }
}

fn swap(s: &mut [usize]) -> bool {
    if s.len() >= 2 {
        s.swap(0, 1);
        true
    } else {
        false
    }
}

fn push(dst: &mut Vec<usize>, src: &mut Vec<usize>) -> bool {
    if let Some(v) = src.first().copied() {
        src.remove(0);
        dst.insert(0, v);
        true
    } else {
        false
    }
}

fn rot(s: &mut Vec<usize>) -> bool {
    if s.len() >= 2 {
        let v = s.remove(0);
        s.push(v);
        true
    } else {
        false
    }
}

fn rrot(s: &mut Vec<usize>) -> bool {
    if s.len() >= 2 {
        let v = s.pop().unwrap();
        s.insert(0, v);
        true
    } else {
        false
    }
}

/// Single-op inverse. `pa`/`pb` and `ra`/`rra` (+ b/composite variants) swap;
/// swaps are self-inverse.
fn inverse(op: Operation) -> Operation {
    use Operation::*;
    match op {
        Sa => Sa,
        Sb => Sb,
        Ss => Ss,
        Pa => Pb,
        Pb => Pa,
        Ra => Rra,
        Rb => Rrb,
        Rr => Rrr,
        Rra => Ra,
        Rrb => Rb,
        Rrr => Rr,
    }
}

/// Generous node budget for the full-state reference search (correctness over
/// speed; used only by tests).
const REF_MAX_STATES: usize = 4_000_000;

/// Shortest op sequence taking `start` → `goal` on the full-state graph.
pub fn shortest_path(start: &State, goal: &State, max_len: usize) -> Option<Vec<Operation>> {
    bidir_bfs(
        start.clone(),
        goal.clone(),
        max_len,
        REF_MAX_STATES,
        |op, s| apply(op, &s.0, &s.1),
    )
}

/// Shortest op sequence on the rank-compressed graph (cost independent of n).
fn shortest_path_c(start: CState, goal: CState, max_len: usize) -> Option<Vec<Operation>> {
    bidir_bfs(start, goal, max_len, MAX_STATES, apply_c)
}

/// Bidirectional BFS over the undirected configuration graph for any state type
/// `S` with a functional `step`. Returns the shortest op sequence `start →
/// goal`, or `None` if none is found within `max_len` depth / the node budget.
fn bidir_bfs<S, F>(
    start: S,
    goal: S,
    max_len: usize,
    max_states: usize,
    mut step: F,
) -> Option<Vec<Operation>>
where
    S: Clone + std::hash::Hash + Eq,
    F: FnMut(Operation, &S) -> Option<S>,
{
    if start == goal {
        return Some(Vec::new());
    }
    // Each side: state -> (parent state, op applied to parent to reach state).
    let mut fwd: HashMap<S, Option<(S, Operation)>> = HashMap::new();
    let mut bwd: HashMap<S, Option<(S, Operation)>> = HashMap::new();
    let mut fd: HashMap<S, usize> = HashMap::new();
    let mut bd: HashMap<S, usize> = HashMap::new();
    fwd.insert(start.clone(), None);
    bwd.insert(goal.clone(), None);
    fd.insert(start.clone(), 0);
    bd.insert(goal.clone(), 0);
    let mut ffront = vec![start];
    let mut bfront = vec![goal];
    let mut fdepth = 0usize;
    let mut bdepth = 0usize;
    let mut best = usize::MAX;
    let mut best_meet: Option<S> = None;

    while !ffront.is_empty() && !bfront.is_empty() {
        if fdepth + bdepth >= best || fdepth + bdepth >= max_len {
            break;
        }
        // Hard safety budget: abandon if the search balloons. Never incorrect.
        if fwd.len() + bwd.len() > max_states {
            break;
        }
        let expand_fwd = ffront.len() <= bfront.len();
        let (front, seen, seen_d, other_d) = if expand_fwd {
            (&mut ffront, &mut fwd, &mut fd, &bd)
        } else {
            (&mut bfront, &mut bwd, &mut bd, &fd)
        };
        let nd = if expand_fwd { fdepth + 1 } else { bdepth + 1 };
        let mut next = Vec::new();
        for st in front.drain(..) {
            for op in Operation::ALL {
                if let Some(ns) = step(op, &st) {
                    if seen.contains_key(&ns) {
                        continue;
                    }
                    seen.insert(ns.clone(), Some((st.clone(), op)));
                    seen_d.insert(ns.clone(), nd);
                    if let Some(&od) = other_d.get(&ns) {
                        let total = nd + od;
                        if total < best {
                            best = total;
                            best_meet = Some(ns.clone());
                        }
                    }
                    next.push(ns);
                }
            }
        }
        *front = next;
        if expand_fwd {
            fdepth += 1;
        } else {
            bdepth += 1;
        }
    }
    best_meet.map(|m| reconstruct(&m, &fwd, &bwd))
}

/// Rebuild the path start → goal through meeting state `meet`, present in both
/// parent maps. Forward side gives start→meet; backward gives goal→meet, which
/// we invert for meet→goal.
fn reconstruct<S: Clone + std::hash::Hash + Eq>(
    meet: &S,
    fwd: &HashMap<S, Option<(S, Operation)>>,
    bwd: &HashMap<S, Option<(S, Operation)>>,
) -> Vec<Operation> {
    let mut ops = Vec::new();
    let mut cur = meet.clone();
    while let Some((p, o)) = fwd.get(&cur).expect("fwd parent") {
        ops.push(*o);
        cur = p.clone();
    }
    ops.reverse();
    let mut cur = meet.clone();
    while let Some((p, o)) = bwd.get(&cur).expect("bwd parent") {
        ops.push(inverse(*o));
        cur = p.clone();
    }
    ops
}

/// Replay `logs` from `(initial_a, [])`, snapshotting the state at each index
/// in `boundaries` (assumed sorted, deduped).
fn snapshots(
    initial_a: &[usize],
    logs: &[Operation],
    boundaries: &[usize],
) -> HashMap<usize, State> {
    let mut snap = HashMap::new();
    let mut a = initial_a.to_vec();
    let mut b: Vec<usize> = Vec::new();
    let mut bi = 0;
    for (idx, &op) in logs.iter().enumerate() {
        while bi < boundaries.len() && boundaries[bi] == idx {
            snap.insert(idx, (a.clone(), b.clone()));
            bi += 1;
        }
        if let Some((na, nb)) = apply(op, &a, &b) {
            a = na;
            b = nb;
        }
    }
    while bi < boundaries.len() && boundaries[bi] == logs.len() {
        snap.insert(logs.len(), (a.clone(), b.clone()));
        bi += 1;
    }
    snap
}

/// Subtree-size cap for the N1 racers. K=6 is the measured sweet spot.
pub const N1_K: usize = 6;

/// Inverse of a rank permutation (`0..n`).
fn inverse_perm(ranked: &[usize]) -> Vec<usize> {
    let mut inv = vec![0usize; ranked.len()];
    for (i, &r) in ranked.iter().enumerate() {
        inv[r] = i;
    }
    inv
}

/// Solve `ranked` with quick3 (`cfg`) and N1-reoptimize its subtrees. Returns a
/// `StackPair` holding the input with the reoptimized log applied.
pub fn quick3_n1(ranked: &[usize], cfg: crate::algo::PivotCfg) -> crate::stacks::StackPair {
    use crate::stacks::StackPair;
    let mut s = StackPair::new(ranked.to_vec());
    let spans = crate::algo::sort_quick3_with_spans(&mut s, cfg, N1_K);
    let reopt = reoptimize_spans(ranked, s.logs(), &spans);
    let mut out = StackPair::new(ranked.to_vec());
    for op in reopt {
        out.execute(op);
    }
    out
}

/// Reverse twin of [`quick3_n1`] (idea N2 ∘ N1): N1-reoptimize the *inverse*
/// solve, then reverse-invert it back to a solver for `ranked`.
pub fn quick3_n1_rev(ranked: &[usize], cfg: crate::algo::PivotCfg) -> crate::stacks::StackPair {
    use crate::stacks::StackPair;
    let inv = inverse_perm(ranked);
    let mut s = StackPair::new(inv.clone());
    let spans = crate::algo::sort_quick3_with_spans(&mut s, cfg, N1_K);
    let reopt = reoptimize_spans(&inv, s.logs(), &spans);
    let mut out = StackPair::new(ranked.to_vec());
    for op in reopt.iter().rev() {
        out.execute(inverse(*op));
    }
    out
}

/// Apply `ops` to `state`, returning the resulting state.
fn run_ops(state: &State, ops: &[Operation]) -> State {
    let mut s = state.clone();
    for &op in ops {
        if let Some(ns) = apply(op, &s.0, &s.1) {
            s = ns;
        }
    }
    s
}

/// Re-optimize `logs` (a full solve from `(initial_a, [])`) by replacing each
/// subtree span `[start, end)` with a shorter op sequence found by searching the
/// rank-compressed window graph (active set = the chunk's `values`). Every
/// candidate replacement is verified to reproduce the real end state before it's
/// accepted, so the result always reaches the same final state and is `<=` the
/// input length.
pub fn reoptimize_spans(
    initial_a: &[usize],
    logs: &[Operation],
    spans: &[crate::algo::SubtreeSpan],
) -> Vec<Operation> {
    let mut spans: Vec<&crate::algo::SubtreeSpan> =
        spans.iter().filter(|s| s.end > s.start).collect();
    spans.sort_by_key(|s| (s.start, s.end));
    let mut bounds: Vec<usize> = Vec::with_capacity(spans.len() * 2);
    for s in &spans {
        bounds.push(s.start);
        bounds.push(s.end);
    }
    bounds.sort_unstable();
    bounds.dedup();
    let snap = snapshots(initial_a, logs, &bounds);

    let mut out: Vec<Operation> = Vec::with_capacity(logs.len());
    let mut prev_end = 0;
    for s in spans {
        let (i, j) = (s.start, s.end);
        if i < prev_end {
            continue; // overlap guard (shouldn't happen)
        }
        out.extend_from_slice(&logs[prev_end..i]);
        let si = &snap[&i];
        let sj = &snap[&j];
        let active: std::collections::HashSet<usize> = s.values.iter().copied().collect();
        let cap = (j - i).min(REOPT_MAX_DEPTH);
        let found = shortest_path_c(compress(si, &active), compress(sj, &active), cap);
        match found {
            // verify the candidate path reproduces the real end state
            Some(p) if p.len() < j - i && run_ops(si, &p) == *sj => out.extend(p),
            _ => out.extend_from_slice(&logs[i..j]),
        }
        prev_end = j;
    }
    out.extend_from_slice(&logs[prev_end..]);
    out
}

// ===========================================================================
// Rank-compressed window states (O(K) instead of O(n))
// ===========================================================================
//
// Ops are value-blind and a window's "passive" elements (everything outside the
// active set D) keep their relative order, so passives are interchangeable: we
// track only active-element labels separated by *gap counts* of passives. A
// stack, top→bottom, is `gaps[0]` passives, `labels[0]`, `gaps[1]` passives,
// `labels[1]`, … , `labels[k-1]`, `gaps[k]` passives  (`gaps.len()==k+1`).
// Rotations/pushes/swaps become count edits → cost independent of n.
//
// Soundness: swapping two passives, or any reordering of passives, is treated as
// a no-op (forbidden). That only ever *omits* moves an optimal settled-order-
// preserving path wouldn't make; the spliced result is verified on the real
// state regardless, so the model can never cause an incorrect sort.

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct CStack {
    labels: Vec<u16>,
    gaps: Vec<u32>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct CState {
    a: CStack,
    b: CStack,
}

impl CStack {
    fn len(&self) -> usize {
        self.labels.len() + self.gaps.iter().map(|&g| g as usize).sum::<usize>()
    }

    /// Rotate up (top → bottom). Returns false if it was a no-op.
    fn rot(&mut self) -> bool {
        if self.len() < 2 {
            return false;
        }
        if self.gaps[0] > 0 {
            let last = self.gaps.len() - 1;
            self.gaps[0] -= 1;
            self.gaps[last] += 1;
        } else {
            // top is labels[0]; move it to the bottom
            let l0 = self.labels.remove(0);
            let _g0 = self.gaps.remove(0); // == 0
            self.labels.push(l0);
            self.gaps.push(0);
        }
        true
    }

    /// Rotate down (bottom → top).
    fn rrot(&mut self) -> bool {
        if self.len() < 2 {
            return false;
        }
        let last = self.gaps.len() - 1;
        if self.gaps[last] > 0 {
            self.gaps[last] -= 1;
            self.gaps[0] += 1;
        } else {
            let lk = self.labels.pop().unwrap();
            self.gaps.pop(); // == 0
            self.labels.insert(0, lk);
            self.gaps.insert(0, 0);
        }
        true
    }

    /// Swap top two elements.
    fn swap2(&mut self) -> bool {
        if self.len() < 2 {
            return false;
        }
        if self.gaps[0] >= 2 {
            return false; // passive,passive → order-preserving no-op
        }
        if self.gaps[0] == 1 {
            // passive, active0  -> active0, passive
            self.gaps[0] = 0;
            self.gaps[1] += 1;
            true
        } else {
            // gaps[0]==0, top is labels[0]
            if self.gaps[1] > 0 {
                // active0, passive -> passive, active0
                self.gaps[0] += 1;
                self.gaps[1] -= 1;
                true
            } else {
                // active0, active1 -> swap labels
                self.labels.swap(0, 1);
                true
            }
        }
    }

    /// Pop the top element off (for a push). Returns `Some(Some(label))` for an
    /// active, `Some(None)` for a passive, or `None` if empty.
    fn pop_top(&mut self) -> Option<Option<u16>> {
        if self.len() == 0 {
            return None;
        }
        if self.gaps[0] > 0 {
            self.gaps[0] -= 1;
            Some(None)
        } else {
            let l0 = self.labels.remove(0);
            let g0 = self.gaps.remove(0); // == 0
            self.gaps[0] += g0; // 0; keep front gap
            Some(Some(l0))
        }
    }

    /// Push an element onto the top.
    fn push_top(&mut self, item: Option<u16>) {
        match item {
            None => self.gaps[0] += 1,
            Some(l) => {
                self.labels.insert(0, l);
                self.gaps.insert(0, 0);
            }
        }
    }
}

fn compress_stack(stk: &[usize], active: &std::collections::HashSet<usize>) -> CStack {
    let mut labels = Vec::new();
    let mut gaps = vec![0u32];
    for &v in stk {
        if active.contains(&v) {
            labels.push(v as u16);
            gaps.push(0);
        } else {
            *gaps.last_mut().unwrap() += 1;
        }
    }
    CStack { labels, gaps }
}

fn compress(state: &State, active: &std::collections::HashSet<usize>) -> CState {
    CState {
        a: compress_stack(&state.0, active),
        b: compress_stack(&state.1, active),
    }
}

/// Functionally apply `op` to a compressed state, or `None` if no-op.
fn apply_c(op: Operation, s: &CState) -> Option<CState> {
    use Operation::*;
    let mut s = s.clone();
    let changed = match op {
        Sa => s.a.swap2(),
        Sb => s.b.swap2(),
        Ss => {
            let x = s.a.swap2();
            let y = s.b.swap2();
            x || y
        }
        Pa => {
            if let Some(item) = s.b.pop_top() {
                s.a.push_top(item);
                true
            } else {
                false
            }
        }
        Pb => {
            if let Some(item) = s.a.pop_top() {
                s.b.push_top(item);
                true
            } else {
                false
            }
        }
        Ra => s.a.rot(),
        Rb => s.b.rot(),
        Rr => {
            let x = s.a.rot();
            let y = s.b.rot();
            x || y
        }
        Rra => s.a.rrot(),
        Rrb => s.b.rrot(),
        Rrr => {
            let x = s.a.rrot();
            let y = s.b.rrot();
            x || y
        }
    };
    if changed { Some(s) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::PivotCfg;
    use crate::algo::sort_quick3_with_spans;
    use crate::stacks::StackPair;
    use std::collections::HashSet;

    const STOCK: PivotCfg = PivotCfg {
        p2_den: 3,
        a_p1_num: 2,
        a_p1_den: 3,
        b_p1_den: 2,
    };

    /// Compressed `apply` must agree with full-state `apply` (the oracle) under
    /// compression, for every op, over random states and active sets.
    #[test]
    fn compressed_apply_matches_full() {
        let mut rng: u64 = 0x5151_2727_9393_ABCD;
        let mut next = |bound: usize| {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((rng >> 33) as usize) % bound
        };
        for _ in 0..5000 {
            let n = 2 + next(10); // 2..=11
            let mut vals: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                vals.swap(i, next(i + 1));
            }
            let split = next(n + 1);
            let full: State = (vals[split..].to_vec(), vals[..split].to_vec());
            // random active subset
            let mut active = HashSet::new();
            for v in 0..n {
                if next(2) == 0 {
                    active.insert(v);
                }
            }
            let comp = compress(&full, &active);
            for op in Operation::ALL {
                let full_after = apply(op, &full.0, &full.1).unwrap_or_else(|| full.clone());
                let expected = compress(&full_after, &active);
                let got = apply_c(op, &comp).unwrap_or_else(|| comp.clone());
                assert_eq!(
                    got, expected,
                    "op {op:?} mismatch\n full={full:?} active={active:?}\n comp={comp:?}"
                );
            }
        }
    }

    #[test]
    fn reopt_keeps_sorted_and_shortens() {
        let mut rng: u64 = 0x0BAD_C0DE_0BAD_F00D;
        let mut next = |bound: usize| {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((rng >> 33) as usize) % bound
        };
        for _ in 0..30 {
            let n = 20 + next(80); // 20..=99
            let mut vals: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                vals.swap(i, next(i + 1));
            }
            let mut s = StackPair::new(vals.clone());
            let spans = sort_quick3_with_spans(&mut s, STOCK, 6);
            let orig = s.logs().to_vec();
            let reopt = reoptimize_spans(&vals, &orig, &spans);
            assert!(reopt.len() <= orig.len(), "reopt must not grow");
            // verify it sorts
            let mut chk = StackPair::new(vals.clone());
            for &op in &reopt {
                chk.execute(op);
            }
            assert!(chk.is_sorted(), "reoptimized log must sort (n={n})");
        }
    }

    /// Apply a path and return the resulting state.
    fn run(start: &State, path: &[Operation]) -> State {
        let mut s = start.clone();
        for &op in path {
            if let Some(ns) = apply(op, &s.0, &s.1) {
                s = ns;
            }
        }
        s
    }

    /// Independent reference: plain forward BFS from `start` to `goal`.
    fn bfs_optimal(start: &State, goal: &State) -> usize {
        use std::collections::VecDeque;
        let mut dist: HashMap<State, usize> = HashMap::new();
        dist.insert(start.clone(), 0);
        let mut q = VecDeque::new();
        q.push_back(start.clone());
        while let Some(st) = q.pop_front() {
            let d = dist[&st];
            if st == *goal {
                return d;
            }
            for op in Operation::ALL {
                if let Some(ns) = apply(op, &st.0, &st.1) {
                    if !dist.contains_key(&ns) {
                        dist.insert(ns.clone(), d + 1);
                        q.push_back(ns);
                    }
                }
            }
        }
        unreachable!("goal reachable")
    }

    #[test]
    fn is_optimal_vs_plain_bfs() {
        let mut rng: u64 = 0xFEED_FACE_C0DE_1234;
        let mut next = |bound: usize| {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((rng >> 33) as usize) % bound
        };
        for _ in 0..300 {
            let n = 2 + next(6); // 2..=7 (plain BFS stays cheap)
            let mut vals: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                vals.swap(i, next(i + 1));
            }
            // random A/B split to exercise non-empty B
            let split = next(n + 1);
            let start: State = (vals[split..].to_vec(), vals[..split].to_vec());
            let goal: State = ((0..n).collect(), Vec::new());
            let path = shortest_path(&start, &goal, 60).expect("path");
            assert_eq!(run(&start, &path), goal);
            assert_eq!(path.len(), bfs_optimal(&start, &goal), "must be optimal");
        }
    }

    #[test]
    fn reaches_goal_small() {
        // deterministic LCG over many random small (a,b) splits
        let mut rng: u64 = 0x1234_5678_9ABC_DEF0;
        let mut next = |bound: usize| {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((rng >> 33) as usize) % bound
        };
        for _ in 0..2000 {
            let n = 2 + next(7); // 2..=8
            let mut vals: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                vals.swap(i, next(i + 1));
            }
            let start: State = (vals, Vec::new());
            let goal: State = ((0..n).collect(), Vec::new());
            let path = shortest_path(&start, &goal, 40).expect("path");
            assert_eq!(run(&start, &path), goal, "path must reach goal");
        }
    }
}
