use etas_std::{StdIntrinsicId, intrinsic};

#[derive(Clone, Debug, Default)]
pub struct PureIntrinsicRegistry;

impl PureIntrinsicRegistry {
    pub fn contains(&self, intrinsic: StdIntrinsicId) -> bool {
        if crate::text::is_text_intrinsic(intrinsic) {
            return true;
        }
        matches!(
            intrinsic.0,
            intrinsic::pure::ASSERT
                | intrinsic::pure::ABORT
                | intrinsic::pure::BYTES_LEN
                | intrinsic::pure::HTTP_ENCODE_REQUEST
                | intrinsic::pure::HTTP_DECODE_RESPONSE_HEAD
                | intrinsic::pure::HTTP_DECODE_RESPONSE_HEAD_INCREMENTAL
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
                | intrinsic::pure::HTTP_DECODE_RESPONSE_INCREMENTAL
                | intrinsic::pure::HTTP_DECODE_RESPONSE_HEAD_PREFIX
                | intrinsic::pure::HTTP_DECODE_CHUNK_SIZE_LINE_PREFIX
        )
    }
}
