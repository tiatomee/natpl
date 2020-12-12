use fraction::*;
use std::{collections::BTreeMap, fmt::Display};

use crate::{num::dec_in_scientific_notation, num::max_precision, syntax::Name};

#[derive(Debug, Clone)]
pub struct Value {
    pub kind: ValueKind,
    pub unit: Unit,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ValueKind {
    FunctionRef(Name),
    Number(BigDecimal),
}

impl Display for ValueKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueKind::FunctionRef(name) => f.write_fmt(format_args!("<function {}>", name)),
            ValueKind::Number(num) => {
                let (int, dec, exp, sign) = dec_in_scientific_notation(&num);

                if exp < -50 {
                    return f.write_str("≈0");
                }

                let exp_str = if exp == 0 {
                    "".to_string()
                } else {
                    format!("x10^{}", exp)
                };

                if (0..4).contains(&exp) {
                    if dec.len() < exp as usize {
                        f.write_fmt(format_args!(
                            "{:.prec$}",
                            num,
                            prec = num.get_precision().min(4)
                        ))
                    } else {
                        f.write_fmt(format_args!(
                            "{:.prec$}",
                            num,
                            prec = dec.len().min(4) - exp as usize
                        ))
                    }
                } else if (-3..0).contains(&exp) {
                    f.write_fmt(format_args!(
                        "{:.prec$}",
                        num,
                        prec = (dec.len() + (-exp) as usize).min(4)
                    ))
                } else if dec.is_empty() {
                    f.write_fmt(format_args!("{}{}{}", sign, int, exp_str))
                } else {
                    let dec = max_precision(&dec, 3);
                    f.write_fmt(format_args!("{}{}.{}{}", sign, int, dec, exp_str))
                }
            }
        }
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Unit {
    parts: BTreeMap<Name, BigDecimal>,
}

impl Unit {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_named(name: Name) -> Self {
        let mut this = Self::new();
        this.parts.insert(name, 1.into());
        this
    }

    pub fn singleton(&self) -> Option<&Name> {
        if self.parts.len() != 1 {
            None
        } else {
            let (name, p) = self.parts.iter().next()?;
            if *p != 1.into() {
                None
            } else {
                Some(name)
            }
        }
    }

    pub fn multiply(&self, other: &Unit) -> Unit {
        let mut parts = self.parts.clone();

        for (name, val) in &other.parts {
            let total = parts
                .entry(name.clone())
                .and_modify(|e| *e += val)
                .or_insert_with(|| val.clone())
                .clone();

            if total == 0.into() {
                parts.remove(name);
            }
        }

        Unit { parts }
    }

    pub fn divide(&self, other: &Unit) -> Unit {
        let mut parts = self.parts.clone();

        for (name, val) in &other.parts {
            let total = parts
                .entry(name.clone())
                .and_modify(|e| *e -= val)
                .or_insert_with(|| -val.clone())
                .clone();

            if total == 0.into() {
                parts.remove(name);
            }
        }

        Unit { parts }
    }

    pub fn pow(&self, n: &BigDecimal) -> Unit {
        let sign = n.sign();
        let n_add = n.abs();

        let parts = self
            .parts
            .iter()
            .filter_map(|(k, v)| {
                let v = v * &n_add;
                if v == 0.into() {
                    None
                } else if sign == Some(Sign::Minus) {
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

            if *pow != 1.into() {
                f.write_fmt(format_args!("^{}", *pow))?;
            }
        }

        Ok(())
    }
}
