use etas_std::{StdIntrinsicId, intrinsic};

use crate::{
    BuiltinError, BuiltinValue, bytes, codec, collections, control, crypto, http, option_result,
    text,
};

pub fn call_pure_intrinsic(
    intrinsic: StdIntrinsicId,
    args: &[BuiltinValue],
) -> Result<BuiltinValue, BuiltinError> {
    match intrinsic.0 {
        intrinsic::pure::ASSERT => control::assert::assert(args),
        intrinsic::pure::ABORT => control::abort::abort(args),
        intrinsic::pure::TEXT_TRIM => text::string::trim(args),
        intrinsic::pure::TEXT_LOWERCASE => text::string::lowercase(args),
        intrinsic::pure::TEXT_UPPERCASE => text::string::uppercase(args),
        intrinsic::pure::TEXT_CONTAINS => text::string::contains(args),
        intrinsic::pure::TEXT_STARTS_WITH => text::string::starts_with(args),
        intrinsic::pure::TEXT_ENDS_WITH => text::string::ends_with(args),
        intrinsic::pure::TEXT_LEN => text::string::len(args),
        intrinsic::pure::TEXT_LINES => text::string::lines(args),
        intrinsic::pure::TEXT_SPLIT => text::string::split(args),
        intrinsic::pure::TEXT_JOIN => text::string::join(args),
        intrinsic::pure::TEXT_TO_STRING_I32 => text::string::to_string_i32(args),
        intrinsic::pure::TEXT_TO_STRING_USIZE => text::string::to_string_usize(args),
        intrinsic::pure::TEXT_PARSE_I32 => text::string::parse_i32(args),
        intrinsic::pure::BYTES_LEN => bytes::ops::len(args),
        intrinsic::pure::LIST_LEN => collections::list::len(args),
        intrinsic::pure::LIST_IS_EMPTY => collections::list::is_empty(args),
        intrinsic::pure::MAP_CONTAINS_KEY => collections::map::contains_key(args),
        intrinsic::pure::HTTP_ENCODE_REQUEST => http::codec::encode_request(args),
        intrinsic::pure::HTTP_DECODE_RESPONSE_HEAD => http::codec::decode_response_head(args),
        intrinsic::pure::HTTP_DECODE_RESPONSE => http::codec::decode_response(args),
        intrinsic::pure::TEXT_UTF8_DECODE => codec::text::utf8_decode(args),
        intrinsic::pure::TEXT_UTF8_ENCODE => codec::text::utf8_encode(args),
        intrinsic::pure::CRYPTO_SHA256 => crypto::sha256_digest(args),
        intrinsic::pure::CRYPTO_CONSTANT_TIME_EQ => crypto::constant_time_eq(args),
        intrinsic::pure::OPTION_IS_SOME => option_result::option::is_some(args),
        intrinsic::pure::OPTION_SOME => option_result::option::some(args),
        intrinsic::pure::OPTION_NONE => option_result::option::none(args),
        intrinsic::pure::OPTION_IS_NONE => option_result::option::is_none(args),
        intrinsic::pure::OPTION_UNWRAP => option_result::option::unwrap(args),
        intrinsic::pure::RESULT_OK => option_result::result::ok(args),
        intrinsic::pure::RESULT_ERR => option_result::result::err(args),
        intrinsic::pure::RESULT_IS_OK => option_result::result::is_ok(args),
        intrinsic::pure::RESULT_IS_ERR => option_result::result::is_err(args),
        intrinsic::pure::RESULT_UNWRAP => option_result::result::unwrap(args),
        _ => Err(BuiltinError::UnsupportedIntrinsic { intrinsic }),
    }
}
