use crate::{StdModuleId, StdRegistryBuilder, intrinsic};

use super::{TextIntrinsic, register_intrinsic};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    for (name, id, summary) in [
        (
            "len",
            intrinsic::pure::TEXT_LEN,
            "Return UTF-8 byte length.",
        ),
        (
            "char_count",
            intrinsic::pure::TEXT_CHAR_COUNT,
            "Return Unicode scalar count.",
        ),
        (
            "is_empty",
            intrinsic::pure::TEXT_IS_EMPTY,
            "Return whether the text has no bytes.",
        ),
        (
            "is_ascii",
            intrinsic::pure::TEXT_IS_ASCII,
            "Return whether every scalar is ASCII.",
        ),
    ] {
        let output = if matches!(name, "is_empty" | "is_ascii") {
            "bool"
        } else {
            "usize"
        };
        register_intrinsic(
            builder,
            module,
            &["std", "text"],
            TextIntrinsic {
                name,
                type_params: &[],
                params: &["string"],
                output,
                public_effects: &[],
                id,
                summary,
            },
        );
    }
}
