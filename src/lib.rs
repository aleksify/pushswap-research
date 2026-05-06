use std::collections::HashSet;

pub fn process_and_rank(values: Vec<i32>) -> Result<Vec<usize>, String> {
    // Sort to rank
    // Unstable means that sort doesn't guarantee that equal values
    // stay in the same order. Since we have no duplicates, doesn't matter.
    // Under the hood, it uses Quicksort to sort.
    let mut sorted = values.clone().sort_unstable();

    // Check for duplicates
    // Since it's already sorted, we can just compare neighbors using windows
    for window in sorted.windows(2) {
        if window[0] == window[1] {
            return Err(format!("Duplicate value '{}' is not allowed.", window[0]));
        }
    }

    // into_iter consumes the `values` array
    let ranked_values: Vec<usize> = values
        .into_iter()
        .map(|val| {
            // binary_search returns Ok(index). 
            // We use unwrap() because we already guarantee every number exists in the sorted.
            sorted.binary_search(&val).unwrap() 
        })
        .collect();

    Ok(ranked_values)
}
