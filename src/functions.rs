use fraction::BigDecimal;

use crate::{
    runtime::EvalError,
    runtime::UnitError,
    syntax::FC,
    value_unit::{Unit, Value, ValueKind},
};

macro_rules! functions {
    ($($($names:ident)|+ => $body:expr),*$(,)?) => {
        functions!(
            @names { $($($names,)*)* }
        );

        pub(crate) fn builtin_func(fc: FC, name: &str, base: &Value, args: &[(FC, Value)]) -> Option<Result<Value, EvalError>> {
            macro_rules! conv_float_fn {
                ($fun:expr) => {{
                    |n: &BigDecimal| {
                        let f = crate::num::float_from_decimal(n);
                        crate::num::decimal_from_float(&$fun(f))
                    }
                }};
            }

            macro_rules! argument_count_check {
                ($expected:expr) => {
                    if args.len() != $expected {
                        return Some(Err(EvalError::CallArgumentMismatch {
                            fc,
                            base: base.clone(),
                            num_args_expected: $expected,
                            num_args_applied: args.len(),
                        }));
                    }
                };
            }

            macro_rules! unary {
                ($unit_check:expr, $do_fn:expr, $unit_fn:expr) => {{
                    argument_count_check!(1);
                    let (arg_fc, arg) = &args[0];

                    if !$unit_check(&arg.unit) {
                        return Some(Err(EvalError::UnitError(UnitError::UnitMismatch { fc: *arg_fc, found: arg.unit.clone(), expected: Unit::new() })))
                    }

                    let kind = match &arg.kind {
                        ValueKind::Number(n) => {
                            ValueKind::Number($do_fn(n))
                        }
                        _ => return Some(Err(EvalError::ValueKindMismatch(*arg_fc, arg.clone()))),
                    };

                    let unit = $unit_fn(&arg.unit);

                    Ok(Value { kind, unit })
                }};
            }

            macro_rules! unary_unitless_float {
                ($do_fn:expr) => {
                    unary_unitless_float!($do_fn, Unit::new())
                };
                ($do_fn:expr, $unit:expr) => {
                    unary!(|unit| unit == &Unit::new(), conv_float_fn!($do_fn), |_| $unit)
                };
            }


            let res = match name {
                $($(stringify!($names))|* => $body,)*
                _ => return None,
            };

            Some(res)
        }
    };

    (@names { $($names:ident,)* }) => {
        pub(crate) static BUILTIN_FUNCTION_NAMES: &[&str] = &[
            $(stringify!($names),)*
        ];
    };
}

functions! {
    sin => unary_unitless_float!(|f: rug::Float| f.sin()),
    asin | arcsin => unary_unitless_float!(|f: rug::Float| f.asin()),

    cos => unary_unitless_float!(|f: rug::Float| f.cos()),
    acos | arccos => unary_unitless_float!(|f: rug::Float| f.acos()),

    tan => unary_unitless_float!(|f: rug::Float| f.tan()),
    atan | arctan => unary_unitless_float!(|f: rug::Float| f.atan()),

    sqrt => unary!(|_| true, conv_float_fn!(|f: rug::Float| f.sqrt()), |u: &Unit| u.pow(&(BigDecimal::from(1) / BigDecimal::from(2)))),
}
