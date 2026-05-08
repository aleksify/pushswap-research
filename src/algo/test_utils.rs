#![cfg(test)]

use crate::stacks::StackPair;

pub fn permutations(n: usize) -> Vec<Vec<usize>> {
    let mut vals: Vec<usize> = (0..n).collect();
    let mut result = Vec::new();
    permute(&mut vals, 0, &mut result);
    result
}

fn permute(vals: &mut Vec<usize>, start: usize, result: &mut Vec<Vec<usize>>) {
    if start == vals.len() {
        result.push(vals.clone());
        return;
    }
    for i in start..vals.len() {
        vals.swap(start, i);
        permute(vals, start + 1, result);
        vals.swap(start, i);
    }
}

pub fn assert_sorts(input: &[usize], sort_fn: fn(&mut StackPair)) {
    let expected: Vec<usize> = (0..input.len()).collect();
    let mut stacks = StackPair::new(input.to_vec());
    sort_fn(&mut stacks);
    let result: Vec<usize> = stacks.a().iter().copied().collect();
    assert_eq!(result, expected, "failed for {:?}", input);
}

/// Test sort_fn on all permutations from min_n..=max_n.
pub fn assert_sorts_all(min_n: usize, max_n: usize, sort_fn: fn(&mut StackPair)) {
    for n in min_n..=max_n {
        for p in permutations(n) {
            assert_sorts(&p, sort_fn);
        }
    }
}

/// Test sort_fn on random shuffles of 0..n for each size, `iterations` times per size.
/// Uses seeded RNG for deterministic, reproducible results.
pub fn assert_sorts_random(sizes: &[usize], iterations: usize, sort_fn: fn(&mut StackPair)) {
    use rand::rngs::StdRng;
    use rand::seq::SliceRandom;
    use rand::SeedableRng;

    for &n in sizes {
        for seed in 0..iterations {
            let mut rng = StdRng::seed_from_u64(seed as u64);
            let mut vals: Vec<usize> = (0..n).collect();
            vals.shuffle(&mut rng);
            assert_sorts(&vals, sort_fn);
        }
    }
}
