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
    logs: Vec<Operation>,
}

impl StackPair {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            a: VecDeque::with_capacity(capacity),
            b: VecDeque::with_capacity(capacity),
            logs: Vec::with_capacity(20 * capacity),
        }
    }

    pub fn rotate_a(&mut self) {
        if let Some(x) = self.a.pop_front() {
            self.a.push_back(x);
        }
        self.logs.push(Operation::Ra);
    }

    pub fn swap_a(&mut self) {
        if let Some(first) = self.a.pop_front() {
            if let Some(second) = self.a.pop_front() {
                self.a.push_front(first);
                self.a.push_front(second);
            } else {
                // Only had one, put it back
                self.a.push_front(first);
            }
        }
        self.logs.push(Operation::Sa);
    }
}
