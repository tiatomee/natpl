use bigdecimal::BigDecimal;

pub fn dec_in_scientific_notation(d: &BigDecimal) -> (String, String, i64) {
    let (num, exp) = d.as_bigint_and_exponent();

    let num_str = num.to_string();

    let (int_str, dec_str) = if num_str.starts_with('-') {
        num_str.split_at(2)
    } else {
        num_str.split_at(1)
    };

    let ex = -exp + (dec_str.len() as i64);

    (int_str.into(), dec_str.into(), ex)
}

pub fn max_precision(s: &str, prec: u8) -> String {
    let mut buf = String::with_capacity(prec as _);
    if s.len() < prec as _ {
        buf.push_str(s);
    } else {
        buf.push_str(&s[0..prec as _]);
    }

    buf
}

#[cfg(test)]
mod test {
    use super::dec_in_scientific_notation;

    #[test]
    fn avogadro() {
        let x = "6.022e23".parse().unwrap();
        let (i, d, e) = dec_in_scientific_notation(&x);
        assert_eq!(i, "6");
        assert_eq!(d, "022");
        assert_eq!(e, 23);
    }

    #[test]
    fn neg_exp_direct() {
        let x = "0.08".parse().unwrap();
        let (i, d, e) = dec_in_scientific_notation(&x);
        assert_eq!(i, "8");
        assert_eq!(d, "");
        assert_eq!(e, -2);
    }

    #[test]
    fn neg_exp_e_notation() {
        let x = "0.8e-3".parse().unwrap();
        let (i, d, e) = dec_in_scientific_notation(&x);
        assert_eq!(i, "8");
        assert_eq!(d, "");
        assert_eq!(e, -4);
    }

    #[test]
    fn large_integer() {
        let x = "1234567890".parse().unwrap();
        let (i, d, e) = dec_in_scientific_notation(&x);
        assert_eq!(i, "1");
        assert_eq!(d, "234567890");
        assert_eq!(e, 9);
    }
}
