use crate::{StdModuleId, StdRegistryBuilder, intrinsic};

use super::{TextIntrinsic, index_param, register_intrinsic, text_error_effect};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    for (name, params, output, id, summary) in [
        (
            "trim",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_TRIM,
            "Trim Unicode whitespace from both ends.",
        ),
        (
            "trim_start",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_TRIM_START,
            "Trim leading Unicode whitespace.",
        ),
        (
            "trim_end",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_TRIM_END,
            "Trim trailing Unicode whitespace.",
        ),
        (
            "strip_prefix",
            &["string", "string"][..],
            "Option[string]",
            intrinsic::pure::TEXT_STRIP_PREFIX,
            "Remove one exact prefix.",
        ),
        (
            "strip_suffix",
            &["string", "string"][..],
            "Option[string]",
            intrinsic::pure::TEXT_STRIP_SUFFIX,
            "Remove one exact suffix.",
        ),
        (
            "lowercase",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_LOWERCASE,
            "Apply Unicode lowercase mapping.",
        ),
        (
            "uppercase",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_UPPERCASE,
            "Apply Unicode uppercase mapping.",
        ),
        (
            "ascii_lowercase",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_ASCII_LOWERCASE,
            "Map ASCII letters to lowercase.",
        ),
        (
            "ascii_uppercase",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_ASCII_UPPERCASE,
            "Map ASCII letters to uppercase.",
        ),
        (
            "eq_ignore_ascii_case",
            &["string", "string"][..],
            "bool",
            intrinsic::pure::TEXT_EQ_IGNORE_ASCII_CASE,
            "Compare while folding ASCII letters only.",
        ),
        (
            "replace",
            &["string", "string", "string"][..],
            "string",
            intrinsic::pure::TEXT_REPLACE,
            "Replace every non-overlapping exact match.",
        ),
        (
            "replace_n",
            &["string", "string", "string", "usize"][..],
            "string",
            intrinsic::pure::TEXT_REPLACE_N,
            "Replace at most n exact matches.",
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
    let type_params = [index_param()];
    let effects = [text_error_effect()];
    register_intrinsic(
        builder,
        module,
        &["std", "text"],
        TextIntrinsic {
            name: "replace_range",
            type_params: &type_params,
            params: &["string", "Range[I]", "string"],
            output: "string",
            public_effects: &effects,
            id: intrinsic::pure::TEXT_REPLACE_RANGE,
            summary: "Replace a checked UTF-8 byte interval.",
        },
    );
}
