use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{
    syntax::{Expression, HasFC, InfixOp, Item, LineItem, Name, PrefixOp, SiPrefix, FC},
    value_unit::{Unit, Value, ValueKind},
};

use fraction::{BigDecimal, ToPrimitive};
use thiserror::Error;

#[derive(Default)]
pub struct Runtime {
    units: HashSet<Name>,
    unit_aliases: HashMap<Name, Value>,
    variables: HashMap<Name, Value>,
    functions: HashMap<Name, (Vec<Name>, Expression)>,
}

impl Runtime {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn eval_line_item(&mut self, item: LineItem) -> Result<EvalResult, ItemError> {
        match self.line_item_to_item(item) {
            Some(item) => self.eval_item(item),
            None => Ok(EvalResult::Empty),
        }
    }

    pub fn find_units<'a, 'b: 'a>(&'b self, unit: &'a Unit) -> HashMap<&'a Name, ValueKind> {
        self.unit_aliases
            .iter()
            .filter_map(|(name, val)| {
                if &val.unit == unit {
                    Some((name, val.kind.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn find_variables_with_unit<'a, 'b: 'a>(
        &'b self,
        unit: &'a Unit,
    ) -> HashMap<&'a Name, ValueKind> {
        self.variables
            .iter()
            .filter_map(|(name, val)| {
                if &val.unit == unit {
                    Some((name, val.kind.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn line_item_to_item(&mut self, item: LineItem) -> Option<Item> {
        let it = match item {
            LineItem::Empty => return None,
            LineItem::VarSearch(fc) => Item::VarSearch(fc),
            LineItem::UnitSearch(expr) => Item::UnitSearch(expr),
            LineItem::UnitDeclaration(fc, name) => Item::UnitDeclaration(fc, name),
            LineItem::UnitAlias(fc, name, expr) => Item::UnitAlias(fc, name, expr),
            LineItem::MaybeDeclarationOrEqualityExpression(decl) => {
                let name = decl.declaration_name();

                let is_defined = self.units.contains(name)
                    || self.variables.contains_key(name)
                    || self.functions.contains_key(name);

                if is_defined {
                    decl.into_expression()
                } else {
                    decl.into_declaration()
                }
            }
            LineItem::PrintedExpression(fc, expr) => Item::PrintedExpression(fc, expr),
            LineItem::SilentExpression(expr) => Item::SilentExpression(expr),
        };
        Some(it)
    }

    fn eval_item(&mut self, item: Item) -> Result<EvalResult, ItemError> {
        use std::collections::hash_map::Entry;

        match item {
            Item::VarSearch(_) => Ok(EvalResult::VariableSearchResult {
                unit_aliases: self
                    .unit_aliases
                    .iter()
                    .map(|(n, v)| (n.clone(), v.clone()))
                    .collect(),
                variables: self
                    .variables
                    .iter()
                    .map(|(n, v)| (n.clone(), v.clone()))
                    .collect(),
                functions: Default::default(), // TODO
            }),
            Item::UnitSearch(expr) => {
                let val = self.eval_expr(&expr)?;
                Ok(EvalResult::UnitSearchResult {
                    unit_aliases: self
                        .find_units(&val.unit)
                        .into_iter()
                        .map(|(n, v)| (n.clone(), v))
                        .collect(),
                    variables: self
                        .find_variables_with_unit(&val.unit)
                        .into_iter()
                        .map(|(n, v)| (n.clone(), v))
                        .collect(),
                    unit: val.unit,
                })
            }
            Item::UnitDeclaration(_, name) => {
                let name = name.name();

                let is_unique = self.units.insert(name.clone());
                if !is_unique {
                    Err(ItemError::UnitRedeclared(name))
                } else {
                    Ok(EvalResult::Empty)
                }
            }
            Item::UnitAlias(_, name, expr) => {
                let value = self.eval_expr(&expr)?;

                let name = name.name();
                match self.unit_aliases.entry(name.clone()) {
                    Entry::Occupied(_) => Err(ItemError::UnitRedeclared(name)),
                    Entry::Vacant(entry) => {
                        let val = entry.insert(value).clone();
                        Ok(EvalResult::Value(val))
                    }
                }
            }
            Item::VariableDeclaration { fc: _, name, rhs } => {
                let value = self.eval_expr(&rhs)?;

                let name = name.name();
                match self.variables.entry(name.clone()) {
                    Entry::Occupied(_) => Err(ItemError::VariableRedefined(name)),
                    Entry::Vacant(entry) => {
                        let val = entry.insert(value).clone();
                        Ok(EvalResult::Value(val))
                    }
                }
            }
            Item::FunctionDeclaration {
                fc: _,
                name,
                arg_names,
                rhs,
            } => {
                let name = name.name();

                match self.functions.entry(name.clone()) {
                    Entry::Occupied(_) => Err(ItemError::FunctionRedefined(name)),
                    Entry::Vacant(entry) => {
                        entry.insert((arg_names.into_iter().map(|n| n.name()).collect(), rhs));
                        Ok(EvalResult::Empty)
                    }
                }
            }
            Item::PrintedExpression(_, e) => {
                let val = self.eval_expr(&e)?;
                Ok(EvalResult::PrintValue(e, val))
            }
            Item::SilentExpression(e) => {
                let val = self.eval_expr(&e)?;
                Ok(EvalResult::Value(val))
            }
        }
    }

    fn eval_expr(&self, expr: &Expression) -> Result<Value, EvalError> {
        match expr {
            Expression::IntegerLit { fc: _, val } => Ok(Value {
                kind: ValueKind::Number(val.clone()),
                unit: Unit::new(),
            }),
            Expression::FloatLit { fc: _, val } => Ok(Value {
                kind: ValueKind::Number(val.clone()),
                unit: Unit::new(),
            }),
            Expression::MaybeUnitPrefix {
                fc,
                name,
                full_name,
                prefix,
            } => {
                if let Some(val) = self.lookup(full_name) {
                    return Ok(val);
                }

                if let Some(val) = self.lookup(name) {
                    apply_prefix(*fc, *prefix, val)
                } else {
                    Err(EvalError::UndefinedName(*fc, full_name.clone()))
                }
            }
            Expression::Variable(id) => self
                .lookup(id.name_ref())
                .ok_or_else(|| EvalError::UndefinedName(id.fc(), id.name_ref().clone())),
            Expression::Call {
                fc: _,
                base,
                args,
            } => {
                match &**base {
                    Expression::Variable(s) if s.name_ref() == "sqrt" => {
                        assert_eq!(args.len(), 1);
                        let arg = &args[0];
                        let val = self.eval_expr(arg)?;

                        let kind = match &val.kind {
                            ValueKind::Number(n) => ValueKind::Number(n.to_f64().unwrap().sqrt().into()),
                            _ => todo!(),
                        };

                        let unit = val.unit.pow(&(BigDecimal::from(1) / BigDecimal::from(2)));

                        Ok(Value {
                            kind,
                            unit,
                        })
                    }
                    _ => todo!()
                }
            }
            Expression::PrefixOp { fc, op, expr } => {
                let mut val = self.eval_expr(expr)?;
                match op {
                    crate::syntax::PrefixOp::Pos => match &mut val.kind {
                        ValueKind::Number(_) => Ok(val),
                        ValueKind::FunctionRef(_) => {
                            Err(EvalError::InvalidPrefixOperator(*fc, *op, val))
                        }
                    },
                    crate::syntax::PrefixOp::Neg => match &mut val.kind {
                        ValueKind::Number(num) => {
                            *num = -&*num;
                            Ok(val)
                        }
                        ValueKind::FunctionRef(_) => {
                            Err(EvalError::InvalidPrefixOperator(*fc, *op, val))
                        }
                    },
                }
            }
            Expression::InfixOp { fc, op, lhs, rhs } => self.eval_infix_op(*fc, *op, lhs, rhs),
            Expression::UnitOf(_, expr) => {
                let val = self.eval_expr(expr)?;
                Ok(Value {
                    kind: ValueKind::Number(BigDecimal::from(1)),
                    unit: val.unit,
                })
            }
            Expression::Parenthesised(_, expr) => self.eval_expr(expr),
        }
    }

    // Name might be changed in future
    #[allow(clippy::ptr_arg)]
    pub fn lookup(&self, name: &Name) -> Option<Value> {
        if let Some(val) = self.variables.get(name) {
            Some(val.clone())
        } else if self.units.contains(name) {
            Some(Value {
                kind: ValueKind::Number(BigDecimal::from(1)),
                unit: Unit::new_named(name.clone()),
            })
        } else if let Some(val) = self.unit_aliases.get(name) {
            Some(val.clone())
        } else if self.functions.contains_key(name) {
            Some(Value {
                kind: ValueKind::FunctionRef(name.clone()),
                unit: Unit::new(),
            })
        } else {
            None
        }
    }

    pub fn eval_infix_op(
        &self,
        fc: FC,
        op: InfixOp,
        lhs: &Expression,
        rhs: &Expression,
    ) -> Result<Value, EvalError> {
        let lhs = self.eval_expr(lhs)?;
        let rhs = self.eval_expr(rhs)?;

        let unit = infix_unit(fc, op, &lhs, &rhs)?;

        match (op, &lhs.kind, &rhs.kind) {
            (InfixOp::Add, ValueKind::Number(a), ValueKind::Number(b)) => Ok(Value {
                kind: ValueKind::Number(a + b),
                unit,
            }),
            (InfixOp::Sub, ValueKind::Number(a), ValueKind::Number(b)) => Ok(Value {
                kind: ValueKind::Number(a - b),
                unit,
            }),
            (InfixOp::Mul, ValueKind::Number(a), ValueKind::Number(b)) => Ok(Value {
                kind: ValueKind::Number(a * b),
                unit,
            }),
            (InfixOp::Div, ValueKind::Number(a), ValueKind::Number(b)) => Ok(Value {
                kind: ValueKind::Number(a / b),
                unit,
            }),
            (InfixOp::Mod, ValueKind::Number(a), ValueKind::Number(b)) => Ok(Value {
                kind: ValueKind::Number(a % b),
                unit,
            }),
            (InfixOp::Pow, ValueKind::Number(a), ValueKind::Number(b)) => {
                let pow: isize = if b.fract() == 0.into() {
                    b.trunc().to_isize().unwrap()
                } else {
                    unimplemented!("Floating point power is not implemented")
                };

                let mut res = BigDecimal::from(1);

                for _ in 0..pow.abs() {
                    res *= a;
                }

                if pow.is_negative() {
                    res = res.recip();
                }

                Ok(Value {
                    kind: ValueKind::Number(res),
                    unit,
                })
            }
            (InfixOp::Eq, ValueKind::Number(a), ValueKind::Number(b)) => {
                if a == b {
                    Ok(lhs.clone())
                } else {
                    Err(EvalError::EqualtyError(fc, op, lhs.clone(), rhs.clone()))
                }
            }
            (InfixOp::Neq, ValueKind::Number(a), ValueKind::Number(b)) => {
                if a != b {
                    Ok(Value {
                        kind: ValueKind::Number(1.into()),
                        unit,
                    })
                } else {
                    Err(EvalError::EqualtyError(fc, op, lhs.clone(), rhs.clone()))
                }
            }
            (InfixOp::Gt, ValueKind::Number(a), ValueKind::Number(b)) => {
                if a > b {
                    Ok(Value {
                        kind: ValueKind::Number(1.into()),
                        unit,
                    })
                } else {
                    Err(EvalError::EqualtyError(fc, op, lhs.clone(), rhs.clone()))
                }
            }
            (op, _, _) => Err(EvalError::InvalidInfixOperator(fc, op, lhs, rhs)),
        }
    }
}

fn apply_prefix(fc: FC, prefix: SiPrefix, mut val: Value) -> Result<Value, EvalError> {
    let kind = match &val.kind {
        ValueKind::Number(n) => ValueKind::Number(&prefix.value() * n),
        ValueKind::FunctionRef(_) => return Err(EvalError::InvalidSiPrefix(fc, prefix, val)),
    };
    val.kind = kind;
    Ok(val)
}

fn infix_unit(fc: FC, op: InfixOp, lhs: &Value, rhs: &Value) -> Result<Unit, UnitError> {
    match op {
        InfixOp::Add | InfixOp::Sub | InfixOp::Mod => {
            if lhs.unit == rhs.unit {
                Ok(lhs.unit.clone())
            } else {
                Err(UnitError::IncompatibleUnits(
                    fc,
                    op,
                    lhs.unit.clone(),
                    rhs.unit.clone(),
                ))
            }
        }

        InfixOp::Mul => Ok(lhs.unit.multiply(&rhs.unit)),
        InfixOp::Div => Ok(lhs.unit.divide(&rhs.unit)),
        InfixOp::Pow => {
            if rhs.unit != Unit::new() {
                return Err(UnitError::IncompatibleUnits(
                    fc,
                    op,
                    lhs.unit.clone(),
                    rhs.unit.clone(),
                ));
            }
            match &rhs.kind {
                ValueKind::Number(n) => {
                    Ok(lhs.unit.pow(n))
                }
                _ => Err(UnitError::InvalidPowerValue(
                    fc,
                    lhs.unit.clone(),
                    rhs.kind.clone(),
                )),
            }
        }
        InfixOp::Eq => {
            if lhs.unit == rhs.unit {
                Ok(lhs.unit.clone())
            } else {
                Err(UnitError::IncompatibleUnits(
                    fc,
                    op,
                    lhs.unit.clone(),
                    rhs.unit.clone(),
                ))
            }
        }
        InfixOp::Neq => {
            if lhs.unit == rhs.unit {
                Ok(Unit::new())
            } else {
                Err(UnitError::IncompatibleUnits(
                    fc,
                    op,
                    lhs.unit.clone(),
                    rhs.unit.clone(),
                ))
            }
        }
        InfixOp::Gt => {
            if lhs.unit == rhs.unit {
                Ok(Unit::new())
            } else {
                Err(UnitError::IncompatibleUnits(
                    fc,
                    op,
                    lhs.unit.clone(),
                    rhs.unit.clone(),
                ))
            }
        }
    }
}

pub enum EvalResult {
    Empty,
    Value(Value),
    PrintValue(Expression, Value),
    VariableSearchResult {
        unit_aliases: BTreeMap<Name, Value>,
        variables: BTreeMap<Name, Value>,
        functions: BTreeMap<Name, Vec<Name>>,
    },
    UnitSearchResult {
        unit: Unit,
        unit_aliases: BTreeMap<Name, ValueKind>,
        variables: BTreeMap<Name, ValueKind>,
    },
}

#[derive(Debug, Error)]
pub enum ItemError {
    #[error("Unit redeclared: {}", .0)]
    UnitRedeclared(Name),
    #[error("Variable redefined: {}", .0)]
    VariableRedefined(Name),
    #[error("Function redefined: {}", .0)]
    FunctionRedefined(Name),

    #[error("Eval error: {}", .0)]
    EvalError(#[from] EvalError),
}

#[derive(Debug, Error)]
pub enum EvalError {
    #[error("Undefined name: {}", .1)]
    UndefinedName(FC, Name),

    #[error("Invalid prefix operator {:?} on value {} [{}]", .1, .2.kind, .2.unit)]
    InvalidPrefixOperator(FC, PrefixOp, Value),

    #[error("Invalid infix operator {:?} on {} [{}] and {} [{}]", .1, .2.kind, .2.unit, .3.kind, .3.unit)]
    InvalidInfixOperator(FC, InfixOp, Value, Value),

    #[error("Invalid SI-prefix {:?} on value {} [{}]", .1, .2.kind, .2.unit)]
    InvalidSiPrefix(FC, SiPrefix, Value),

    #[error("Equality assertion error {:?} on values {} [{}] and {} [{}]", .1, .2.kind, .2.unit, .3.kind, .3.unit)]
    EqualtyError(FC, InfixOp, Value, Value),

    #[error("Unit error: {}", .0)]
    UnitError(#[from] UnitError),
}

#[derive(Debug, Error)]
pub enum UnitError {
    #[error("Incompatible units ({}) and ({}) for operation {:?}", .2, .3, .1)]
    IncompatibleUnits(FC, InfixOp, Unit, Unit),

    #[error("Invalid power on unit ({}): {}", .1, .2)]
    InvalidPowerValue(FC, Unit, ValueKind),
}
