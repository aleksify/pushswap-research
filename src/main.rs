use push_swap_rs::algo;
use push_swap_rs::{disorder, process_and_rank};
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
    algo_set: bool,
    bench: bool,
    values: Vec<i32>,
}

impl Config {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            algo: AlgoFlag::default(),
            algo_set: false,
            bench: false,
            values: Vec::with_capacity(capacity),
        }
    }
}

fn parse_args() -> Config {
    let mut config = Config::with_capacity(500);

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--simple" | "--medium" | "--complex" | "--adaptive" => {
                if config.algo_set {
                    eprintln!(
                        "Error: simple, medium, complex, or adaptive cannot be used together"
                    );
                    process::exit(1);
                }
                config.algo_set = true;
                config.algo = match arg.as_str() {
                    "--simple" => AlgoFlag::Simple,
                    "--medium" => AlgoFlag::Medium,
                    "--complex" => AlgoFlag::Complex,
                    "--adaptive" => AlgoFlag::Adaptive,
                    _ => unreachable!(),
                };
            }
            "--bench" => config.bench = true,
            other => {
                if other.starts_with("--") {
                    eprintln!("Error: Unknown flag '{}'", other);
                    process::exit(1);
                }
                for num_str in other.split_whitespace() {
                    match num_str.parse::<i32>() {
                        Ok(num) => config.values.push(num),
                        Err(_) => {
                            eprintln!("Error: Expected an integer, found '{}'", num_str);
                            process::exit(1);
                        }
                    }
                }
            }
        }
    }
    if config.values.is_empty() {
        eprintln!("Error: No values provided");
        process::exit(1);
    }
    config
}

fn main() {
    let config = parse_args();

    let ranked = process_and_rank(config.values).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    });

    let n = ranked.len();
    let mut stacks = StackPair::new(ranked.clone());

    let sort_fn: fn(&mut StackPair) = if n <= 5 {
        algo::sort_selection
    } else {
        match config.algo {
            AlgoFlag::Simple => algo::sort_insert,
            AlgoFlag::Medium => algo::sort_chunk,
            AlgoFlag::Complex => algo::sort_turk,
            AlgoFlag::Adaptive => {
                let d = disorder(&ranked);
                if d < 0.2 {
                    algo::sort_insert
                } else if d < 0.5 {
                    algo::sort_chunk
                } else {
                    algo::sort_turk
                }
            }
        }
    };

    sort_fn(&mut stacks);

    for log in stacks.logs() {
        if let Log::Execute(op) = log {
            println!("{op}");
        }
    }
}
