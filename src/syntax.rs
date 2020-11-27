pub type Name = String;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Item {
    UnitDeclaration(Name),
    VariableDeclaration {
        name: Name,
        rhs: Expression,
    },
    FunctionDeclaration {
        name: Name,
        arg_names: Vec<Name>,
        rhs: Expression,
    },
    PrintedExpression(Expression),
    SilentExpression(Expression),
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum LineItem {
    UnitDeclaration(Name),
    MaybeDeclarationOrEqualityExpression(DeclarationOrEquality),
    PrintedExpression(Expression),
    SilentExpression(Expression),
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum DeclarationLhs {
    Variable(Name),
    Function { name: Name, args: Vec<Name> },
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct DeclarationOrEquality {
    pub lhs: DeclarationLhs,
    pub rhs: Expression,
}

impl DeclarationOrEquality {
    pub fn declaration_name(&self) -> &Name {
        match &self.lhs {
            DeclarationLhs::Variable(name) => name,
            DeclarationLhs::Function { name, args: _ } => name,
        }
    }

    pub fn into_expression(self) -> Item {
        let lhs = match self.lhs {
            DeclarationLhs::Variable(name) => Expression::Variable(name),
            DeclarationLhs::Function { name, args } => Expression::Call {
                base: Box::new(Expression::Variable(name)),
                args: args.into_iter().map(Expression::Variable).collect(),
            },
        };
        let expr = Expression::InfixOp {
            op: InfixOp::Eq,
            lhs: Box::new(lhs),
            rhs: Box::new(self.rhs),
        };
        Item::SilentExpression(expr)
    }

    pub fn into_declaration(self) -> Item {
        match self.lhs {
            DeclarationLhs::Variable(name) => Item::VariableDeclaration {
                name,
                rhs: self.rhs,
            },
            DeclarationLhs::Function { name, args } => Item::FunctionDeclaration {
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
        mantissa: usize,
        exponent: isize,
    },
    FloatLit {
        mantissa_int: usize,
        mantissa_dec: usize,
        exponent: isize,
    },

    MaybeUnitPrefix {
        name: Name,
        full_name: Name,
        prefix: SiPrefix,
    },

    Variable(Name),
    Call {
        base: Box<Expression>,
        args: Vec<Expression>,
    },

    PrefixOp {
        op: PrefixOp,
        expr: Box<Expression>,
    },

    InfixOp {
        op: InfixOp,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },

    UnitOf(Box<Expression>),

    /// Syntactic sugar for multiplication
    Juxtaposition {
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },
}

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

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum PrefixOp {
    Pos,
    Neg,
}
