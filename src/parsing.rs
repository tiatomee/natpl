use crate::{syntax::{DeclarationLhs, DeclarationOrEquality, Expression, InfixOp, LineItem, Name, PrefixOp, SiPrefix}, tokenising::{Span, Token}};
use thiserror::Error;

pub type Result<'src, T> = core::result::Result<T, ParseError<'src>>;

#[derive(Debug, Clone, Error)]
pub enum ParseError<'src> {
    #[error("Unexpected end of input")]
    UnexpectedEnd,
    #[error("Unexpected token found at col {}: {:?}", .1.start, .0)]
    UnexpectedToken(Token<'src>, Span),
}


pub struct Parser<'toks, 'src> {
    toks: &'toks [(Token<'src>, Span)],
}

impl<'toks, 'src> Parser<'toks, 'src> {
    pub fn parse_line(line_toks: &'toks [(Token<'src>, Span)]) -> Result<'src, LineItem> {
        let mut this = Self { toks: line_toks };

        if let Ok(unit) = this.parse_unit_declaration() {
            return Ok(LineItem::UnitDeclaration(unit));
        } else {
            this.toks = line_toks;
        }

        if let Ok(expr) = this.parse_printed_expr() {
            return Ok(LineItem::PrintedExpression(expr));
        } else {
            this.toks = line_toks;
        };

        if let Ok(decl_or_eq) = this.parse_declaration_or_equality() {
            return Ok(LineItem::MaybeDeclarationOrEqualityExpression(decl_or_eq));
        } else {
            this.toks = line_toks;
        };

        let expr = this.parse_expr()?;

        Ok(LineItem::SilentExpression(expr))
    }

    fn parse_unit_declaration(&mut self) -> Result<'src, Name> {
        self.expect(|t| matches!(t, Token::Unit))?;
        let name = self.expect(|t| match t {
            Token::Identifier(name) => Some(name),
            _ => None,
        })?;

        if !self.toks.is_empty() {
            Err(ParseError::UnexpectedEnd)
        } else {
            Ok(name.into())
        }
    }

    fn parse_declaration_or_equality(&mut self) -> Result<'src, DeclarationOrEquality> {
        let name = self.expect_identifier()?;

        let lhs = match self.peek() {
            Some(Token::ParenOpen) => {
                // function call / definition
                let args = self.paren_list(|p| p.expect_identifier().map(|s| s.into()))?;
                DeclarationLhs::Function {
                    name: name.into(),
                    args,
                }
            }
            _ => DeclarationLhs::Variable(name.into()),
        };

        self.expect(|t| matches!(t, Token::OpEq))?;

        let rhs = self.parse_expr()?;

        if !self.toks.is_empty() {
            Err(ParseError::UnexpectedEnd)
        } else {
            Ok(DeclarationOrEquality { lhs, rhs })
        }
    }

    fn parse_printed_expr(&mut self) -> Result<'src, Expression> {
        self.expect(|t| matches!(t, Token::OpGt))?;
        self.parse_expr()
    }

    fn parse_expr(&mut self) -> Result<'src, Expression> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<'src, Expression> {

        let t = self.peek().ok_or(ParseError::UnexpectedEnd)?;

        let mut lhs = if let Some(((), bp, op)) = prefix_binding_power(t) {
            self.next()?;
            let rhs = self.parse_expr_bp(bp)?;
            Expression::PrefixOp {
                op,
                expr: Box::new(rhs),
            }
        } else {
            self.parse_expr_atom()?
        };

        loop {
            let t = match self.peek() {
                Some(t) => t,
                None => break,
            };

            if let Some((l_bp, ())) = postfix_binding_power(t) {
                if l_bp < min_bp {
                    break;
                }

                if matches!(t, Token::ParenOpen) {
                    let args = self.paren_list(Self::parse_expr)?;
                    lhs = Expression::Call {
                        base: Box::new(lhs),
                        args,
                    };
                } else {
                    unreachable!()
                }
                continue;
            }

            let (l_bp, r_bp, op) = 
                    if let Some(val) = infix_binding_power(t) {
                        if !matches!(t, Token::Identifier(_)) {
                            self.next()?;
                        }
                        val
                    } else {
                        break
                    };

            if l_bp < min_bp {
                break
            }

            let rhs = self.parse_expr_bp(r_bp)?;

            lhs = Expression::InfixOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    fn parse_expr_atom(&mut self) -> Result<'src, Expression> {
        let (t, span) = self.next_with_span()?;
        match t {
            Token::BracketOpen => {
                let expr = self.parse_expr()?;
                self.expect(|t| matches!(t, Token::BracketClose))?;
                Ok(Expression::UnitOf(Box::new(expr)))
            },
            Token::ParenOpen => {
                let expr = self.parse_expr()?;
                self.expect(|t| matches!(t, Token::ParenClose))?;
                Ok(expr)
            }
            Token::Identifier(name) => {
                if let Some((prefix, stripped)) = identifier_maybe_unit_prefix(name) {
                    Ok(Expression::MaybeUnitPrefix {
                        prefix,
                        full_name: name.into(),
                        name: stripped.into(),
                    })
                } else {
                    Ok(Expression::Variable(name.into()))
                }
            }
            Token::IntegerLit(val) => Ok(Expression::IntegerLit {
                mantissa: val,
                exponent: 0,
            }),
            Token::FloatLit((int, dec)) => Ok(Expression::FloatLit {
                mantissa_int: int,
                mantissa_dec: dec,
                exponent: 0,
            }),
            Token::ScientificFloatLit((int, dec, exp)) => Ok(Expression::FloatLit {
                mantissa_int: int,
                mantissa_dec: dec,
                exponent: exp,
            }),
            Token::ScientificIntegerLit((val, exp)) => Ok(Expression::IntegerLit {
                mantissa: val,
                exponent: exp,
            }),
            t => Err(ParseError::UnexpectedToken(t, span)),
        }
    }

    fn expect_identifier(&mut self) -> Result<'src, &'src str> {
        self.expect(|t| match t {
            Token::Identifier(name) => Some(name),
            _ => None,
        })
    }

    fn paren_list<T>(
        &mut self,
        mut f: impl FnMut(&mut Self) -> Result<'src, T>,
    ) -> Result<'src, Vec<T>> {
        self.expect(|t| matches!(t, Token::ParenOpen))?;

        let mut vals = vec![];

        loop {
            if let Some(Token::ParenClose) = self.peek() {
                self.next()?;
                return Ok(vals);
            }

            vals.push(f(self)?);

            let end = self.expect(|t| {
                if matches!(t, Token::ParenClose) {
                    Some(true)
                } else if matches!(t, Token::Comma) {
                    Some(false)
                } else {
                    None
                }
            })?;

            if end {
                return Ok(vals);
            }
        }
    }

    fn peek(&mut self) -> Option<Token<'src>> {
        match self.toks {
            [] => None,
            [(t, _), ..] => Some(*t),
        }
    }

    fn next(&mut self) -> Result<'src, Token<'src>> {
        match self.toks {
            [] => Err(ParseError::UnexpectedEnd),
            [(t, _), rest @ ..] => {
                self.toks = rest;
                Ok(*t)
            }
        }
    }

    fn next_with_span(&mut self) -> Result<'src, (Token<'src>, Span)> {
        match self.toks {
            [] => Err(ParseError::UnexpectedEnd),
            [(t, span), rest @ ..] => {
                self.toks = rest;
                Ok((*t, span.clone()))
            }
        }
    }

    fn expect<T: ExpectRet>(
        &mut self,
        f: impl FnOnce(Token<'src>) -> T,
    ) -> Result<'src, T::RetType> {
        match self.toks {
            [] => Err(ParseError::UnexpectedEnd),
            [(t, span), rest @ ..] => match f(*t).as_option() {
                Some(res) => {
                    self.toks = rest;
                    Ok(res)
                }
                None => Err(ParseError::UnexpectedToken(*t, span.clone())),
            },
        }
    }
}

fn prefix_binding_power(t: Token<'_>) -> Option<((), u8, PrefixOp)> {
    match t {
        Token::OpAdd => Some(((), 5, PrefixOp::Pos)),
        Token::OpSub => Some(((), 5, PrefixOp::Neg)),
        _ => None,
    }
}

fn infix_binding_power(t: Token<'_>) -> Option<(u8, u8, InfixOp)> {
    match t {
        
        Token::OpPow => Some((91, 90, InfixOp::Pow)),
        Token::OpMul | Token::Identifier(_) => Some((80, 81, InfixOp::Mul)),
        Token::OpDiv => Some((80, 81, InfixOp::Div)),
        Token::OpMod => Some((80, 81, InfixOp::Mod)),
        Token::OpAdd => Some((70, 71, InfixOp::Add)),
        Token::OpSub => Some((70, 71, InfixOp::Sub)),

        Token::OpEq => Some((20, 21, InfixOp::Eq)),
        Token::OpNeq => Some((20, 21, InfixOp::Neq)),
        Token::OpGt => Some((20, 21, InfixOp::Gt)),
        
        _ => None
    }
}

fn postfix_binding_power(t: Token<'_>) -> Option<(u8, ())> {
    match t {
        Token::ParenOpen => Some((255, ())),
        _ => None,
    }
}

fn identifier_maybe_unit_prefix(name: &str) -> Option<(SiPrefix, &str)> {
    use SiPrefix::*;

    const PREFIXES: &[(&str, SiPrefix)] = &[
        // long form
        ("femto", Femto),
        ("pico", Pico),
        ("nano", Nano),
        ("micro", Micro),
        ("milli", Milli),
        ("centi", Centi),
        ("deci", Deci),
        ("deca", Deca),
        ("hecto", Hecto),
        ("kilo", Kilo),
        ("mega", Mega),
        ("giga", Giga),
        ("tera", Tera),
        ("peta", Peta),
        // short form
        ("f", Femto),
        ("p", Pico),
        ("n", Nano),
        ("μ", Micro),
        ("m", Milli),
        ("c", Centi),
        ("d", Deci),
        ("da", Deca),
        ("h", Hecto),
        ("k", Kilo),
        ("M", Mega),
        ("G", Giga),
        ("T", Tera),
        ("P", Peta),
    ];

    for (prefix, si) in PREFIXES {
        if let Some(n) = name.strip_prefix(prefix) {
            return Some((*si, n));
        }
    }
    None
}

trait ExpectRet {
    type RetType;

    fn as_option(self) -> Option<Self::RetType>;
}

impl<T> ExpectRet for Option<T> {
    type RetType = T;

    fn as_option(self) -> Option<Self::RetType> {
        self
    }
}

impl ExpectRet for bool {
    type RetType = ();

    fn as_option(self) -> Option<Self::RetType> {
        if self {
            Some(())
        } else {
            None
        }
    }
}
