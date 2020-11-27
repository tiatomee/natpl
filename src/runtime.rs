use std::{collections::{HashMap, HashSet}};

use crate::{syntax::{Expression, FC, HasFC, InfixOp, Item, LineItem, Name, PrefixOp}, term::{Term, Value, ValueKind}};

use thiserror::Error;



#[derive(Default)]
pub struct Runtime {
    units: HashSet<Name>,
    variables: HashMap<Name, Value>,
    functions: HashMap<Name, (Vec<Name>, Expression)>,
}

impl Runtime {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn eval_line_item(&mut self, item: LineItem) -> Result<EvalResult, ItemError> {
        let item = self.line_item_to_item(item);
        self.eval_item(item)
    }

    pub fn line_item_to_item(&mut self, item: LineItem) -> Item {
        match item {
            LineItem::UnitDeclaration(fc, name) => {
                Item::UnitDeclaration(fc, name)
            },
            LineItem::MaybeDeclarationOrEqualityExpression(decl) => {

                let name = decl.declaration_name();

                let is_defined = self.units.contains(name) || self.variables.contains_key(name) || self.functions.contains_key(name);

                if is_defined {
                    decl.into_expression()
                } else {
                    decl.into_declaration()
                }
            }
            LineItem::PrintedExpression(fc, expr) => {
                Item::PrintedExpression(fc, expr)
            }
            LineItem::SilentExpression(expr) => {
                Item::SilentExpression(expr)
            }
        }
    }

    fn eval_item(&mut self, item: Item) -> Result<EvalResult, ItemError> {
        use std::collections::hash_map::Entry;

        match item {
            Item::UnitDeclaration(_, name) => {
                let name = name.name();

                let is_unique = self.units.insert(name.clone());
                if !is_unique {
                    Err(ItemError::UnitRedeclared(name))
                } else {
                    Ok(EvalResult::Empty)
                }
            }
            Item::VariableDeclaration { fc: _, name, rhs } => {
                let value = self.eval_expr(&rhs)?;

                let name = name.name();
                match self.variables.entry(name.clone()) {
                    Entry::Occupied(_) => {
                        Err(ItemError::VariableRedefined(name))
                    }
                    Entry::Vacant(entry) => {
                        let val = entry.insert(value).clone();
                        Ok(EvalResult::Value(val))
                    }
                }
            }
            Item::FunctionDeclaration { fc: _, name, arg_names, rhs } => {
                let name = name.name();

                match self.functions.entry(name.clone()) {
                    Entry::Occupied(_) => {
                        Err(ItemError::FunctionRedefined(name))
                    }
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
            Expression::IntegerLit { fc: _, mantissa, exponent } => {
                let num: f64 = format!("{}e{}", mantissa, exponent).parse().unwrap();
                Ok(Value {
                    kind: ValueKind::Number(num),
                    unit: Term::Value(ValueKind::Number(1.0)),
                })
            }
            Expression::FloatLit { fc: _, mantissa_int, mantissa_dec, exponent } => {
                // forgive me
                let num: f64 = format!("{}.{}e{}", mantissa_int, mantissa_dec, exponent).parse().unwrap();
                Ok(Value {
                    kind: ValueKind::Number(num),
                    unit: Term::Value(ValueKind::Number(1.0)),
                })
            }
            Expression::MaybeUnitPrefix { fc: _, name, full_name, prefix } => {
                if let Some(val) = self.lookup(full_name) {
                    return Ok(val);
                }

                if let Some(val) = self.lookup(name) {
                    todo!()
                } else {
                    todo!()
                }
            }
            Expression::Variable(id) => {
                self.lookup(id.name_ref()).ok_or_else(|| EvalError::UndefinedName(id.fc(), id.name_ref().clone()))
            }
            Expression::Call { fc: _, base, args } => {
                todo!()
            }
            Expression::PrefixOp { fc, op, expr } => {
                let mut val = self.eval_expr(expr)?;
                match op {
                    crate::syntax::PrefixOp::Pos => {
                        match &mut val.kind {
                            ValueKind::Number(num) => Ok(val),
                            ValueKind::FunctionRef(_) => Err(EvalError::InvalidPrefixOperator(*fc, *op, val)),
                        }
                    }
                    crate::syntax::PrefixOp::Neg => {
                        match &mut val.kind {
                            ValueKind::Number(num) => {
                                *num *= -1.0;
                                Ok(val)
                            }
                            ValueKind::FunctionRef(_) => Err(EvalError::InvalidPrefixOperator(*fc, *op, val)),
                        }
                    }
                }
            }
            Expression::InfixOp { fc, op, lhs, rhs } => {
                self.eval_infix_op(*fc, *op, lhs, rhs)
            },
            Expression::UnitOf(_, expr) => {
                let val = self.eval_expr(expr)?;
                Ok(Value {
                    kind: ValueKind::Number(1.0),
                    unit: val.unit,
                })
            },
            Expression::Parenthesised(_, expr) => self.eval_expr(expr)
        }
    }

    pub fn lookup(&self, name: &Name) -> Option<Value> {
        if let Some(val) = self.variables.get(name) {
            Some(val.clone())
        } else if self.units.contains(name) {
            Some(Value {
                kind: ValueKind::Number(1.0),
                unit: Term::UnitRef(name.clone()),
            })
        } else if self.functions.contains_key(name) {
            Some(Value {
                kind: ValueKind::FunctionRef(name.clone()),
                unit: Term::Value(ValueKind::Number(1.0)),
            })
        } else {
            None
        }
    }

    fn eval_infix_op(&self, fc: FC, op: InfixOp, lhs: &Expression, rhs: &Expression) -> Result<Value, EvalError> {
        let lhs = self.eval_expr(lhs)?;
        let rhs = self.eval_expr(rhs)?;

        match (op, &lhs.kind, &rhs.kind) {
            (InfixOp::Add, ValueKind::Number(a), ValueKind::Number(b)) => {
                let unit = lhs.unit.clone();
                Ok(Value {
                    kind: ValueKind::Number(a + b),
                    unit,
                })
            }
            (InfixOp::Sub, ValueKind::Number(a), ValueKind::Number(b)) => {
                let unit = lhs.unit.clone();
                Ok(Value {
                    kind: ValueKind::Number(a - b),
                    unit,
                })
            }
            (InfixOp::Mul, ValueKind::Number(a), ValueKind::Number(b)) => {
                let unit = lhs.unit.clone();
                Ok(Value {
                    kind: ValueKind::Number(a * b),
                    unit,
                })
            }
            (InfixOp::Div, ValueKind::Number(a), ValueKind::Number(b)) => {
                let unit = lhs.unit.clone();
                Ok(Value {
                    kind: ValueKind::Number(a / b),
                    unit,
                })
            }
            (InfixOp::Mod, ValueKind::Number(a), ValueKind::Number(b)) => {
                let unit = lhs.unit.clone();
                Ok(Value {
                    kind: ValueKind::Number(a.rem_euclid(*b)),
                    unit,
                })
            }
            (InfixOp::Pow, ValueKind::Number(a), ValueKind::Number(b)) => {
                let unit = lhs.unit.clone();
                Ok(Value {
                    kind: ValueKind::Number(a.powf(*b)),
                    unit,
                })
            }
            (InfixOp::Eq, ValueKind::Number(a), ValueKind::Number(b)) => {
                let unit = lhs.unit.clone();
                Ok(Value {
                    kind: todo!(),
                    unit,
                })
            }
            (InfixOp::Neq, ValueKind::Number(a), ValueKind::Number(b)) => {
                let unit = lhs.unit.clone();
                Ok(Value {
                    kind: todo!(),
                    unit,
                })
            }
            (InfixOp::Gt, ValueKind::Number(a), ValueKind::Number(b)) => {
                let unit = lhs.unit.clone();
                Ok(Value {
                    kind: todo!(),
                    unit,
                })
            }
            (op, _, _) => {
                Err(EvalError::InvalidInfixOperator(fc, op, lhs, rhs))
            }
        }
    }
}

pub enum EvalResult {
    Empty,
    Value(Value),
    PrintValue(Expression, Value),
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

    #[error("Invalid prefix operator {:?} on value {:?}", .1, .2)]
    InvalidPrefixOperator(FC, PrefixOp, Value),

    #[error("Invalid infix operator {:?} on {:?} and {:?}", .1, .2, .3)]
    InvalidInfixOperator(FC, InfixOp, Value, Value),
}