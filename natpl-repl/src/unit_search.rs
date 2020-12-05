use std::collections::HashMap;

use fraction::{BigDecimal, ToPrimitive};
use natpl::{
    runtime::Runtime,
    syntax::{Name, SiPrefix},
    value_unit::{Value, ValueKind},
};

type Rating = u64;

pub(crate) fn unit_matches(rt: &Runtime, val: &Value) -> impl Iterator<Item = (Name, ValueKind)> {
    fn name_with_prefix(name: &str, prefix: Option<SiPrefix>) -> String {
        if let Some(pre) = prefix {
            if name.len() <= 2 {
                format!("{}{}", pre.short_prefix(), name)
            } else {
                format!("{}{}", pre.full_prefix(), name)
            }
        } else {
            name.to_string()
        }
    }

    let val = val.clone();

    let units = rt.find_units(&val.unit);

    let mut matches = closest_match(&val.kind, units);

    // Iterator speghetti!!
    // Try to take the best candidate, if it's actually anything
    // unique (not `s` vs second)
    matches
        .next()
        .into_iter()
        .filter({
            let val = val.clone();
            move |(_, _, val_kind)| val_kind != &val.kind
        })
        .map(|(name, pre, val_kind)| (name_with_prefix(&name, pre), val_kind))
        // Then take the "raw" unit, this one is always printed
        // as a kind of fallback in case the suggestion is not good.
        .chain(Some((val.unit.to_string(), val.kind.clone())).into_iter())
        // Then the rest of the candidates (that are unique)
        .chain(
            matches
                .filter({
                    let val = val.clone();
                    move |(_, _, val_kind)| val_kind != &val.kind
                })
                .map(|(name, pre, val_kind)| (name_with_prefix(&name, pre), val_kind)),
        )
}

/// List of best unit aliases (and their prefixes) to display
fn closest_match(
    val_kind: &ValueKind,
    matches: HashMap<&Name, ValueKind>,
) -> impl Iterator<Item = (Name, Option<SiPrefix>, ValueKind)> {
    let prefixes = &[
        SiPrefix::Femto,
        SiPrefix::Pico,
        SiPrefix::Nano,
        SiPrefix::Micro,
        SiPrefix::Milli,
        SiPrefix::Centi,
        // These three often end up doing weird things
        // and are not used very often anyway 👀
        // SiPrefix::Deci,
        // SiPrefix::Deca,
        // SiPrefix::Hecto,
        SiPrefix::Kilo,
        SiPrefix::Mega,
        SiPrefix::Giga,
        SiPrefix::Tera,
        SiPrefix::Peta,
    ];

    let mut rating: HashMap<String, (Option<SiPrefix>, ValueKind, Rating)> = Default::default();

    'matches: for (name, val) in matches {
        let (v, m) = match (val_kind, &val) {
            (ValueKind::Number(a), ValueKind::Number(b)) => (a, b),
            _ => continue 'matches,
        };

        for prefix in prefixes {
            let m = m * &prefix.value();

            let dist = num_distance(v, &m, 1.0);

            rating
                .entry(name.clone())
                .and_modify(|(pre, val, rating)| {
                    if dist < *rating {
                        *pre = Some(*prefix);
                        *val = ValueKind::Number(v / &m);
                        *rating = dist;
                    }
                })
                .or_insert_with(|| (Some(*prefix), val.clone(), dist));
        }

        let dist = num_distance(v, &m, 0.5);

        rating
            .entry(name.clone())
            .and_modify(|(pre, val, rating)| {
                if dist < *rating {
                    *pre = None;
                    *val = ValueKind::Number(v / m);
                    *rating = dist;
                }
            })
            .or_insert_with(|| (None, val.clone(), dist));
    }

    let mut ratings = rating.into_iter().collect::<Vec<_>>();
    ratings.sort_by(|(na, (pa, _, ra)), (nb, (pb, _, rb))| {
        pa.map(|s| s.sort_towards_middle())
            .cmp(&pb.map(|s| s.sort_towards_middle()))
            .then(ra.cmp(rb))
            .then(na.len().cmp(&nb.len()))
    });
    ratings
        .into_iter()
        .map(|(name, (prefix, val, _))| (name, prefix, val))
}

/// Grade how close a number is to another in terms of "niceness"
fn num_distance(a: &BigDecimal, b: &BigDecimal, handicap: f64) -> Rating {
    let div = a / b;
    let div_abs = div.abs();

    // Only take numbers between 1 and 999
    if div_abs < BigDecimal::from(1) || div_abs > BigDecimal::from(999) {
        Rating::max_value()
    } else {
        (div_abs.to_f64().unwrap_or(std::f64::NAN) * handicap) as Rating
    }
}
