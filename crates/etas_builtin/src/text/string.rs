use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

pub fn trim(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    unary_string(args, |value| value.trim().to_owned())
}

pub fn len(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    unary_string_value(args, |value| BuiltinValue::Usize(value.chars().count()))
}

pub fn lowercase(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    unary_string(args, str::to_lowercase)
}

pub fn uppercase(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    unary_string(args, str::to_uppercase)
}

pub fn contains(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    binary_string_bool(args, |lhs, rhs| lhs.contains(rhs))
}

pub fn starts_with(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    binary_string_bool(args, |lhs, rhs| lhs.starts_with(rhs))
}

pub fn ends_with(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    binary_string_bool(args, |lhs, rhs| lhs.ends_with(rhs))
}

pub fn lines(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    unary_string_value(args, |value| {
        BuiltinValue::Array(
            value
                .lines()
                .map(|line| BuiltinValue::String(line.to_owned()))
                .collect(),
        )
    })
}

pub fn split(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 2)?;
    let BuiltinValue::String(value) = &args[0] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: args[0].type_tag(),
        });
    };
    let BuiltinValue::String(separator) = &args[1] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: args[1].type_tag(),
        });
    };
    Ok(BuiltinValue::Array(
        value
            .split(separator)
            .map(|part| BuiltinValue::String(part.to_owned()))
            .collect(),
    ))
}

pub fn join(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 2)?;
    let BuiltinValue::Array(parts) = &args[0] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Array,
            actual: args[0].type_tag(),
        });
    };
    let BuiltinValue::String(separator) = &args[1] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: args[1].type_tag(),
        });
    };
    let mut rendered = Vec::with_capacity(parts.len());
    for part in parts {
        let BuiltinValue::String(value) = part else {
            return Err(BuiltinError::TypeMismatch {
                expected: BuiltinTypeTag::String,
                actual: part.type_tag(),
            });
        };
        rendered.push(value.as_str());
    }
    Ok(BuiltinValue::String(rendered.join(separator)))
}

pub fn to_string_i32(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    unary_integer_to_string(args, |value| (value as i32).to_string())
}

pub fn to_string_usize(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    unary_integer_to_string(args, |value| (value as usize).to_string())
}

pub fn parse_i32(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    unary_string_value(args, |value| match value.parse::<i32>() {
        Ok(parsed) => BuiltinValue::ResultOk(Box::new(BuiltinValue::I32(parsed))),
        Err(error) => BuiltinValue::ResultErr(Box::new(BuiltinValue::String(error.to_string()))),
    })
}

fn unary_string(
    args: &[BuiltinValue],
    op: impl FnOnce(&str) -> String,
) -> Result<BuiltinValue, BuiltinError> {
    unary_string_value(args, |value| BuiltinValue::String(op(value)))
}

fn unary_string_value(
    args: &[BuiltinValue],
    op: impl FnOnce(&str) -> BuiltinValue,
) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    match &args[0] {
        BuiltinValue::String(value) => Ok(op(value)),
        other => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: other.type_tag(),
        }),
    }
}

fn binary_string_bool(
    args: &[BuiltinValue],
    op: impl FnOnce(&str, &str) -> bool,
) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 2)?;
    let BuiltinValue::String(lhs) = &args[0] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: args[0].type_tag(),
        });
    };
    let BuiltinValue::String(rhs) = &args[1] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: args[1].type_tag(),
        });
    };
    Ok(BuiltinValue::Bool(op(lhs, rhs)))
}

fn unary_integer_to_string(
    args: &[BuiltinValue],
    render: impl FnOnce(i64) -> String,
) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    let value = match &args[0] {
        BuiltinValue::I32(value) => i64::from(*value),
        BuiltinValue::I64(value) => *value,
        BuiltinValue::Usize(value) => *value as i64,
        other => {
            return Err(BuiltinError::TypeMismatch {
                expected: BuiltinTypeTag::I64,
                actual: other.type_tag(),
            });
        }
    };
    Ok(BuiltinValue::String(render(value)))
}
