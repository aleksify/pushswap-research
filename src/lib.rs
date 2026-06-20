pub mod algo;
pub mod optimizer;
pub mod stacks;

/// Disorder score in range 0.0..=1.0.
/// 0.0 = sorted, 1.0 = fully reversed.
pub fn disorder(slice: &[usize]) -> f64 {
    let n = slice.len();
    if n <= 1 {
        return 0.0;
    }
    let total_pairs = (n * (n - 1) / 2) as f64;
    let inversions: usize = slice
        .iter()
        .enumerate()
        .map(|(i, &a)| slice[i + 1..].iter().filter(|&&b| a > b).count())
        .sum();
    inversions as f64 / total_pairs
}

pub fn bench_all(results: &[(stacks::StackPair, String, usize)], disorder: f64) {
    let max_name = results.iter().map(|(_, name, _)| name.len()).max().unwrap();
    let w = max_name + 3; // +1 prefix, +1 colon, +1 space
    let dislabel = format!("{:<w$}", "disorder:");
    eprintln!("[bench] {dislabel}{:.2}%", disorder * 100.0);
    for (i, (s, name, pre_opt)) in results.iter().enumerate() {
        let prefix = if i == 0 { "*" } else { "" };
        let label = format!("{prefix}{}:", name);
        eprintln!("[bench] {label:<w$}{}({pre_opt})", s.total_ops(),);
    }
}

pub fn bench(stacks: &stacks::StackPair, disorder: f64, strategy: &str, pre_opt: usize) {
    use stacks::Operation;

    let mut sa = 0u32;
    let mut sb = 0u32;
    let mut ss = 0u32;
    let mut pa = 0u32;
    let mut pb = 0u32;
    let mut ra = 0u32;
    let mut rb = 0u32;
    let mut rr = 0u32;
    let mut rra = 0u32;
    let mut rrb = 0u32;
    let mut rrr = 0u32;

    for op in stacks.logs() {
        match op {
            Operation::Sa => sa += 1,
            Operation::Sb => sb += 1,
            Operation::Ss => ss += 1,
            Operation::Pa => pa += 1,
            Operation::Pb => pb += 1,
            Operation::Ra => ra += 1,
            Operation::Rb => rb += 1,
            Operation::Rr => rr += 1,
            Operation::Rra => rra += 1,
            Operation::Rrb => rrb += 1,
            Operation::Rrr => rrr += 1,
        }
    }
    let total = sa + sb + ss + pa + pb + ra + rb + rr + rra + rrb + rrr;

    eprintln!("[bench] disorder:   {:.2}%", disorder * 100.0);
    eprintln!("[bench] strategy:   {strategy}");
    eprintln!("[bench] total_ops:  {total}({pre_opt})");
    eprintln!("[bench] sa: {sa}  sb: {sb}  ss: {ss}  pa: {pa}  pb: {pb}");
    eprintln!("[bench] ra: {ra}  rb: {rb}  rr: {rr}  rra: {rra}  rrb: {rrb}  rrr: {rrr}");
}

pub fn parse_values(args: &[String]) -> Result<Vec<i32>, String> {
    let mut values = Vec::new();
    for arg in args {
        for num_str in arg.split_whitespace() {
            values.push(
                num_str
                    .parse::<i32>()
                    .map_err(|_| format!("Expected an integer, found '{num_str}'"))?,
            );
        }
    }
    if values.is_empty() {
        return Err("No values provided".to_string());
    }
    Ok(values)
}

pub fn process_and_rank(values: &[i32]) -> Result<Vec<usize>, String> {
    // Sort to rank
    // Unstable means that sort doesn't guarantee that equal values
    // stay in the same order. Since we have no duplicates, doesn't matter.
    // Under the hood, it uses Quicksort to sort.
    let mut sorted = values.to_vec();
    sorted.sort_unstable();

    // Check for duplicates
    // Since it's already sorted, we can just compare neighbors using windows
    for window in sorted.windows(2) {
        if window[0] == window[1] {
            return Err(format!("Duplicate value '{}' is not allowed.", window[0]));
        }
    }

    let ranked_values: Vec<usize> = values
        .iter()
        .map(|val| {
            // binary_search returns Ok(index).
            // We use unwrap() because we already guarantee every number exists in the sorted.
            sorted.binary_search(val).unwrap()
        })
        .collect();

    Ok(ranked_values)
}
