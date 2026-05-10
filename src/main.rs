use push_swap_rs::algo::Algorithm;
use push_swap_rs::optimizer;
use push_swap_rs::{bench, bench_all, disorder, parse_values, process_and_rank};
use push_swap_rs::stacks::{Log, StackPair};
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
                        let names: Vec<_> = Algorithm::ALL.iter().map(|a| a.name()).collect();
                        eprintln!("Error: Unknown flag '{other}'. Available: {}, bench, no-opt", names.join(", "));
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

    Config { algo, bench, no_opt, values }
}

fn main() {
    let config = parse_args();

    let ranked = process_and_rank(&config.values).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    });

    let mut stacks = StackPair::new(ranked.clone());

    if let Some(algo) = config.algo {
        algo.sort()(&mut stacks);
        if !config.no_opt {
            stacks.set_logs(optimizer::optimize(stacks.logs().to_vec()));
        }

        for log in stacks.logs() {
            if let Log::Execute(op) = log {
                println!("{op}");
            }
        }

        if config.bench {
            bench(&stacks, disorder(&ranked), &algo.to_string());
        }
    } else {
        let no_opt = config.no_opt;
        let handles: Vec<_> = Algorithm::ALL
            .iter()
            .map(|&algo| {
                let mut s = stacks.clone();
                thread::spawn(move || {
                    algo.sort()(&mut s);
                    if !no_opt {
                        s.set_logs(optimizer::optimize(s.logs().to_vec()));
                    }
                    (s, algo)
                })
            })
            .collect();

        let mut results: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();

        results.sort_by_key(|(s, _)| s.total_ops_opt());
        let (best_stacks, _) = &results[0];

        for log in best_stacks.logs() {
            if let Log::Execute(op) = log {
                println!("{op}");
            }
        }

        if config.bench {
            bench_all(&results, disorder(&ranked));
        }
    }
}
