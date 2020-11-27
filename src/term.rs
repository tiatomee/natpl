use std::fmt::Display;

use crate::syntax::{InfixOp, Name};

#[derive(Debug, Clone)]
pub struct Value {
    pub kind: ValueKind,
    pub unit: Term,
}

#[derive(Debug, Clone)]
pub enum ValueKind {
    FunctionRef(Name),
    Number(f64),
}

impl Display for ValueKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueKind::FunctionRef(name) => f.write_fmt(format_args!("<function {}>", name)),
            ValueKind::Number(num) => f.write_fmt(format_args!("{}", num)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Term {
    Value(ValueKind),
    UnitRef(Name),
    InfixOp {
        op: InfixOp,
        lhs: Box<Term>,
        rhs: Box<Term>,
    }
}

impl Term {
    pub fn normalised(&self) -> Term {
        self.clone()
    }

    pub fn normalise(&mut self) {
        *self = self.normalised();
    }
}

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::Value(v) => v.fmt(f),
            Term::UnitRef(u) => f.write_fmt(format_args!("{}", u)),
            Term::InfixOp { op, lhs, rhs } => {
                f.write_fmt(format_args!("({} {} {})", lhs, op, rhs))
            }
        }
    }
}