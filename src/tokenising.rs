use logos::Logos;

pub type Span = logos::Span;

#[derive(Logos, Debug, Copy, Clone)]
pub enum Token<'input> {
    #[token("unit")]
    Unit,

    #[token("=")]
    OpEq,

    #[token("≠")]
    #[token("/=")]
    #[token("!=")]
    #[token("<>")]
    OpNeq,

    #[token(">")]
    OpGt,

    #[token("+")]
    OpAdd,

    #[token("-")]
    OpSub,

    #[token("*")]
    OpMul,

    #[token("/")]
    #[token("÷")]
    OpDiv,

    #[token("mod")]
    OpMod,

    #[token("^")]
    OpPow,

    #[token("[")]
    BracketOpen,

    #[token("]")]
    BracketClose,

    #[token("(")]
    ParenOpen,

    #[token(")")]
    ParenClose,

    #[token(",")]
    Comma,

    #[regex("(\\p{XID_Start}|_)(\\p{XID_Continue}|')*")]
    Identifier(&'input str),

    #[regex("[0-9]+", |lex| lex.slice().parse().ok())]
    IntegerLit(usize),

    #[regex(r"[0-9]+\.[0-9]+", |lex| {
        let mut parts = lex.slice().split('.');
        let integer = parts.next()?.parse().ok()?;
        let decimal = parts.next()?.parse().ok()?;
        Some((integer, decimal))
    })]
    FloatLit((usize, usize)),

    #[regex(r"[0-9]+\.[0-9]+x10\^-?[0-9]+", |lex| {
        let mut s = lex.slice().split("x10^");
        let (integer, decimal) = {
            let s = s.next()?;
            let mut parts = s.split('.');
            let integer = parts.next()?.parse().ok()?;
            let decimal = parts.next()?.parse().ok()?;
            (integer, decimal)
        };
        let exp = s.next()?.parse().ok()?;
        Some((integer, decimal, exp))
    })]
    #[regex(r"[0-9]+\.[0-9]+x10(⁰|¹|²|³|⁴|⁵|⁶|⁷|⁸|⁹)+", |lex| {
        let mut s = lex.slice().split("x10");
        let (integer, decimal) = {
            let s = s.next()?;
            let mut parts = s.split('.');
            let integer = parts.next()?.parse().ok()?;
            let decimal = parts.next()?.parse().ok()?;
            (integer, decimal)
        };
        let exp = unicode_power_num(s.next()?)?;
        Some((integer, decimal, exp as isize))
    })]
    ScientificFloatLit((usize, usize, isize)),

    #[regex(r"[0-9]+x10\^(-)?[0-9]+", |lex| {
        let mut s = lex.slice().split("x10^");
        let mantissa = s.next()?.parse().ok()?;
        let exp = s.next()?.parse().ok()?;
        Some((mantissa, exp))
    })]
    #[regex(r"[0-9]++x10(⁰|¹|²|³|⁴|⁵|⁶|⁷|⁸|⁹)+", |lex| {
        let mut s = lex.slice().split("x10");
        let mantissa = s.next()?.parse().ok()?;
        let exp = unicode_power_num(s.next()?)?;
        Some((mantissa, exp as isize))
    })]
    ScientificIntegerLit((usize, isize)),

    #[error]
    // skip whitespace
    #[regex(r"[ \t\n\f]+", logos::skip)]
    // line comments
    #[regex(r"#[^\n]*", logos::skip)]
    Error,
}

fn unicode_power_num(input: &str) -> Option<usize> {
    fn unicode_pow_to_digit_val(c: char) -> Option<usize> {
        match c {
            '⁰' => Some(0),
            '¹' => Some(1),
            '²' => Some(2),
            '³' => Some(3),
            '⁴' => Some(4),
            '⁵' => Some(5),
            '⁶' => Some(6),
            '⁷' => Some(7),
            '⁸' => Some(8),
            '⁹' => Some(9),
            _ => None,
        }
    }

    input.chars().try_fold(0usize, |acc, c| {
        acc.checked_mul(10)?
            .checked_add(unicode_pow_to_digit_val(c)?)
    })
}

pub fn tokenise<'src>(line: &'src str) -> Vec<(Token<'src>, Span)> {
    Token::lexer(line).spanned().collect()
}
