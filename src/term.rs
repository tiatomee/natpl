use std::{collections::BTreeMap, fmt::Display};
use bigdecimal::BigDecimal;

use crate::syntax::Name;

#[derive(Debug, Clone)]
pub struct Value {
    pub kind: ValueKind,
    pub unit: Unit,
}

#[derive(Debug, Clone)]
pub enum ValueKind {
    FunctionRef(Name),
    Number(BigDecimal),
}

impl Display for ValueKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueKind::FunctionRef(name) => f.write_fmt(format_args!("<function {}>", name)),
            ValueKind::Number(num) => f.write_fmt(format_args!("{}", num)),
        }
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Unit {
    parts: BTreeMap<Name, isize>,
}

impl Unit {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_named(name: Name) -> Self {
        let mut this = Self::new();
        this.parts.insert(name, 1);
        this
    }

    pub fn multiply(&self, other: &Unit) -> Unit {
        let mut parts = self.parts.clone();

        for (name, val) in &other.parts {
            let total = *parts
                .entry(name.clone())
                .and_modify(|e| *e += *val)
                .or_insert(*val);

            if total == 0 {
                parts.remove(name);
            }
        }

        Unit { parts }
    }

    pub fn divide(&self, other: &Unit) -> Unit {
        let mut parts = self.parts.clone();

        for (name, val) in &other.parts {
            let total = *parts
                .entry(name.clone())
                .and_modify(|e| *e -= *val)
                .or_insert(-*val);

            if total == 0 {
                parts.remove(name);
            }
        }

        Unit { parts }
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, (name, pow)) in self.parts.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }

            f.write_str(name)?;

            if *pow != 1 {
                f.write_fmt(format_args!("^{}", *pow))?;
            }
        }

        Ok(())
    }
}
