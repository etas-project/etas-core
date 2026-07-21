use etas_std::StdIntrinsicId;

use crate::BuiltinTypeTag;

#[derive(Clone, Debug, PartialEq)]
pub enum BuiltinError {
    UnsupportedIntrinsic {
        intrinsic: StdIntrinsicId,
    },
    ArityMismatch {
        expected: usize,
        actual: usize,
    },
    TypeMismatch {
        expected: BuiltinTypeTag,
        actual: BuiltinTypeTag,
    },
    NumericOverflow,
    DivideByZero,
    InvalidShift,
    InvalidSlice,
    InvalidUtf8,
    InvalidJson,
    AssertionFailed,
    Abort {
        message: String,
    },
}

pub(crate) fn expect_arity(
    args: &[crate::BuiltinValue],
    expected: usize,
) -> Result<(), BuiltinError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(BuiltinError::ArityMismatch {
            expected,
            actual: args.len(),
        })
    }
}
