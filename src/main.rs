use push_swap_rs::algo::Algorithm;
use push_swap_rs::{bench, disorder, parse_values, process_and_rank};
use push_swap_rs::stacks::{Log, StackPair};
use std::env;
use std::process;

#[derive(Debug)]
struct Config {
    algo: Option<Algorithm>,
    bench: bool,
    values: Vec<i32>,
}

fn parse_args() -> Config {
    let mut algo: Option<Algorithm> = None;
    let mut bench = false;
    let mut value_args = Vec::new();

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--bench" => bench = true,
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
                        eprintln!("Error: Unknown flag '{other}'. Available: {}, bench", names.join(", "));
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

    let algo = config.algo.unwrap_or({
        if n <= 5 {
            Algorithm::Selection
        } else if d < 0.2 {
            Algorithm::Insertion
        } else if d < 0.5 {
            Algorithm::KSort
        } else {
            Algorithm::Turk
        }
    });

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
