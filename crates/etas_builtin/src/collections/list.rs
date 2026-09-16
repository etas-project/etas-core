use crate::{BuiltinError, BuiltinValue};

use super::count::CountQuery;

pub fn len(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    CountQuery::Len.evaluate(args)
}

pub fn len_from_count(count: usize) -> BuiltinValue {
    CountQuery::Len.evaluate_count(count)
}

pub fn is_empty(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    CountQuery::IsEmpty.evaluate(args)
}

pub fn is_empty_from_count(count: usize) -> BuiltinValue {
    CountQuery::IsEmpty.evaluate_count(count)
}
