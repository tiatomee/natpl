use fraction::{BigDecimal, Sign};

pub fn dec_in_scientific_notation(d: &BigDecimal) -> (String, String, i64, Sign) {
    let d = d.clone().calc_precision(Some(150));
    let num = format!("{:.prec$}", d, prec = d.get_precision());

    let mut parts = num.split('.');
    let int_part = parts.next().unwrap();
    let (int_part, sign) = if int_part.starts_with('-') {
        (int_part.trim_start_matches('-'), Sign::Minus)
    } else if int_part.starts_with('+') {
        (int_part.trim_start_matches('+'), Sign::Plus)
    } else {
        (int_part, Sign::Plus)
    };

    if int_part == "0" || int_part == "-0" || int_part == "+0" || int_part.is_empty() {
        let dec_part = if let Some(dec) = parts.next() {
            dec
        } else {
            return ("0".to_string(), "".to_string(), 0, Sign::Plus);
        };
        let full_length = dec_part.len();
        let stripped_left = dec_part.trim_start_matches('0');
        let length_diff = full_length - stripped_left.len();
        let exp = -(length_diff as i64 + 1);
        let stripped_full = stripped_left.trim_end_matches('0');
        let (first, rest) = stripped_full.split_at(1);
        (first.to_string(), rest.to_string(), exp, sign)
    } else {
        let exp = int_part.len() - 1;
        let (start, int_rest) = int_part.split_at(1);
        let int_rest = int_rest.trim_end_matches('0');
        let dec_part = parts.next().unwrap_or("").trim_end_matches('0');
        let dec_part = format!("{}{}", int_rest, dec_part);
        (start.to_string(), dec_part, exp as i64, sign)
    }
}

pub fn from_decimal_str_and_exp(s: &str, exp: isize) -> BigDecimal {
    fn power_of_10(n: isize) -> BigDecimal {
        let steps = n.abs();

        let mut d = BigDecimal::from(1);

        // the `*=` doesn't work with type inference
        #[allow(clippy::assign_op_pattern)]
        for _ in 0..steps {
            d = d * 10u8.into();
        }

        if n.is_negative() {
            d.recip()
        } else {
            d
        }
    }

    BigDecimal::from_decimal_str(s).unwrap() * power_of_10(exp)
}

pub fn float_from_decimal(d: &BigDecimal) -> rug::Float {
    let str = format!("{:.prec$}", d, prec = d.get_precision().max(32));
    let val = rug::Float::parse(&str).unwrap();
    rug::Float::with_val(256, val)
}

pub fn decimal_from_float(f: &rug::Float) -> BigDecimal {
    let str = format!("{}", f);
    let mut parts = str.split(|c| c == 'e' || c == 'E');
    let mantissa = parts.next().unwrap();

    if let Some(exp) = parts.next() {
        let exp = exp.parse().unwrap();
        from_decimal_str_and_exp(mantissa, exp)
    } else {
        BigDecimal::from_decimal_str(mantissa).unwrap()
    }
}

pub fn max_precision(s: &str, prec: u8) -> String {
    let mut buf = String::with_capacity(prec as _);
    if s.len() < prec as usize {
        buf.push_str(s);
    } else {
        buf.push_str(&s[0..prec as _]);
    }

    buf
}

#[cfg(test)]
mod test {
    use fraction::{BigDecimal, Sign};

    use super::{dec_in_scientific_notation, from_decimal_str_and_exp as parse};

    #[test]
    fn avogadro() {
        let x = parse("6.022", 23);
        let (i, d, e, sign) = dec_in_scientific_notation(&x);
        assert_eq!(i, "6");
        assert_eq!(d, "022");
        assert_eq!(e, 23);
        assert_eq!(sign, Sign::Plus);
    }

    #[test]
    fn neg_exp_direct() {
        let x = parse("0.08", 0);
        let (i, d, e, _) = dec_in_scientific_notation(&x);
        assert_eq!(i, "8");
        assert_eq!(d, "");
        assert_eq!(e, -2);
    }

    #[test]
    fn neg_exp_e_notation() {
        let x = parse("0.8", -3);
        let (i, d, e, _) = dec_in_scientific_notation(&x);
        assert_eq!(i, "8");
        assert_eq!(d, "");
        assert_eq!(e, -4);
    }

    #[test]
    fn large_integer() {
        let x = parse("1234567890", 0);
        let (i, d, e, _) = dec_in_scientific_notation(&x);
        assert_eq!(i, "1");
        assert_eq!(d, "23456789");
        assert_eq!(e, 9);
    }

    #[test]
    fn fraction() {
        let x = BigDecimal::from(1) / BigDecimal::from(3);
        let (i, d, e, _) = dec_in_scientific_notation(&x);
        assert_eq!(i, "3");
        assert!(d.starts_with("3333333"));
        assert_eq!(e, -1);
    }
}
