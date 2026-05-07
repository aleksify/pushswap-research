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

#[derive(Debug)]
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

    fn swap(stack: &mut VecDeque<usize>) {
        if let Some(first) = stack.pop_front() {
            if let Some(second) = stack.pop_front() {
                stack.push_front(first);
                stack.push_front(second);
            } else {
                // Only had one, put it back
                stack.push_front(first);
            }
        }
    }

    fn push(dst: &mut VecDeque<usize>, src: &mut VecDeque<usize>) {
        if let Some(val) = src.pop_front() {
            dst.push_front(val);
        }
    }

    fn rotate(stack: &mut VecDeque<usize>) {
        if let Some(val) = stack.pop_front() {
            stack.push_back(val);
        }
    }

    fn rev_rotate(stack: &mut VecDeque<usize>) {
        if let Some(val) = stack.pop_back() {
            stack.push_front(val);
        }
    }

    pub fn swap_a(&mut self) {
        StackPair::swap(&mut self.a);
        self.logs.push(Operation::Sa);
    }

    pub fn swap_b(&mut self) {
        StackPair::swap(&mut self.b);
        self.logs.push(Operation::Sb);
    }

    pub fn swap_both(&mut self) {
        StackPair::swap(&mut self.a);
        StackPair::swap(&mut self.b);
        self.logs.push(Operation::Ss);
    }

    pub fn push_a(&mut self) {
        StackPair::push(&mut self.a, &mut self.b);
        self.logs.push(Operation::Pa);
    }

    pub fn push_b(&mut self) {
        StackPair::push(&mut self.b, &mut self.a);
        self.logs.push(Operation::Pb);
    }

    pub fn rotate_a(&mut self) {
        StackPair::rotate(&mut self.a);
        self.logs.push(Operation::Ra);
    }
    pub fn rotate_b(&mut self) {
        StackPair::rotate(&mut self.b);
        self.logs.push(Operation::Rb);
    }

    pub fn rotate_both(&mut self) {
        StackPair::rotate(&mut self.a);
        StackPair::rotate(&mut self.b);
        self.logs.push(Operation::Rr);
    }

    pub fn rev_rotate_a(&mut self) {
        StackPair::rev_rotate(&mut self.a);
        self.logs.push(Operation::Rra);
    }
    pub fn rev_rotate_b(&mut self) {
        StackPair::rev_rotate(&mut self.b);
        self.logs.push(Operation::Rrb);
    }

    pub fn rev_rotate_both(&mut self) {
        StackPair::rev_rotate(&mut self.a);
        StackPair::rev_rotate(&mut self.b);
        self.logs.push(Operation::Rrr);
    }
}
