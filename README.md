## Introduction

This project started as a School 42 assignment - write a program that sorts a stack of integers using a limited set of 11 operations, in as few moves as possible.

I wanted to push further. The journey went roughly like this:

1. **Multiple algorithms in parallel.** The solver runs different sorting algorithms concurrently on every input and picks whichever produces the shortest output. Different algorithms win on different input distributions.
2. **A peephole optimizer.** Some algorithms emit sequences with local redundancies - `ra` followed by `rra` cancels out, `ra` followed by `rb` can collapse to `rr`, etc. I wrote a peephole optimizer that post-processes the output, rewriting these patterns away. The first version used a handful of hand-written rules.
3. **A superoptimizer to generate the rules.** Hand-writing rewrite rules is tedious and incomplete — you'll always miss patterns. So I built a superoptimizer: an exhaustive BFS search over the state space of stack configurations that discovers every reducible operation sequence up to a given depth. The optimizer's rule table is generated at build time from this search.
4. **Hit the scaling wall.** Past a certain depth, the rule count and binary size explode while the actual gains diminish. This led to thinking about algorithm-specific optimization rather than universal rules — see [Current Issues](#current-issues).

Table of Contents
=================

* [How to run](#how-to-run)
* [The Game](#the-game)
* [Optimizer](#optimizer)
* [Superoptimizer](#superoptimizer)
* [Current Issues](#current-issues)
* [More Thoughts](#more-thoughts)
* [How to build](#how-to-build)

## How to run

You can build it locally (instructions at the bottom), or if you don't have Rust & cargo installed, you can download a binary from the Releases page. They were generated using GitHub Actions for which there's a log, so you can check that it wasn't tampered with.

Use this to download the Linux binary and chmod it:
```
curl -L -o push_swap https://github.com/aleksify/pushswap-optimizer/releases/download/v0.4/push_swap && chmod +x push_swap
```

The default binary is a generic linux binary that should work on any distro, since it's statically linked with musl. The `_mac` binary is built for Apple Silicon Macs, but in order to run that binary, you'd need to run some commands since Apple by default forbids you to run unsigned binaries, so it's easier to build locally to be honest. But you can use these commands to run that binary: `xattr -cr push_swap_mac` to remove it from quarantine, and `codesign --force --deep -s - push_swap_mac` to sign it yourself.

## The Game

push_swap is a School 42 project. The challenge: given a stack A of integers, sort it in ascending order using only a limited set of operations, and do it in as few moves as possible.

The rules:
- You have two stacks, A and B. A starts with all the input values; B starts empty.
- You can only manipulate stacks through 11 operations: swaps (`sa`, `sb`, `ss`), pushes (`pa`, `pb`), rotations (`ra`, `rb`, `rr`), and reverse rotations (`rra`, `rrb`, `rrr`).
- The goal is to get all values sorted in A using the fewest operations.

While these are called "stacks," the rotation operations (moving top to bottom or bottom to top) mean they actually behave more like deques (double-ended queues).

Composite operations (`ss`, `rr`, `rrr`) apply to both stacks simultaneously for the cost of a single move — e.g., `ss` does `sa` + `sb` in one operation, so merging two independent single-stack ops into a composite saves a move.

By default, the `push_swap` binary runs all available sorting algorithms in parallel (selection, insertion, k_chunk, turk) and picks whichever produces the shortest solution. You can select a specific algorithm with `--turk`, `--selection`, `--insertion`, or `--k_chunk`. Use `--bench` for benchmark output comparing operation counts, and `--no-opt` to disable the optimizer.

```
./push_swap 3 1 2                   # sort, pick best algo
./push_swap --turk 3 1 2            # use turk algorithm only
./push_swap --bench 3 1 2           # benchmark all algos
./push_swap --bench --turk 3 1 2    # benchmark turk only
```

## Optimizer

The optimizer (`src/optimizer.rs`) is a universal peephole optimizer that post-processes the operation sequence produced by any sorting algorithm, rewriting it to use fewer moves. It repeatedly applies passes until no further reductions are found.

It operates in two passes:

1. **Normalization pass**: Between barrier operations (`pa`, `pb`, `ss`, `rr`, `rrr`), operations on stack A and stack B are independent and can be freely reordered. This pass groups A-ops and B-ops within each barrier-free block, bringing same-stack operations adjacent to each other. This exposes cancellations and merges that wouldn't be visible in the original interleaved order. Both A-first and B-first orderings are tried, and the shorter result is kept.

2. **Peephole pass**: Scans with variable-width windows (longest first, greedy) and applies rewrite rules from a lookup table. Rules come in two flavors:
   - **Annihilators**: sequences that cancel to nothing (e.g., `ra rra` -> empty).
   - **Reductions**: sequences replaceable by shorter equivalents (e.g., `ra rb` -> `rr`, or `ra pb rra` -> `sa pb`).

   On a match, the window steps back to catch cascading reductions.

The rewrite rules themselves are generated by the superoptimizer (see below) and embedded at compile time from `superopt_cache.json`.

## Superoptimizer

A [superoptimizer](https://en.wikipedia.org/wiki/Superoptimization) is a technique originally from compiler research: instead of applying hand-written rewrite rules, it exhaustively searches the space of all possible instruction sequences to find provably optimal replacements. Traditional compilers use superoptimization to discover peephole rules for instruction selection and scheduling.

Our superoptimizer (`src/bin/superopt.rs`) generates the rewrite rule table used by the optimizer. It works via **BFS graph search** over the state space of stack configurations:

1. Starting from a canonical two-stack state, it explores all possible sequences of the 11 operations, level by level (depth 1, depth 2, ..., up to depth N).
2. An **oracle** (hash map from stack state to shortest known operation sequence) tracks the first time each state is reached.
3. When a longer sequence reaches an already-known state, the difference is a rewrite rule: the longer sequence can be replaced by the shorter one (a **reduction**), or eliminated entirely if the state is the starting state (an **annihilator**).
4. A **reducible suffix set** prunes the search: any sequence ending in a known-reducible pattern is skipped, since it could never be optimal.
5. All discovered rules are **fuzz-verified** against 1,000 random stack configurations to guard against bugs.

The canonical state uses stacks of size `2N+1` (with a floor of 3). This size is the minimum needed to guarantee that all 11 operations produce distinct state transitions — with fewer elements, some operations become degenerate (e.g., swap is identical to rotate on a 2-element stack), and the discovered rules might not generalize to larger stacks.

Results are cached in `superopt_cache.json`, and the search can resume from the last explored depth. The cache is embedded into the optimizer binary at compile time via `include_str!`.

## Current Issues

The superoptimizer's exhaustive approach hits three scaling walls as N grows:

- **Memory**: The BFS oracle grows exponentially with depth. Bit-packing operation sequences and states could help, but only delays the inevitable — beyond N=10 or so, the working set would need to be backed by an on-disk database rather than held in RAM.
- **Binary size**: All discovered rules are embedded into the final binary via `include_str!`. At N=8, the binary approaches 400 MB; at N=9, it's close to 2 GB.
- **Diminishing returns**: The number of rules explodes with depth, but most of them never fire in practice. Higher-depth rules match increasingly rare patterns that most algorithms rarely produce.

In short: RAM usage, binary size, and rule count all blow up, while the actual optimization gains diminish rapidly.

**Where to go from here?** Rather than discovering rules universally across all possible stack states, a more promising direction would be algorithm-specific optimization — generating rules only for patterns that a given algorithm actually produces. Two approaches:

1. **Corpus-driven search**: Fuzz each algorithm with thousands of random inputs, collect the operation sequences, and run the superoptimizer only over that corpus. This dramatically shrinks the search space by focusing on states the algorithm actually visits.
2. **Post-hoc pruning**: Run the full superoptimizer, then fuzz-test to identify which rules were actually applied, and discard the rest.

That said, even these approaches face diminishing returns. Heuristic algorithms like Turk are already intelligent enough in their move selection that there's less room for a post-hoc optimizer to improve on.

## More Thoughts

So far, every approach in this project is either a hand-designed sorting algorithm or a post-hoc optimizer on top of one. But there's a whole other class of techniques worth considering — ones that *search* for solutions directly rather than constructing them procedurally:

- **Genetic algorithms**: Evolve a population of operation sequences. Crossover splices sequences together, mutation flips or inserts operations, and selection keeps the fittest. Over generations, the population converges toward shorter solutions.
- **Reinforcement learning**: Treat sorting as a game. State = current stacks, actions = the 11 operations, reward = sorting completion (minus a per-op cost). Train a policy network (e.g., PPO, AlphaZero-style MCTS) to pick moves. The network learns to navigate the state space without explicit rules.
- **Heuristic lookahead**: Beam search or Monte Carlo Tree Search with a bounded horizon. At each step, expand candidate move sequences up to depth K, score the resulting states, and commit to the best path's first move.

**The common obstacle: fitness.** All three approaches need a way to score how "close to sorted" a given (A, B) state is. The naive choice — count inversions in A, or measure displacement from sorted order — fundamentally doesn't work for push_swap.

The reason: **sorting often requires first *adding* disorder.** To sort a stack with push_swap, you typically push values to B, rearrange them there, and push them back in the right order. During this process, A looks more disordered than when you started — values have been removed, rotations have shuffled what remains. A naive fitness function would punish exactly the moves a good algorithm needs to make. It's like Tower of Hanoi: progress requires intermediate states that look like regressions.

A workable fitness function would need to model the *structure* of a valid sort, not just the appearance of order. Some ideas:

- **Value-aware decomposition**: Allow B to be "sorted descending" and A to be "sorted ascending" — measure inversions within each, but treat the split itself as free. Penalize only when values are in the wrong stack *and* in the wrong relative order.
- **Distance to a reachable canonical**: Precompute (via superoptimizer-style BFS) the shortest path from any small state to the sorted goal, and use that as a learned distance metric for larger states.
- **Learned fitness**: Let the RL agent learn its own value function from terminal rewards alone (AlphaZero-style). This avoids hand-designing fitness but pays the cost of a much harder training problem.

## How to build

The project uses a Makefile for common tasks:

```sh
# 1. Generate optimizer rules with the superoptimizer.
#    Recommended: N=5. Don't go above 8 (OOM).
make superopt N=5

# 2. Build the release binary (uses the generated rules).
make release

# 3. Run it.
./push_swap 4 2 7 1 3
```

Other Makefile targets:

| Target | Description |
|--------|-------------|
| `make build` | Debug build, copies binaries to project root |
| `make release` | Optimized release build |
| `make test` | Run test suite |
| `make fmt` | Format code with rustfmt |
| `make lint` | Run clippy |
| `make superopt N=5` | Run superoptimizer to depth N |
| `make clean-cache` | Reset `superopt_cache.json` to empty |
| `make clean` | Remove built binaries from root |
| `make fclean` | Full clean (including `target/`) |
