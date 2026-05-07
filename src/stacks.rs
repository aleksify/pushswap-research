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

// Ignore operations will be ignored when optimization is ON
// Later, optimizer will go over the Logs, and switch off useless ops
#[derive(Debug)]
pub enum Log {
    Execute(Operation),
    Ignore(Operation),
}

#[derive(Debug)]
pub struct StackPair {
    a: VecDeque<usize>,
    b: VecDeque<usize>,
    logs: Vec<Log>,
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

    // Idea here is that if operation wasn't successful,
    // it means it would do nothing.
    // So, in optimization mode, we just gonna ignore it.
    // In case of double ops, we do OR,
    // because if at least 1 op was needed,
    // then we keep it the way it is, since its cost is 1 op anyway
    pub fn execute(&mut self, op: Operation) {
        let success = match op {
            Operation::Sa => StackPair::swap(&mut self.a),
            Operation::Sb => StackPair::swap(&mut self.b),
            Operation::Ss => {
                let a = StackPair::swap(&mut self.a);
                let b = StackPair::swap(&mut self.b);
                a || b
            }
            Operation::Pa => StackPair::push(&mut self.a, &mut self.b),
            Operation::Pb => StackPair::push(&mut self.b, &mut self.a),
            Operation::Ra => StackPair::rotate(&mut self.a),
            Operation::Rb => StackPair::rotate(&mut self.b),
            Operation::Rr => {
                let a = StackPair::rotate(&mut self.a);
                let b = StackPair::rotate(&mut self.b);
                a || b
            }
            Operation::Rra => StackPair::rev_rotate(&mut self.a),
            Operation::Rrb => StackPair::rev_rotate(&mut self.b),
            Operation::Rrr => {
                let a = StackPair::rev_rotate(&mut self.a);
                let b = StackPair::rev_rotate(&mut self.b);
                a || b
            }
        };
        let entry = if success {
            Log::Execute(op)
        } else {
            Log::Ignore(op)
        };
        self.logs.push(entry);
    }

    fn swap(stack: &mut VecDeque<usize>) -> bool {
        if stack.len() >= 2 {
            stack.swap(0, 1);
            true
        } else {
            false
        }
    }

    fn push(dst: &mut VecDeque<usize>, src: &mut VecDeque<usize>) -> bool {
        if let Some(val) = src.pop_front() {
            dst.push_front(val);
            true
        } else {
            false
        }
    }

    fn rotate(stack: &mut VecDeque<usize>) -> bool {
        if let Some(val) = stack.pop_front() {
            stack.push_back(val);
            true
        } else {
            false
        }
    }

    fn rev_rotate(stack: &mut VecDeque<usize>) -> bool {
        if let Some(val) = stack.pop_back() {
            stack.push_front(val);
            true
        } else {
            false
        }
    }
}
