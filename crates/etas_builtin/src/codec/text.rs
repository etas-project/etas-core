use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

pub fn utf8_encode(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    let BuiltinValue::String(text) = &args[0] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: args[0].type_tag(),
        });
    };
    Ok(BuiltinValue::Bytes(text.as_bytes().to_vec()))
}

pub fn utf8_decode(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 2)?;
    let BuiltinValue::Bytes(bytes) = &args[0] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Bytes,
            actual: args[0].type_tag(),
        });
    };
    let malformed = malformed_mode(&args[1])?;
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(BuiltinValue::ResultOk(Box::new(BuiltinValue::String(
            text.to_owned(),
        )))),
        Err(_) if malformed == MalformedInputMode::Replace => Ok(BuiltinValue::ResultOk(Box::new(
            BuiltinValue::String(String::from_utf8_lossy(bytes).into_owned()),
        ))),
        Err(_) => Ok(BuiltinValue::ResultErr(Box::new(BuiltinValue::Variant {
            name: "InvalidUtf8".to_owned(),
            fields: Vec::new(),
        }))),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MalformedInputMode {
    Strict,
    Replace,
}

fn malformed_mode(value: &BuiltinValue) -> Result<MalformedInputMode, BuiltinError> {
    match value {
        BuiltinValue::Variant { name, fields } if name == "Replace" && fields.is_empty() => {
            Ok(MalformedInputMode::Replace)
        }
        BuiltinValue::Variant { name, fields } if name == "Strict" && fields.is_empty() => {
            Ok(MalformedInputMode::Strict)
        }
        other => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Variant,
            actual: other.type_tag(),
        }),
    }
}
