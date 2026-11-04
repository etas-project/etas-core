use crate::{StdLibrarySourceId, StdModuleId, StdRegistryBuilder, intrinsic};

use super::{TextIntrinsic, register_source_intrinsic, text_error_effect};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    for (name, params, output, checked, id, summary) in [
        (
            "concat",
            &["Array[string]"][..],
            "string",
            false,
            intrinsic::pure::TEXT_CONCAT,
            "Concatenate text parts.",
        ),
        (
            "join",
            &["Array[string]", "string"][..],
            "string",
            false,
            intrinsic::pure::TEXT_JOIN,
            "Join text parts with a separator.",
        ),
        (
            "repeat",
            &["string", "usize"][..],
            "string",
            true,
            intrinsic::pure::TEXT_REPEAT,
            "Repeat text with checked output length.",
        ),
        (
            "pad_start",
            &["string", "usize", "char"][..],
            "string",
            false,
            intrinsic::pure::TEXT_PAD_START,
            "Pad the start to a scalar width.",
        ),
        (
            "pad_end",
            &["string", "usize", "char"][..],
            "string",
            false,
            intrinsic::pure::TEXT_PAD_END,
            "Pad the end to a scalar width.",
        ),
        (
            "reverse_chars",
            &["string"][..],
            "string",
            false,
            intrinsic::pure::TEXT_REVERSE_CHARS,
            "Reverse Unicode scalar order.",
        ),
    ] {
        let effects = checked
            .then(text_error_effect)
            .into_iter()
            .collect::<Vec<_>>();
        register_source_intrinsic(
            builder,
            module,
            &["std", "text"],
            TextIntrinsic {
                name,
                type_params: &[],
                params,
                output,
                public_effects: &effects,
                id,
                summary,
            },
            StdLibrarySourceId::TextAssembly,
            name,
        );
    }
}
