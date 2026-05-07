use std::collections::VecDeque;

#[derive(Debug)]
pub enum Operation {
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

    pub fn execute(&mut self, op: Operation) {
        match op {
            Operation::Sa => StackPair::swap(&mut self.a),
            Operation::Sb => StackPair::swap(&mut self.b),
            Operation::Ss => {
                StackPair::swap(&mut self.a);
                StackPair::swap(&mut self.b);
            }
            Operation::Pa => StackPair::push(&mut self.a, &mut self.b),
            Operation::Pb => StackPair::push(&mut self.b, &mut self.a),
            Operation::Ra => StackPair::rotate(&mut self.a),
            Operation::Rb => StackPair::rotate(&mut self.b),
            Operation::Rr => {
                StackPair::rotate(&mut self.a);
                StackPair::rotate(&mut self.b);
            }
            Operation::Rra => StackPair::rev_rotate(&mut self.a),
            Operation::Rrb => StackPair::rev_rotate(&mut self.b),
            Operation::Rrr => {
                StackPair::rev_rotate(&mut self.a);
                StackPair::rev_rotate(&mut self.b);
            }

        }
        self.logs.push(op);
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
}
