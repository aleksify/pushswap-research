mod stack_pair;

use push_swap_rs::process_and_rank;
use stack_pair::StackPair;
use std::env;
use std::process;

#[derive(Debug, Default)]
struct Config {
    simple: bool,
    medium: bool,
    complex: bool,
    adaptive: bool,
    bench: bool,
    values: Vec<i32>,
}

impl Config {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }
}

fn main() {
    let mut config = Config::with_capacity(500);

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--simple" => {
                if config.medium || config.complex || config.adaptive {
                    eprintln!(
                        "Error: simple, medium, complex, or adaptive cannot be used together"
                    );
                    process::exit(1);
                }
                config.simple = true;
            }
            "--medium" => {
                if config.simple || config.complex || config.adaptive {
                    eprintln!(
                        "Error: simple, medium, complex, or adaptive cannot be used together"
                    );
                    process::exit(1);
                }
                config.medium = true;
            }
            "--complex" => {
                if config.simple || config.medium || config.adaptive {
                    eprintln!(
                        "Error: simple, medium, complex, or adaptive cannot be used together"
                    );
                    process::exit(1);
                }
                config.complex = true;
            }
            "--adaptive" => {
                if config.simple || config.medium || config.complex {
                    eprintln!(
                        "Error: simple, medium, complex, or adaptive cannot be used together"
                    );
                    process::exit(1);
                }
                config.adaptive = true;
            }
            "--bench" => {
                config.bench = true;
            }
            other => {
                if other.starts_with("--") {
                    eprintln!("Error: Unknown flag '{}'", other);
                    process::exit(1);
                }
                // Here we handle naturally both "1 2 3" or 1 2 3
                // 1 is "1".   "1".split_whitespace() = "1"
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

    let ranked = process_and_rank(config.values).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    });
    let stacks = StackPair::new(ranked);

    println!("{:#?}", stacks);
}
