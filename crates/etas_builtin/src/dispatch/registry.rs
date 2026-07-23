use etas_std::{StdIntrinsicId, intrinsic};

#[derive(Clone, Debug, Default)]
pub struct PureIntrinsicRegistry;

impl PureIntrinsicRegistry {
    pub fn contains(&self, intrinsic: StdIntrinsicId) -> bool {
        matches!(
            intrinsic.0,
            intrinsic::pure::ASSERT
                | intrinsic::pure::ABORT
                | intrinsic::pure::TEXT_TRIM
                | intrinsic::pure::TEXT_CONTAINS
                | intrinsic::pure::TEXT_LOWERCASE
                | intrinsic::pure::TEXT_UPPERCASE
                | intrinsic::pure::TEXT_STARTS_WITH
                | intrinsic::pure::TEXT_ENDS_WITH
                | intrinsic::pure::TEXT_LEN
                | intrinsic::pure::TEXT_LINES
                | intrinsic::pure::TEXT_SPLIT
                | intrinsic::pure::TEXT_JOIN
                | intrinsic::pure::TEXT_TO_STRING_I32
                | intrinsic::pure::TEXT_TO_STRING_USIZE
                | intrinsic::pure::TEXT_PARSE_I32
                | intrinsic::pure::BYTES_LEN
                | intrinsic::pure::HTTP_ENCODE_REQUEST
                | intrinsic::pure::HTTP_DECODE_RESPONSE_HEAD
                | intrinsic::pure::TEXT_UTF8_DECODE
                | intrinsic::pure::TEXT_UTF8_ENCODE
                | intrinsic::pure::CRYPTO_SHA256
                | intrinsic::pure::CRYPTO_CONSTANT_TIME_EQ
                | intrinsic::pure::LIST_LEN
                | intrinsic::pure::LIST_IS_EMPTY
                | intrinsic::pure::MAP_CONTAINS_KEY
                | intrinsic::pure::OPTION_IS_SOME
                | intrinsic::pure::OPTION_IS_NONE
                | intrinsic::pure::OPTION_UNWRAP
                | intrinsic::pure::OPTION_SOME
                | intrinsic::pure::OPTION_NONE
                | intrinsic::pure::RESULT_IS_OK
                | intrinsic::pure::RESULT_IS_ERR
                | intrinsic::pure::RESULT_OK
                | intrinsic::pure::RESULT_ERR
                | intrinsic::pure::RESULT_UNWRAP
                | intrinsic::pure::HTTP_DECODE_RESPONSE
        )
    }
}
