use etas_std::{StdIntrinsicId, intrinsic::pure};

use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

#[derive(Clone, Copy, Debug)]
pub enum CountQuery {
    Len,
    IsEmpty,
}

impl CountQuery {
    pub fn for_intrinsic(id: StdIntrinsicId) -> Option<Self> {
        match id.0 {
            pure::LIST_LEN => Some(Self::Len),
            pure::LIST_IS_EMPTY => Some(Self::IsEmpty),
            _ => None,
        }
    }

    pub fn evaluate_count(self, count: usize) -> BuiltinValue {
        match self {
            Self::Len => BuiltinValue::Usize(count),
            Self::IsEmpty => BuiltinValue::Bool(count == 0),
        }
    }

    pub fn evaluate_text(self, text: &str) -> BuiltinValue {
        match self {
            Self::Len => BuiltinValue::Usize(text.chars().count()),
            Self::IsEmpty => BuiltinValue::Bool(text.is_empty()),
        }
    }

    pub fn evaluate(self, args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
        expect_arity(args, 1)?;
        let count = match &args[0] {
            BuiltinValue::Array(value) | BuiltinValue::List(value) | BuiltinValue::Slice(value) => {
                value.len()
            }
            BuiltinValue::Map(value) => value.len(),
            BuiltinValue::String(value) => return Ok(self.evaluate_text(value)),
            other => {
                return Err(BuiltinError::TypeMismatch {
                    expected: BuiltinTypeTag::Array,
                    actual: other.type_tag(),
                });
            }
        };
        Ok(self.evaluate_count(count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_preserve_container_and_unicode_scalar_semantics() {
        for (input, length) in [
            (BuiltinValue::Array(vec![BuiltinValue::Unit; 3]), 3),
            (BuiltinValue::List(vec![BuiltinValue::Unit; 2]), 2),
            (BuiltinValue::Slice(vec![BuiltinValue::Unit]), 1),
            (
                BuiltinValue::Map(vec![(BuiltinValue::Unit, BuiltinValue::Unit)]),
                1,
            ),
            (BuiltinValue::String("中😀e\u{301}".into()), 4),
            (BuiltinValue::String(String::new()), 0),
            (BuiltinValue::Array(vec![]), 0),
        ] {
            let args = [input];
            assert_eq!(
                CountQuery::Len.evaluate(&args),
                Ok(BuiltinValue::Usize(length))
            );
            assert_eq!(
                CountQuery::IsEmpty.evaluate(&args),
                Ok(BuiltinValue::Bool(length == 0))
            );
        }
    }

    #[test]
    fn queries_reject_unsupported_values_and_wrong_arity() {
        for query in [CountQuery::Len, CountQuery::IsEmpty] {
            assert_eq!(
                query.evaluate(&[]),
                Err(BuiltinError::ArityMismatch {
                    expected: 1,
                    actual: 0
                })
            );
            assert_eq!(
                query.evaluate(&[BuiltinValue::Unit, BuiltinValue::Unit]),
                Err(BuiltinError::ArityMismatch {
                    expected: 1,
                    actual: 2
                })
            );
            for input in [
                BuiltinValue::Bool(false),
                BuiltinValue::Bytes(vec![]),
                BuiltinValue::Set(vec![]),
            ] {
                let tag = input.type_tag();
                assert_eq!(
                    query.evaluate(&[input]),
                    Err(BuiltinError::TypeMismatch {
                        expected: BuiltinTypeTag::Array,
                        actual: tag
                    })
                );
            }
        }
    }
}
