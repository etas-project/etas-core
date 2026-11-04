use crate::{StdModuleId, StdRegistryBuilder, intrinsic};

use super::{TextIntrinsic, register_intrinsic};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    let numeric = [
        (
            "i8",
            intrinsic::pure::TEXT_PARSE_I8,
            intrinsic::pure::TEXT_FROM_I8,
        ),
        (
            "i16",
            intrinsic::pure::TEXT_PARSE_I16,
            intrinsic::pure::TEXT_FROM_I16,
        ),
        (
            "i32",
            intrinsic::pure::TEXT_PARSE_I32,
            intrinsic::pure::TEXT_FROM_I32,
        ),
        (
            "i64",
            intrinsic::pure::TEXT_PARSE_I64,
            intrinsic::pure::TEXT_FROM_I64,
        ),
        (
            "i128",
            intrinsic::pure::TEXT_PARSE_I128,
            intrinsic::pure::TEXT_FROM_I128,
        ),
        (
            "isize",
            intrinsic::pure::TEXT_PARSE_ISIZE,
            intrinsic::pure::TEXT_FROM_ISIZE,
        ),
        (
            "u8",
            intrinsic::pure::TEXT_PARSE_U8,
            intrinsic::pure::TEXT_FROM_U8,
        ),
        (
            "u16",
            intrinsic::pure::TEXT_PARSE_U16,
            intrinsic::pure::TEXT_FROM_U16,
        ),
        (
            "u32",
            intrinsic::pure::TEXT_PARSE_U32,
            intrinsic::pure::TEXT_FROM_U32,
        ),
        (
            "u64",
            intrinsic::pure::TEXT_PARSE_U64,
            intrinsic::pure::TEXT_FROM_U64,
        ),
        (
            "u128",
            intrinsic::pure::TEXT_PARSE_U128,
            intrinsic::pure::TEXT_FROM_U128,
        ),
        (
            "usize",
            intrinsic::pure::TEXT_PARSE_USIZE,
            intrinsic::pure::TEXT_FROM_USIZE,
        ),
        (
            "f32",
            intrinsic::pure::TEXT_PARSE_F32,
            intrinsic::pure::TEXT_FROM_F32,
        ),
        (
            "f64",
            intrinsic::pure::TEXT_PARSE_F64,
            intrinsic::pure::TEXT_FROM_F64,
        ),
    ];
    for (ty, parse_id, from_id) in numeric {
        let parse_name = format!("parse_{ty}");
        register_intrinsic(
            builder,
            module,
            &["std", "text"],
            TextIntrinsic {
                name: &parse_name,
                type_params: &[],
                params: &["string"],
                output: &format!("Result[{ty}, std.text.TextError]"),
                public_effects: &[],
                id: parse_id,
                summary: "Parse a complete locale-independent numeric string.",
            },
        );
        let from_name = format!("from_{ty}");
        register_intrinsic(
            builder,
            module,
            &["std", "text"],
            TextIntrinsic {
                name: &from_name,
                type_params: &[],
                params: &[ty],
                output: "string",
                public_effects: &[],
                id: from_id,
                summary: "Render a numeric value without narrowing conversion.",
            },
        );
    }
    for (name, params, output, id, summary) in [
        (
            "parse_bool",
            &["string"][..],
            "Result[bool, std.text.TextError]",
            intrinsic::pure::TEXT_PARSE_BOOL,
            "Parse exactly true or false.",
        ),
        (
            "from_bool",
            &["bool"][..],
            "string",
            intrinsic::pure::TEXT_FROM_BOOL,
            "Render a boolean.",
        ),
        (
            "from_char",
            &["char"][..],
            "string",
            intrinsic::pure::TEXT_FROM_CHAR,
            "Render one Unicode scalar.",
        ),
    ] {
        register_intrinsic(
            builder,
            module,
            &["std", "text"],
            TextIntrinsic {
                name,
                type_params: &[],
                params,
                output,
                public_effects: &[],
                id,
                summary,
            },
        );
    }
}
