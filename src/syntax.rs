use std::fmt::Display;

use bigdecimal::BigDecimal;

use crate::tokenising::Span;

pub type Name = String;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Identifier(pub FC, pub Name);

impl Identifier {
    pub fn name_ref(&self) -> &Name {
        &self.1
    }

    pub fn name(self) -> Name {
        self.1
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct FC {
    pub start: usize,
    pub end: usize,
}

impl FC {
    pub fn from_span(span: Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }

    pub fn merge(self, other: FC) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Item {
    UnitDeclaration(FC, Identifier),
    UnitAlias(FC, Identifier, Expression),
    VariableDeclaration {
        fc: FC,
        name: Identifier,
        rhs: Expression,
    },
    FunctionDeclaration {
        fc: FC,
        name: Identifier,
        arg_names: Vec<Identifier>,
        rhs: Expression,
    },
    PrintedExpression(FC, Expression),
    SilentExpression(Expression),
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum LineItem {
    Empty,
    UnitDeclaration(FC, Identifier),
    UnitAlias(FC, Identifier, Expression),
    MaybeDeclarationOrEqualityExpression(DeclarationOrEquality),
    PrintedExpression(FC, Expression),
    SilentExpression(Expression),
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum DeclarationLhs {
    Variable(Identifier),
    Function {
        fc: FC,
        name: Identifier,
        args: Vec<Identifier>,
    },
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct DeclarationOrEquality {
    pub fc: FC,
    pub lhs: DeclarationLhs,
    pub rhs: Expression,
}

impl DeclarationOrEquality {
    pub fn declaration_name(&self) -> &Name {
        match &self.lhs {
            DeclarationLhs::Variable(name) => &name.1,
            DeclarationLhs::Function { name, .. } => &name.1,
        }
    }

    pub fn into_expression(self) -> Item {
        let lhs = match self.lhs {
            DeclarationLhs::Variable(ident) => Expression::Variable(ident),
            DeclarationLhs::Function { fc, name, args } => Expression::Call {
                fc,
                base: Box::new(Expression::Variable(name)),
                args: args.into_iter().map(Expression::Variable).collect(),
            },
        };
        let expr = Expression::InfixOp {
            fc: self.fc,
            op: InfixOp::Eq,
            lhs: Box::new(lhs),
            rhs: Box::new(self.rhs),
        };
        Item::SilentExpression(expr)
    }

    pub fn into_declaration(self) -> Item {
        match self.lhs {
            DeclarationLhs::Variable(name) => Item::VariableDeclaration {
                fc: name.fc(),
                name,
                rhs: self.rhs,
            },
            DeclarationLhs::Function { fc, name, args } => Item::FunctionDeclaration {
                fc,
                name,
                arg_names: args,
                rhs: self.rhs,
            },
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Expression {
    IntegerLit {
        fc: FC,
        val: BigDecimal,
    },
    FloatLit {
        fc: FC,
        val: BigDecimal,
    },

    MaybeUnitPrefix {
        fc: FC,
        name: Name,
        full_name: Name,
        prefix: SiPrefix,
    },

    Variable(Identifier),
    Call {
        fc: FC,
        base: Box<Expression>,
        args: Vec<Expression>,
    },

    PrefixOp {
        fc: FC,
        op: PrefixOp,
        expr: Box<Expression>,
    },

    InfixOp {
        fc: FC,
        op: InfixOp,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },

    UnitOf(FC, Box<Expression>),
    Parenthesised(FC, Box<Expression>),
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum SiPrefix {
    Femto,
    Pico,
    Nano,
    Micro,
    Milli,
    Centi,
    Deci,
    Deca,
    Hecto,
    Kilo,
    Mega,
    Giga,
    Tera,
    Peta,
}

impl SiPrefix {
    pub fn value(&self) -> BigDecimal {
        match self {
            SiPrefix::Femto => BigDecimal::from(1_000_000_000_000_000u64).inverse(),
            SiPrefix::Pico => BigDecimal::from(1_000_000_000_000u64).inverse(),
            SiPrefix::Nano => BigDecimal::from(1_000_000_000u64).inverse(),
            SiPrefix::Micro => BigDecimal::from(1_000_000u64).inverse(),
            SiPrefix::Milli => BigDecimal::from(1_000u64).inverse(),
            SiPrefix::Centi => BigDecimal::from(100u64).inverse(),
            SiPrefix::Deci => BigDecimal::from(10u64).inverse(),
            SiPrefix::Deca => BigDecimal::from(10u64),
            SiPrefix::Hecto => BigDecimal::from(100u64),
            SiPrefix::Kilo => BigDecimal::from(1_000u64),
            SiPrefix::Mega => BigDecimal::from(1_000_000u64),
            SiPrefix::Giga => BigDecimal::from(1_000_000_000u64),
            SiPrefix::Tera => BigDecimal::from(1_000_000_000_000u64),
            SiPrefix::Peta => BigDecimal::from(1_000_000_000_000_000u64),
        }
    }

    pub fn short_prefix(&self) -> &'static str {
        match self {
            SiPrefix::Femto => "f",
            SiPrefix::Pico => "p",
            SiPrefix::Nano => "n",
            SiPrefix::Micro => "μ",
            SiPrefix::Milli => "m",
            SiPrefix::Centi => "c",
            SiPrefix::Deci => "d",
            SiPrefix::Deca => "da",
            SiPrefix::Hecto => "H",
            SiPrefix::Kilo => "K",
            SiPrefix::Mega => "M",
            SiPrefix::Giga => "G",
            SiPrefix::Tera => "T",
            SiPrefix::Peta => "P",
        }
    }

    pub fn full_prefix(&self) -> &'static str {
        match self {
            SiPrefix::Femto => "femto",
            SiPrefix::Pico => "pico",
            SiPrefix::Nano => "nano",
            SiPrefix::Micro => "micro",
            SiPrefix::Milli => "milli",
            SiPrefix::Centi => "centi",
            SiPrefix::Deci => "deci",
            SiPrefix::Deca => "deca",
            SiPrefix::Hecto => "hecto",
            SiPrefix::Kilo => "kilo",
            SiPrefix::Mega => "mega",
            SiPrefix::Giga => "giga",
            SiPrefix::Tera => "tera",
            SiPrefix::Peta => "peta",
        }
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum InfixOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    Pow,

    Eq,
    Neq,
    Gt,
}

impl Display for InfixOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            InfixOp::Add => "+",
            InfixOp::Sub => "-",
            InfixOp::Mul => "*",
            InfixOp::Div => "/",
            InfixOp::Mod => "mod",
            InfixOp::Pow => "^",
            InfixOp::Eq => "=",
            InfixOp::Neq => "≠",
            InfixOp::Gt => ">",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum PrefixOp {
    Pos,
    Neg,
}

pub trait HasFC {
    fn fc(&self) -> FC;
}

impl HasFC for Identifier {
    fn fc(&self) -> FC {
        self.0
    }
}

impl HasFC for Item {
    fn fc(&self) -> FC {
        match self {
            Item::UnitDeclaration(fc, _) => *fc,
            Item::UnitAlias(fc, _, _) => *fc,
            Item::VariableDeclaration { fc, .. } => *fc,
            Item::FunctionDeclaration { fc, .. } => *fc,
            Item::PrintedExpression(fc, _) => *fc,
            Item::SilentExpression(expr) => expr.fc(),
        }
    }
}

impl HasFC for LineItem {
    fn fc(&self) -> FC {
        match self {
            LineItem::Empty => FC { start: 0, end: 0 },
            LineItem::UnitDeclaration(fc, _) => *fc,
            LineItem::UnitAlias(fc, _, _) => *fc,
            LineItem::MaybeDeclarationOrEqualityExpression(decl) => decl.fc(),
            LineItem::PrintedExpression(fc, _) => *fc,
            LineItem::SilentExpression(expr) => expr.fc(),
        }
    }
}

impl HasFC for DeclarationLhs {
    fn fc(&self) -> FC {
        match self {
            DeclarationLhs::Variable(ident) => ident.fc(),
            DeclarationLhs::Function { fc, .. } => *fc,
        }
    }
}

impl HasFC for DeclarationOrEquality {
    fn fc(&self) -> FC {
        self.fc
    }
}

impl HasFC for Expression {
    fn fc(&self) -> FC {
        match self {
            Expression::IntegerLit { fc, .. } => *fc,
            Expression::FloatLit { fc, .. } => *fc,
            Expression::MaybeUnitPrefix { fc, .. } => *fc,
            Expression::Variable(ident) => ident.fc(),
            Expression::Call { fc, .. } => *fc,
            Expression::PrefixOp { fc, .. } => *fc,
            Expression::InfixOp { fc, .. } => *fc,
            Expression::UnitOf(fc, _) => *fc,
            Expression::Parenthesised(fc, _) => *fc,
        }
    }
}

impl<T> HasFC for (T, Span) {
    fn fc(&self) -> FC {
        FC::from_span(self.1.clone())
    }
}

impl From<Span> for FC {
    fn from(s: Span) -> Self {
        FC::from_span(s)
    }
}

impl From<&'_ Span> for FC {
    fn from(s: &Span) -> Self {
        FC::from_span(s.clone())
    }
}
