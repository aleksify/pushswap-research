use push_swap_rs::algo;
use push_swap_rs::process_and_rank;
use push_swap_rs::stacks::{Log, StackPair};
use std::env;
use std::process;
use std::thread;

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
    fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }

    fn algo_flag_set(&self) -> bool {
        self.simple || self.medium || self.complex || self.adaptive
    }
}

fn parse_args() -> Config {
    let mut config = Config::with_capacity(500);

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--simple" | "--medium" | "--complex" | "--adaptive" => {
                if config.algo_flag_set() {
                    eprintln!(
                        "Error: simple, medium, complex, or adaptive cannot be used together"
                    );
                    process::exit(1);
                }
                match arg.as_str() {
                    "--simple" => config.simple = true,
                    "--medium" => config.medium = true,
                    "--complex" => config.complex = true,
                    "--adaptive" => config.adaptive = true,
                    _ => unreachable!(),
                }
            }
            "--bench" => config.bench = true,
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
    config
}

fn main() {
    let config = parse_args();

    let ranked = process_and_rank(config.values).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    });

    let stacks = StackPair::new(ranked);

    let mut insert_stacks = stacks.clone();
    let mut chunk_stacks = stacks.clone();
    let mut turk_stacks = stacks;

    let insert_handle = thread::spawn(move || {
        algo::sort_insert(&mut insert_stacks);
        insert_stacks
    });
    let chunk_handle = thread::spawn(move || {
        algo::sort_chunk(&mut chunk_stacks);
        chunk_stacks
    });
    let turk_handle = thread::spawn(move || {
        algo::sort_turk(&mut turk_stacks);
        turk_stacks
    });

    let insert_result = insert_handle.join().unwrap();
    let chunk_result = chunk_handle.join().unwrap();
    let turk_result = turk_handle.join().unwrap();

    for log in turk_result.logs() {
        if let Log::Execute(op) = log {
            println!("{op}");
        }
    }

    eprintln!("insert: {:#?}", insert_result.op_count());
    eprintln!("chunk:  {:#?}", chunk_result.op_count());
    eprintln!("turk:   {:#?}", turk_result.op_count());
}
