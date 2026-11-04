use etas_std::{StdIntrinsicId, intrinsic::pure};

use super::input::TextInput;
use crate::{BuiltinError, BuiltinValue};

mod projected;
mod work;

pub use work::TextQueryWork;

#[derive(Clone, Copy, Debug)]
pub enum TextQuery {
    Len,
    CharCount,
    IsEmpty,
    IsAscii,
    Contains,
    StartsWith,
    EndsWith,
}

impl TextQuery {
    pub fn for_intrinsic(id: StdIntrinsicId) -> Option<Self> {
        match id.0 {
            pure::TEXT_LEN => Some(Self::Len),
            pure::TEXT_CHAR_COUNT => Some(Self::CharCount),
            pure::TEXT_IS_EMPTY => Some(Self::IsEmpty),
            pure::TEXT_IS_ASCII => Some(Self::IsAscii),
            pure::TEXT_CONTAINS => Some(Self::Contains),
            pure::TEXT_STARTS_WITH => Some(Self::StartsWith),
            pure::TEXT_ENDS_WITH => Some(Self::EndsWith),
            _ => None,
        }
    }

    pub fn arity(self) -> usize {
        match self {
            Self::Len | Self::CharCount | Self::IsEmpty | Self::IsAscii => 1,
            Self::Contains | Self::StartsWith | Self::EndsWith => 2,
        }
    }

    pub fn evaluate(self, args: &[&str]) -> Result<BuiltinValue, BuiltinError> {
        self.evaluate_projected(args)
    }

    pub fn evaluate_projected<T: TextInput + ?Sized>(
        self,
        args: &[&T],
    ) -> Result<BuiltinValue, BuiltinError> {
        if args.len() != self.arity() {
            return Err(BuiltinError::ArityMismatch {
                expected: self.arity(),
                actual: args.len(),
            });
        }
        let mut flat = [""; 2];
        let contiguous = flat.iter_mut().zip(args).all(|(slot, text)| {
            if let Some(text) = text.contiguous() {
                *slot = text;
                true
            } else {
                false
            }
        });
        if contiguous {
            return Ok(self.evaluate_flat(&flat));
        }
        projected::evaluate(self, args)
    }

    fn evaluate_flat(self, args: &[&str]) -> BuiltinValue {
        match self {
            Self::Len => BuiltinValue::Usize(args[0].len()),
            Self::CharCount => BuiltinValue::Usize(args[0].chars().count()),
            Self::IsEmpty => BuiltinValue::Bool(args[0].is_empty()),
            Self::IsAscii => BuiltinValue::Bool(args[0].is_ascii()),
            Self::Contains => BuiltinValue::Bool(args[0].contains(args[1])),
            Self::StartsWith => BuiltinValue::Bool(args[0].starts_with(args[1])),
            Self::EndsWith => BuiltinValue::Bool(args[0].ends_with(args[1])),
        }
    }
}

#[cfg(test)]
mod projected_tests;
#[cfg(test)]
mod tests;
