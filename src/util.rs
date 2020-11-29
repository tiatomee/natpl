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

pub fn dec_with_max_precision(s: &str, prec: u8) -> String {
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
    fn thing() {
        let x = "6.022e23".parse().unwrap();
        dbg!(dec_in_scientific_notation(&x));
        panic!()
    }
}
