use push_swap_rs::algo::Algorithm;
use push_swap_rs::{bench, disorder, parse_values, process_and_rank};
use push_swap_rs::stacks::{Log, StackPair};
use std::env;
use std::process;

#[derive(Debug, Default, Clone, Copy)]
enum AlgoFlag {
    Simple,
    Medium,
    Complex,
    #[default]
    Adaptive,
}

#[derive(Debug)]
struct Config {
    algo: AlgoFlag,
    bench: bool,
    values: Vec<i32>,
}

fn parse_args() -> Config {
    let mut algo = AlgoFlag::default();
    let mut algo_set = false;
    let mut bench = false;
    let mut value_args = Vec::new();

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--simple" | "--medium" | "--complex" | "--adaptive" => {
                if algo_set {
                    eprintln!(
                        "Error: simple, medium, complex, or adaptive cannot be used together"
                    );
                    process::exit(1);
                }
                algo_set = true;
                algo = match arg.as_str() {
                    "--simple" => AlgoFlag::Simple,
                    "--medium" => AlgoFlag::Medium,
                    "--complex" => AlgoFlag::Complex,
                    "--adaptive" => AlgoFlag::Adaptive,
                    _ => unreachable!(),
                };
            }
            "--bench" => bench = true,
            other => {
                if other.starts_with("--") {
                    eprintln!("Error: Unknown flag '{}'", other);
                    process::exit(1);
                }
                value_args.push(arg);
            }
        }
    }

    let values = parse_values(&value_args).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    });

    Config { algo, bench, values }
}

fn main() {
    let config = parse_args();

    let ranked = process_and_rank(&config.values).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    });

    let n = ranked.len();
    let mut stacks = StackPair::new(ranked.clone());

    let d = disorder(&ranked);

    let algo = if n <= 5 {
        Algorithm::Selection
    } else {
        match config.algo {
            AlgoFlag::Simple => Algorithm::Insertion,
            AlgoFlag::Medium => Algorithm::KSort,
            AlgoFlag::Complex => Algorithm::Turk,
            AlgoFlag::Adaptive => {
                if d < 0.2 {
                    Algorithm::Insertion
                } else if d < 0.5 {
                    Algorithm::KSort
                } else {
                    Algorithm::Turk
                }
            }
        }
    };

    algo.sort()(&mut stacks);

    for log in stacks.logs() {
        if let Log::Execute(op) = log {
            println!("{op}");
        }
    }

    if config.bench {
        bench(&stacks, d, &algo.to_string());
    }
}
