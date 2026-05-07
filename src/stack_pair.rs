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
    pub fn new(values: Vec<usize>) -> Self {
        let len = values.len();
        Self {
            a: VecDeque::from(values),
            b: VecDeque::with_capacity(len),
            logs: Vec::with_capacity(20 * len),
        }
    }

    fn swap(stack: &mut VecDeque<usize>) {
        if stack.len() >= 2 {
            stack.swap(0, 1);
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
