use push_swap::algo::{Algorithm, BFS_LIMIT};
use push_swap::optimizer;
use push_swap::stacks::StackPair;
use push_swap::{bench, bench_all, disorder, parse_values, process_and_rank};
use std::env;
use std::process;
use std::thread;

#[derive(Debug)]
struct Config {
    algo: Option<Algorithm>,
    bench: bool,
    no_opt: bool,
    values: Vec<i32>,
}

fn parse_args() -> Config {
    let mut algo: Option<Algorithm> = None;
    let mut bench = false;
    let mut no_opt = false;
    let mut value_args = Vec::new();

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--bench" => bench = true,
            "--no-opt" => no_opt = true,
            other if other.starts_with("--") => {
                let flag = &other[2..];
                match Algorithm::from_name(flag) {
                    Some(a) => {
                        if algo.is_some() {
                            eprintln!("Error: only one algorithm flag allowed");
                            process::exit(1);
                        }
                        algo = Some(a);
                    }
                    None => {
                        let mut names = vec![Algorithm::Bfs.name()];
                        names.extend(Algorithm::ALL.iter().map(|a| a.name()));
                        eprintln!(
                            "Error: Unknown flag '{other}'. Available: {}, bench, no-opt",
                            names.join(", ")
                        );
                        process::exit(1);
                    }
                }
            }
            _ => value_args.push(arg),
        }
    }

    let values = parse_values(&value_args).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    });

    Config {
        algo,
        bench,
        no_opt,
        values,
    }
}

fn main() {
    let config = parse_args();

    let ranked = process_and_rank(&config.values).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    });

    let mut stacks = StackPair::new(ranked.clone());

    if let Some(algo) = config.algo {
        if matches!(algo, Algorithm::Bfs) && ranked.len() > BFS_LIMIT {
            eprintln!("Error: --bfs only supports n <= {BFS_LIMIT}");
            process::exit(1);
        }
        algo.sort()(&mut stacks);
        let pre_opt = stacks.total_ops();
        if !config.no_opt {
            stacks.set_logs(optimizer::optimize(stacks.logs().to_vec()));
        }

        for op in stacks.logs() {
            println!("{op}");
        }

        if config.bench {
            bench(&stacks, disorder(&ranked), &algo.to_string(), pre_opt);
        }
    } else {
        let no_opt = config.no_opt;
        let mut algos = Algorithm::ALL.to_vec();
        if ranked.len() <= BFS_LIMIT {
            algos.push(Algorithm::Bfs);
        }
        let mut handles = Vec::new();
        // Forward racers: each algo run on the input directly.
        for algo in algos {
            let mut s = stacks.clone();
            handles.push(thread::spawn(move || {
                algo.sort()(&mut s);
                let pre_opt = s.total_ops();
                if !no_opt {
                    s.set_logs(optimizer::optimize(s.logs().to_vec()));
                }
                (s, algo.name().to_string(), pre_opt)
            }));
        }
        // Reverse twins (idea N2): solve the inverse permutation and
        // reverse-invert the op sequence. Free orthogonal racer; Bfs is already
        // optimal so it gets no twin.
        for algo in Algorithm::ALL {
            let algo = *algo;
            let ranked = ranked.clone();
            let name = format!("{}_rev", algo.name());
            handles.push(thread::spawn(move || {
                let mut s = push_swap::algo::reverse_solve(algo.sort(), &ranked);
                let pre_opt = s.total_ops();
                if !no_opt {
                    s.set_logs(optimizer::optimize(s.logs().to_vec()));
                }
                (s, name, pre_opt)
            }));
        }

        let mut results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        results.sort_by_key(|(s, _, _)| s.total_ops());
        let (best_stacks, _, _) = &results[0];

        for op in best_stacks.logs() {
            println!("{op}");
        }

        if config.bench {
            bench_all(&results, disorder(&ranked));
        }
    }
}
