use push_swap_rs::{parse_values, process_and_rank};
use push_swap_rs::stacks::{Operation, StackPair};
use std::io::{self, BufRead};
use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let values = parse_values(&args).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    });

    let ranked = process_and_rank(&values).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    });

    let mut stacks = StackPair::new(ranked);

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap_or_else(|e| {
            eprintln!("Error reading stdin: {e}");
            process::exit(1);
        });
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let op: Operation = trimmed.parse().unwrap_or_else(|e| {
            eprintln!("Error: {e}");
            process::exit(1);
        });
        stacks.execute(op);
    }

    if stacks.b().is_empty() && stacks.a().iter().is_sorted() {
        println!("OK");
    } else {
        println!("KO");
    }
}
