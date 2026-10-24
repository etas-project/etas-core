use etas_std::{StdIntrinsicId, intrinsic::pure};

use crate::{BuiltinError, BuiltinValue};

#[derive(Clone, Copy, Debug)]
pub enum BytesQuery {
    Len,
    Sha256,
    ConstantTimeEq,
}

impl BytesQuery {
    pub fn for_intrinsic(id: StdIntrinsicId) -> Option<Self> {
        match id.0 {
            pure::BYTES_LEN => Some(Self::Len),
            pure::CRYPTO_SHA256 => Some(Self::Sha256),
            pure::CRYPTO_CONSTANT_TIME_EQ => Some(Self::ConstantTimeEq),
            _ => None,
        }
    }

    pub fn arity(self) -> usize {
        match self {
            Self::Len | Self::Sha256 => 1,
            Self::ConstantTimeEq => 2,
        }
    }

    pub fn evaluate(self, args: &[&[u8]]) -> Result<BuiltinValue, BuiltinError> {
        if args.len() != self.arity() {
            return Err(BuiltinError::ArityMismatch {
                expected: self.arity(),
                actual: args.len(),
            });
        }
        Ok(match self {
            Self::Len => super::ops::len_borrowed(args[0]),
            Self::Sha256 => crate::crypto::sha256_digest_borrowed(args[0]),
            Self::ConstantTimeEq => crate::crypto::constant_time_eq_borrowed(args[0], args[1]),
        })
    }
}
