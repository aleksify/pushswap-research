//! Optimal solver via breadth-first search.
//!
//! Every operation has a single-op inverse, so the configuration graph is
//! undirected and unweighted — a plain BFS from the input to the sorted state
//! yields a *provably shortest* operation sequence. No heuristic can beat it.
//!
//! This is only tractable for small `n`: BFS may visit up to `(n+1)·n!`
//! configurations, so the cost grows factorially. It is wired in for
//! `n ≤ BFS_LIMIT` (see `algo.rs`), chosen so a single solve stays well under
//! a second. For larger `n` the Turk-family heuristics take over.
//!
//! Inputs are assumed pre-ranked to a permutation of `0..n` (as
//! `process_and_rank` guarantees); the Lehmer ranking below relies on it.

use crate::stacks::{Operation, StackPair};
use std::collections::VecDeque;

sort_name!();

/// Largest `n` the packed representation supports (4 bits per element).
const MAXN: usize = 12;

fn factorials() -> [u64; MAXN + 1] {
    let mut f = [1u64; MAXN + 1];
    for i in 1..=MAXN {
        f[i] = f[i - 1] * i as u64;
    }
    f
}

/// Lehmer-code rank of a permutation `seq[0..n]` of `0..n`.
#[inline]
fn rank(seq: &[u8; MAXN], n: usize, fact: &[u64; MAXN + 1]) -> u64 {
    let mut r = 0u64;
    for i in 0..n {
        let mut c = 0u64;
        for j in i + 1..n {
            if seq[j] < seq[i] {
                c += 1;
            }
        }
        r += c * fact[n - 1 - i];
    }
    r
}

/// Flat index of configuration `(seq, k)` in `[0, (n+1)·n!)`.
#[inline]
fn index(seq: &[u8; MAXN], n: usize, k: usize, fact: &[u64; MAXN + 1]) -> usize {
    (rank(seq, n, fact) * (n as u64 + 1) + k as u64) as usize
}

#[inline]
fn pack(seq: &[u8; MAXN], n: usize, k: usize) -> u64 {
    let mut x = 0u64;
    for (i, &v) in seq.iter().enumerate().take(n) {
        x |= (v as u64) << (4 * i);
    }
    x | ((k as u64) << 48)
}

#[inline]
fn unpack(x: u64, n: usize) -> ([u8; MAXN], usize) {
    let mut seq = [0u8; MAXN];
    for (i, slot) in seq.iter_mut().enumerate().take(n) {
        *slot = ((x >> (4 * i)) & 0xF) as u8;
    }
    (seq, ((x >> 48) & 0xF) as usize)
}

/// Apply op `op` (code 0..11, matching `Operation::ALL`) to config `(seq, k)`.
/// Returns the new config, or `None` if the op is invalid or a no-op.
#[inline]
fn apply(seq: &[u8; MAXN], n: usize, k: usize, op: u8) -> Option<([u8; MAXN], usize)> {
    let mut s = *seq;
    let mut k2 = k;
    match op {
        0 => {
            // sa
            if k < 2 {
                return None;
            }
            s.swap(0, 1);
        }
        1 => {
            // sb
            if n - k < 2 {
                return None;
            }
            s.swap(k, k + 1);
        }
        2 => {
            // ss
            let mut ch = false;
            if k >= 2 {
                s.swap(0, 1);
                ch = true;
            }
            if n - k >= 2 {
                s.swap(k, k + 1);
                ch = true;
            }
            if !ch {
                return None;
            }
        }
        3 => {
            // pa: B top -> A top
            if k >= n {
                return None;
            }
            s = [0u8; MAXN];
            s[0] = seq[k];
            s[1..=k].copy_from_slice(&seq[0..k]);
            s[k + 1..n].copy_from_slice(&seq[k + 1..n]);
            k2 = k + 1;
        }
        4 => {
            // pb: A top -> B top
            if k == 0 {
                return None;
            }
            s = [0u8; MAXN];
            s[0..k - 1].copy_from_slice(&seq[1..k]);
            s[k - 1] = seq[0];
            s[k..n].copy_from_slice(&seq[k..n]);
            k2 = k - 1;
        }
        5 => {
            // ra
            if k < 2 {
                return None;
            }
            let f = seq[0];
            s[0..k - 1].copy_from_slice(&seq[1..k]);
            s[k - 1] = f;
        }
        6 => {
            // rb
            if n - k < 2 {
                return None;
            }
            let f = seq[k];
            s[k..n - 1].copy_from_slice(&seq[k + 1..n]);
            s[n - 1] = f;
        }
        7 => {
            // rr
            let mut ch = false;
            if k >= 2 {
                let f = seq[0];
                s[0..k - 1].copy_from_slice(&seq[1..k]);
                s[k - 1] = f;
                ch = true;
            }
            if n - k >= 2 {
                let f = seq[k];
                s[k..n - 1].copy_from_slice(&seq[k + 1..n]);
                s[n - 1] = f;
                ch = true;
            }
            if !ch {
                return None;
            }
        }
        8 => {
            // rra
            if k < 2 {
                return None;
            }
            let l = seq[k - 1];
            s[1..k].copy_from_slice(&seq[0..k - 1]);
            s[0] = l;
        }
        9 => {
            // rrb
            if n - k < 2 {
                return None;
            }
            let l = seq[n - 1];
            s[k + 1..n].copy_from_slice(&seq[k..n - 1]);
            s[k] = l;
        }
        10 => {
            // rrr
            let mut ch = false;
            if k >= 2 {
                let l = seq[k - 1];
                s[1..k].copy_from_slice(&seq[0..k - 1]);
                s[0] = l;
                ch = true;
            }
            if n - k >= 2 {
                let l = seq[n - 1];
                s[k + 1..n].copy_from_slice(&seq[k..n - 1]);
                s[k] = l;
                ch = true;
            }
            if !ch {
                return None;
            }
        }
        _ => unreachable!(),
    }
    if s == *seq && k2 == k {
        None
    } else {
        Some((s, k2))
    }
}

/// Sort stack A optimally using BFS. Assumes A holds a permutation of `0..n`
/// with B empty, and `n <= MAXN` (the caller gates on `BFS_LIMIT`).
pub fn sort_bfs(stacks: &mut StackPair) {
    let n = stacks.a().len();
    if n <= 1 || !stacks.b().is_empty() {
        return;
    }
    debug_assert!(n <= MAXN);

    let fact = factorials();
    let mut start = [0u8; MAXN];
    for (slot, &v) in start.iter_mut().zip(stacks.a().iter()) {
        *slot = v as u8;
    }

    let mut goal = [0u8; MAXN];
    for (i, slot) in goal.iter_mut().enumerate().take(n) {
        *slot = i as u8;
    }
    let start_idx = index(&start, n, n, &fact);
    let goal_idx = index(&goal, n, n, &fact);
    if start_idx == goal_idx {
        return; // already sorted
    }

    // BFS from the input, recording for each state the op + predecessor that
    // first reached it. Stop as soon as the sorted state is labeled.
    let total = (fact[n] * (n as u64 + 1)) as usize;
    let mut prev_idx = vec![u32::MAX; total];
    let mut prev_op = vec![0u8; total];
    prev_idx[start_idx] = start_idx as u32; // root sentinel (points to itself)

    let mut q: VecDeque<u64> = VecDeque::new();
    q.push_back(pack(&start, n, n));

    'bfs: while let Some(x) = q.pop_front() {
        let (seq, k) = unpack(x, n);
        let cur = index(&seq, n, k, &fact);
        for op in 0..11u8 {
            if let Some((s2, k2)) = apply(&seq, n, k, op) {
                let idx = index(&s2, n, k2, &fact);
                if prev_idx[idx] == u32::MAX {
                    prev_idx[idx] = cur as u32;
                    prev_op[idx] = op;
                    if idx == goal_idx {
                        break 'bfs;
                    }
                    q.push_back(pack(&s2, n, k2));
                }
            }
        }
    }

    // Reconstruct the op sequence (goal back to start) and execute it forward.
    let mut ops = Vec::new();
    let mut idx = goal_idx;
    while idx != start_idx {
        ops.push(prev_op[idx]);
        idx = prev_idx[idx] as usize;
    }
    ops.reverse();
    for code in ops {
        stacks.execute(Operation::ALL[code as usize]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    type Key = (Vec<usize>, Vec<usize>);

    fn state_key(sp: &StackPair) -> Key {
        (
            sp.a().iter().copied().collect(),
            sp.b().iter().copied().collect(),
        )
    }

    /// Reference shortest-distance oracle via an independent `StackPair`-based
    /// BFS from the sorted state (different code path than the flat solver).
    /// `dist[(a, b)]` is the optimal number of ops to sort that configuration.
    fn reference_distances(n: usize) -> HashMap<Key, usize> {
        let mut start = StackPair::new((0..n).collect());
        start.set_logs(vec![]);
        let mut dist = HashMap::new();
        dist.insert(state_key(&start), 0usize);
        let mut q = VecDeque::new();
        q.push_back(start);
        while let Some(sp) = q.pop_front() {
            let d = dist[&state_key(&sp)];
            for &op in &Operation::ALL {
                let mut next = sp.clone();
                next.execute(op);
                let key = state_key(&next);
                if key != state_key(&sp) && !dist.contains_key(&key) {
                    dist.insert(key, d + 1);
                    q.push_back(next);
                }
            }
        }
        dist
    }

    fn perms(n: usize) -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        let mut cur: Vec<usize> = (0..n).collect();
        permute(&mut cur, 0, &mut out);
        out
    }
    fn permute(cur: &mut Vec<usize>, i: usize, out: &mut Vec<Vec<usize>>) {
        if i == cur.len() {
            out.push(cur.clone());
            return;
        }
        for j in i..cur.len() {
            cur.swap(i, j);
            permute(cur, i + 1, out);
            cur.swap(i, j);
        }
    }

    #[test]
    fn bfs_sorts_and_is_optimal_small_n() {
        for n in 2..=7 {
            let dist = reference_distances(n);
            for p in perms(n) {
                let mut sp = StackPair::new(p.clone());
                sort_bfs(&mut sp);
                assert!(sp.is_sorted(), "n={n} perm={p:?} not sorted by bfs");
                let want = dist[&(p.clone(), Vec::new())];
                assert_eq!(
                    sp.total_ops(),
                    want,
                    "n={n} perm={p:?}: bfs used {} ops, optimal is {want}",
                    sp.total_ops()
                );
            }
        }
    }
}
