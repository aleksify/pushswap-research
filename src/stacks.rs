use std::collections::VecDeque;
use std::fmt;
use std::str::FromStr;

pub trait StackExt {
    fn min_pos(&self) -> usize;
    fn max_pos(&self) -> usize;
    /// Position of smallest element > val, or min position if none exists.
    fn min_above_pos(&self, val: usize) -> usize;
    /// Position of largest element < val, or max position if none exists.
    fn max_below_pos(&self, val: usize) -> usize;
}

impl StackExt for VecDeque<usize> {
    fn min_pos(&self) -> usize {
        self.iter()
            .enumerate()
            .min_by_key(|&(_, v)| v)
            .unwrap()
            .0
    }

    fn max_pos(&self) -> usize {
        self.iter()
            .enumerate()
            .max_by_key(|&(_, v)| v)
            .unwrap()
            .0
    }

    fn min_above_pos(&self, val: usize) -> usize {
        self.iter()
            .enumerate()
            .filter(|&(_, v)| *v > val)
            .min_by_key(|&(_, v)| v)
            .map(|(i, _)| i)
            .unwrap_or_else(|| self.min_pos())
    }

    fn max_below_pos(&self, val: usize) -> usize {
        self.iter()
            .enumerate()
            .filter(|&(_, v)| *v < val)
            .max_by_key(|&(_, v)| v)
            .map(|(i, _)| i)
            .unwrap_or_else(|| self.max_pos())
    }
}

#[derive(Debug, Clone, Copy)]
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

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Operation::Sa => write!(f, "sa"),
            Operation::Sb => write!(f, "sb"),
            Operation::Ss => write!(f, "ss"),
            Operation::Pa => write!(f, "pa"),
            Operation::Pb => write!(f, "pb"),
            Operation::Ra => write!(f, "ra"),
            Operation::Rb => write!(f, "rb"),
            Operation::Rr => write!(f, "rr"),
            Operation::Rra => write!(f, "rra"),
            Operation::Rrb => write!(f, "rrb"),
            Operation::Rrr => write!(f, "rrr"),
        }
    }
}

impl FromStr for Operation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sa" => Ok(Operation::Sa),
            "sb" => Ok(Operation::Sb),
            "ss" => Ok(Operation::Ss),
            "pa" => Ok(Operation::Pa),
            "pb" => Ok(Operation::Pb),
            "ra" => Ok(Operation::Ra),
            "rb" => Ok(Operation::Rb),
            "rr" => Ok(Operation::Rr),
            "rra" => Ok(Operation::Rra),
            "rrb" => Ok(Operation::Rrb),
            "rrr" => Ok(Operation::Rrr),
            _ => Err(format!("Unknown operation: '{s}'")),
        }
    }
}

// Ignore operations will be ignored when optimization is ON
// Later, optimizer will go over the Logs, and switch off useless ops
#[derive(Debug, Clone)]
pub enum Log {
    Execute(Operation),
    Ignore(Operation),
}

#[derive(Debug, Clone)]
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
            Operation::Ss => StackPair::both(&mut self.a, &mut self.b, StackPair::swap),
            Operation::Pa => StackPair::push(&mut self.a, &mut self.b),
            Operation::Pb => StackPair::push(&mut self.b, &mut self.a),
            Operation::Ra => StackPair::rotate(&mut self.a),
            Operation::Rb => StackPair::rotate(&mut self.b),
            Operation::Rr => StackPair::both(&mut self.a, &mut self.b, StackPair::rotate),
            Operation::Rra => StackPair::rev_rotate(&mut self.a),
            Operation::Rrb => StackPair::rev_rotate(&mut self.b),
            Operation::Rrr => StackPair::both(&mut self.a, &mut self.b, StackPair::rev_rotate),
        };
        let entry = if success {
            Log::Execute(op)
        } else {
            Log::Ignore(op)
        };
        self.logs.push(entry);
    }

    pub fn a(&self) -> &VecDeque<usize> {
        &self.a
    }

    pub fn b(&self) -> &VecDeque<usize> {
        &self.b
    }

    pub fn logs(&self) -> &[Log] {
        &self.logs
    }

    pub fn op_count(&self) -> usize {
        self.logs.len()
    }

    pub fn is_sorted(&self) -> bool {
        self.b.is_empty() && self.a.iter().is_sorted()
    }

    pub fn op_count_opt(&self) -> usize {
        self.logs
            .iter()
            .filter(|l| matches!(l, Log::Execute(_)))
            .count()
    }

    // We can't use || because it's lazy
    // meaning, if f(a) was true,
    // it won't execute f(b)
    fn both(
        a: &mut VecDeque<usize>,
        b: &mut VecDeque<usize>,
        f: fn(&mut VecDeque<usize>) -> bool,
    ) -> bool {
        f(a) | f(b)
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
        if stack.len() >= 2 {
            stack.rotate_left(1);
            true
        } else {
            false
        }
    }

    fn rev_rotate(stack: &mut VecDeque<usize>) -> bool {
        if stack.len() >= 2 {
            stack.rotate_right(1);
            true
        } else {
            false
        }
    }
}

pub trait RotateExt {
    /// Rotate A shortest direction to bring position to top.
    fn rotate_a_to_top(&mut self, pos: usize);
    /// Rotate B shortest direction to bring position to top.
    fn rotate_b_to_top(&mut self, pos: usize);
}

impl RotateExt for StackPair {
    fn rotate_a_to_top(&mut self, pos: usize) {
        let n = self.a().len();
        if pos <= n / 2 {
            for _ in 0..pos {
                self.execute(Operation::Ra);
            }
        } else {
            for _ in pos..n {
                self.execute(Operation::Rra);
            }
        }
    }

    fn rotate_b_to_top(&mut self, pos: usize) {
        let n = self.b().len();
        if pos <= n / 2 {
            for _ in 0..pos {
                self.execute(Operation::Rb);
            }
        } else {
            for _ in pos..n {
                self.execute(Operation::Rrb);
            }
        }
    }
}
