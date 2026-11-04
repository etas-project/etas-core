use etas_std::StdIntrinsicId;

use crate::BuiltinTypeTag;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFailure {
    InvalidIndex,
    InvalidRange,
    NotCharBoundary,
    ParseFailure,
    NumericOverflow,
    LengthOverflow,
    InvalidFormat,
}

impl TextFailure {
    pub const fn variant_name(self) -> &'static str {
        match self {
            Self::InvalidIndex => "InvalidIndex",
            Self::InvalidRange => "InvalidRange",
            Self::NotCharBoundary => "NotCharBoundary",
            Self::ParseFailure => "ParseFailure",
            Self::NumericOverflow => "NumericOverflow",
            Self::LengthOverflow => "LengthOverflow",
            Self::InvalidFormat => "InvalidFormat",
        }
    }
}

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
    Text(TextFailure),
    ResourceExhausted,
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
