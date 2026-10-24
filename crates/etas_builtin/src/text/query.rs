use etas_std::{StdIntrinsicId, intrinsic::pure};

use crate::{BuiltinError, BuiltinValue};

#[derive(Clone, Copy, Debug)]
pub enum TextQuery {
    Len,
    Contains,
    StartsWith,
    EndsWith,
    ParseI32,
}

impl TextQuery {
    pub fn for_intrinsic(id: StdIntrinsicId) -> Option<Self> {
        match id.0 {
            pure::TEXT_LEN => Some(Self::Len),
            pure::TEXT_CONTAINS => Some(Self::Contains),
            pure::TEXT_STARTS_WITH => Some(Self::StartsWith),
            pure::TEXT_ENDS_WITH => Some(Self::EndsWith),
            pure::TEXT_PARSE_I32 => Some(Self::ParseI32),
            _ => None,
        }
    }

    pub fn arity(self) -> usize {
        match self {
            Self::Len | Self::ParseI32 => 1,
            Self::Contains | Self::StartsWith | Self::EndsWith => 2,
        }
    }

    pub fn evaluate(self, args: &[&str]) -> Result<BuiltinValue, BuiltinError> {
        if args.len() != self.arity() {
            return Err(BuiltinError::ArityMismatch {
                expected: self.arity(),
                actual: args.len(),
            });
        }
        Ok(match self {
            Self::Len => BuiltinValue::Usize(args[0].chars().count()),
            Self::Contains => BuiltinValue::Bool(args[0].contains(args[1])),
            Self::StartsWith => BuiltinValue::Bool(args[0].starts_with(args[1])),
            Self::EndsWith => BuiltinValue::Bool(args[0].ends_with(args[1])),
            Self::ParseI32 => match args[0].parse::<i32>() {
                Ok(value) => BuiltinValue::ResultOk(Box::new(BuiltinValue::I32(value))),
                Err(error) => {
                    BuiltinValue::ResultErr(Box::new(BuiltinValue::String(error.to_string())))
                }
            },
        })
    }
}

#[cfg(test)]
mod tests;
