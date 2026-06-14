// Exact optimal-length oracle via full-diameter BFS.
//
// Every push_swap operation has a single-op inverse (sa⁻¹=sa, pa⁻¹=pb,
// ra⁻¹=rra, rr⁻¹=rrr, ss⁻¹=ss), so the configuration graph is UNDIRECTED.
// One BFS from the sorted state therefore labels every reachable
// configuration with its exact distance to sorted = its optimal solve
// length. This is ground truth: no heuristics, no sampling.
//
// A configuration is the n distinct values {0..n} split across A and B,
// each ordered. We linearize it as `seq` = (A top→bottom, then B top→bottom)
// plus k = |A|, and rank it to an integer index in [0, (n+1)·n!):
//
//     idx = lehmer_rank(seq) * (n+1) + k
//
// `dist[idx]` holds the distance. The n! permutation inputs (B empty, k=n)
// sit at idx = rank·(n+1) + n, so reading their distances needs no unrank.
//
// Usage:  exact_bfs <n> [sample]      (n in 2..=12; n=12 needs PS_ALLOW_BIG=1)
//   writes bfs_n{N}.json with sphere sizes, the permutation-distance
//   histogram, and optimal-path op statistics over `sample` random inputs.

use rand::RngExt;
use serde_json::json;
use std::collections::VecDeque;

const MAXN: usize = 12;

const OP_NAMES: [&str; 11] = [
    "sa", "sb", "ss", "pa", "pb", "ra", "rb", "rr", "rra", "rrb", "rrr",
];

/// Lehmer-code rank of a permutation `seq[0..n]` of {0..n}.
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

/// Inverse of `rank`: recover `seq[0..n]` from its rank.
fn unrank(mut r: u64, n: usize, fact: &[u64; MAXN + 1]) -> [u8; MAXN] {
    let mut avail: Vec<u8> = (0..n as u8).collect();
    let mut s = [0u8; MAXN];
    for i in 0..n {
        let f = fact[n - 1 - i];
        let q = (r / f) as usize;
        r %= f;
        s[i] = avail.remove(q);
    }
    s
}

#[inline]
fn pack(seq: &[u8; MAXN], n: usize, k: usize, d: u8) -> u64 {
    let mut x = 0u64;
    for (i, &v) in seq.iter().enumerate().take(n) {
        x |= (v as u64) << (4 * i);
    }
    x | ((k as u64) << 48) | ((d as u64) << 56)
}

#[inline]
fn unpack(x: u64, n: usize) -> ([u8; MAXN], usize, u8) {
    let mut seq = [0u8; MAXN];
    for (i, slot) in seq.iter_mut().enumerate().take(n) {
        *slot = ((x >> (4 * i)) & 0xF) as u8;
    }
    (seq, ((x >> 48) & 0xF) as usize, ((x >> 56) & 0xFF) as u8)
}

/// Apply op `op` to config (seq, k). Returns the new config, or None if the
/// op is invalid or a no-op (leaves the configuration unchanged).
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

fn inversions(seq: &[u8; MAXN], n: usize) -> usize {
    let mut c = 0;
    for i in 0..n {
        for j in i + 1..n {
            if seq[i] > seq[j] {
                c += 1;
            }
        }
    }
    c
}

/// Longest strictly-increasing subsequence (O(n²); n is tiny).
fn lis(seq: &[u8; MAXN], n: usize) -> usize {
    let mut dp = [1usize; MAXN];
    let mut best = if n == 0 { 0 } else { 1 };
    for i in 0..n {
        for j in 0..i {
            if seq[j] < seq[i] && dp[j] + 1 > dp[i] {
                dp[i] = dp[j] + 1;
            }
        }
        best = best.max(dp[i]);
    }
    best
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args
        .get(1)
        .and_then(|s| s.parse().ok())
        .filter(|&n| (2..=MAXN).contains(&n))
        .unwrap_or_else(|| {
            eprintln!("usage: exact_bfs <n in 2..=12> [sample]");
            std::process::exit(1);
        });
    let sample: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(50_000);

    let mut fact = [1u64; MAXN + 1];
    for i in 1..=MAXN {
        fact[i] = fact[i - 1] * i as u64;
    }
    let nperm = fact[n];
    let nstates = nperm * (n as u64 + 1);
    eprintln!(
        "n={n}: {nperm} permutations, {nstates} configurations ({:.2} GB dist array)",
        nstates as f64 / 1e9
    );
    if nstates > 2_000_000_000 && std::env::var("PS_ALLOW_BIG").is_err() {
        eprintln!("dist array > 2 GB; set PS_ALLOW_BIG=1 to proceed");
        std::process::exit(1);
    }

    // ── BFS from the sorted state over the undirected config graph ──
    let mut dist: Vec<u8> = vec![u8::MAX; nstates as usize];
    let mut q: VecDeque<u64> = VecDeque::new();
    let mut sphere = vec![0u64; 64];

    let mut goal = [0u8; MAXN];
    for (i, slot) in goal.iter_mut().enumerate().take(n) {
        *slot = i as u8;
    }
    let gidx = (rank(&goal, n, &fact) * (n as u64 + 1) + n as u64) as usize;
    dist[gidx] = 0;
    sphere[0] = 1;
    q.push_back(pack(&goal, n, n, 0));

    let mut reached = 1u64;
    while let Some(x) = q.pop_front() {
        let (seq, k, d) = unpack(x, n);
        for op in 0..11u8 {
            if let Some((s2, k2)) = apply(&seq, n, k, op) {
                let idx = (rank(&s2, n, &fact) * (n as u64 + 1) + k2 as u64) as usize;
                if dist[idx] == u8::MAX {
                    dist[idx] = d + 1;
                    sphere[(d + 1) as usize] += 1;
                    reached += 1;
                    q.push_back(pack(&s2, n, k2, d + 1));
                }
            }
        }
    }
    let diameter = sphere.iter().rposition(|&c| c > 0).unwrap();
    eprintln!("BFS done: reached {reached}/{nstates} configs, eccentricity(sorted)={diameter}",);
    if reached != nstates {
        eprintln!(
            "WARNING: graph not fully connected ({} unreached)",
            nstates - reached
        );
    }

    // ── Distribution of optimal solve lengths over all n! permutations ──
    // A permutation input is config (seq=perm, k=n): idx = rank·(n+1) + n.
    let mut perm_hist = vec![0u64; (diameter + 1) as usize];
    let mut sum = 0u64;
    for r in 0..nperm {
        let idx = (r * (n as u64 + 1) + n as u64) as usize;
        let d = dist[idx];
        perm_hist[d as usize] += 1;
        sum += d as u64;
    }
    let mean = sum as f64 / nperm as f64;
    let pmin = perm_hist.iter().position(|&c| c > 0).unwrap();
    let pmax = perm_hist.iter().rposition(|&c| c > 0).unwrap();
    let mut acc = 0u64;
    let half = nperm / 2;
    let mut median = pmax;
    for (d, &c) in perm_hist.iter().enumerate() {
        acc += c;
        if acc >= half {
            median = d;
            break;
        }
    }
    eprintln!("permutation optimal length: min={pmin} median={median} mean={mean:.3} max={pmax}",);

    // ── Optimal-path structure over a random sample of inputs ──
    // Reconstruct ONE optimal path per sampled input by greedy descent on
    // `dist` (random tie-break), tallying op usage and structure.
    let mut rng = rand::rng();
    let do_all = nperm <= sample as u64;
    let n_samples = if do_all { nperm } else { sample as u64 };
    let mut op_freq = [0u64; 11];
    let mut s_opt: Vec<u32> = Vec::with_capacity(n_samples as usize);
    let mut s_pb: Vec<u32> = Vec::with_capacity(n_samples as usize);
    let mut s_comp: Vec<u32> = Vec::with_capacity(n_samples as usize);
    let mut s_inv: Vec<u32> = Vec::with_capacity(n_samples as usize);
    let mut s_lis: Vec<u32> = Vec::with_capacity(n_samples as usize);

    for t in 0..n_samples {
        let r = if do_all {
            t
        } else {
            rng.random_range(0..nperm)
        };
        let start = unrank(r, n, &fact);
        s_inv.push(inversions(&start, n) as u32);
        s_lis.push(lis(&start, n) as u32);

        let mut seq = start;
        let mut k = n;
        let mut d = dist[(r * (n as u64 + 1) + n as u64) as usize];
        let opt = d;
        let mut pb = 0u32;
        let mut comp = 0u32;
        while d > 0 {
            // collect ops that strictly decrease distance, pick one at random
            let mut choices: Vec<(u8, [u8; MAXN], usize)> = Vec::with_capacity(11);
            for op in 0..11u8 {
                if let Some((s2, k2)) = apply(&seq, n, k, op) {
                    let idx = (rank(&s2, n, &fact) * (n as u64 + 1) + k2 as u64) as usize;
                    if dist[idx] + 1 == d {
                        choices.push((op, s2, k2));
                    }
                }
            }
            let pick = rng.random_range(0..choices.len());
            let (op, s2, k2) = choices[pick];
            op_freq[op as usize] += 1;
            if op == 4 {
                pb += 1;
            }
            if matches!(op, 2 | 7 | 10) {
                comp += 1;
            }
            seq = s2;
            k = k2;
            d -= 1;
        }
        s_opt.push(opt as u32);
        s_pb.push(pb);
        s_comp.push(comp);
    }

    // ── Emit results ──
    let out = json!({
        "n": n,
        "nperm": nperm,
        "nstates": nstates,
        "diameter": diameter,
        "sphere": sphere[..=diameter].to_vec(),
        "perm_hist": perm_hist,
        "perm_stats": { "min": pmin, "median": median, "mean": mean, "max": pmax },
        "n_samples": n_samples,
        "sampled_all": do_all,
        "op_names": OP_NAMES,
        "op_freq": op_freq,
        "sample": {
            "opt": s_opt, "pb": s_pb, "comp": s_comp, "inv": s_inv, "lis": s_lis,
        },
    });
    let path = format!("bfs_n{n}.json");
    std::fs::write(&path, serde_json::to_string(&out).unwrap()).unwrap();
    eprintln!("wrote {path}");
}
