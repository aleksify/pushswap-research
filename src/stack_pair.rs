use std::collections::VecDeque;

#[derive(Debug)]
enum Operation {
    Sa,
    Sb,
    Ss,
    Pa,
    Pb,
    Ra,
    Rb,
    Rr,
    Rra,
    Rrb,
    Rrr,
}

#[derive(Debug, Default)]
pub struct StackPair {
    a: VecDeque<usize>,
    b: VecDeque<usize>,
    ops: Vec<Operation>,
}

impl StackPair {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            a: VecDeque::with_capacity(capacity),
            b: VecDeque::with_capacity(capacity),
            ops: Vec::with_capacity(20 * capacity)
        }
    }

    pub fn rotate_a(&mut self) {
        let val = self.a.pop_front();
        match val {
            Some(actual_value) => self.a.push_back(actual_value),
            None => (),
        }
        self.ops.push(Operation::Ra);
    }
}
