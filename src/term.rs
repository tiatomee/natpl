use bigdecimal::BigDecimal;
use std::{collections::BTreeMap, fmt::Display};

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
            ValueKind::Number(num) => {
                let (int, dec, exp) = crate::util::dec_in_scientific_notation(&num.normalized());

                if dec.len() <= 3 && (-3..3).contains(&exp) {
                    f.write_fmt(format_args!("{}", num))
                } else if dec.len() < 3 && (-3..0).contains(&exp) {
                    f.write_fmt(format_args!("{}", num))
                } else if dec.is_empty() {
                    f.write_fmt(format_args!("{}x10^{}", int, exp))
                } else {
                    let dec = crate::util::dec_with_max_precision(&dec, 3);
                    f.write_fmt(format_args!("{}.{}x10^{}", int, dec, exp))
                }
            }
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

    pub fn pow(&self, n: isize) -> Unit {
        let is_neg = n.is_negative();
        let n_add = n.abs();

        let parts = self
            .parts
            .iter()
            .filter_map(|(k, v)| {
                let v = *v * n_add;
                if v == 0 {
                    None
                } else if is_neg {
                    Some((k.clone(), -v))
                } else {
                    Some((k.clone(), v))
                }
            })
            .collect();

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
