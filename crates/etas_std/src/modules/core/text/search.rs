use crate::{StdModuleId, StdRegistryBuilder, intrinsic};

use super::{TextIntrinsic, index_param, register_intrinsic, text_error_effect};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    for (name, output, id, summary) in [
        (
            "contains",
            "bool",
            intrinsic::pure::TEXT_CONTAINS,
            "Test exact substring containment.",
        ),
        (
            "starts_with",
            "bool",
            intrinsic::pure::TEXT_STARTS_WITH,
            "Test an exact prefix.",
        ),
        (
            "ends_with",
            "bool",
            intrinsic::pure::TEXT_ENDS_WITH,
            "Test an exact suffix.",
        ),
        (
            "find",
            "Option[usize]",
            intrinsic::pure::TEXT_FIND,
            "Find the first exact match.",
        ),
        (
            "rfind",
            "Option[usize]",
            intrinsic::pure::TEXT_RFIND,
            "Find the last exact match.",
        ),
        (
            "find_all",
            "Iter[(usize, usize)]",
            intrinsic::pure::TEXT_FIND_ALL,
            "Lazily traverse non-overlapping exact matches.",
        ),
        (
            "count",
            "usize",
            intrinsic::pure::TEXT_COUNT,
            "Count non-overlapping exact matches.",
        ),
    ] {
        register_intrinsic(
            builder,
            module,
            &["std", "text"],
            TextIntrinsic {
                name,
                type_params: &[],
                params: &["string", "string"],
                output,
                public_effects: &[],
                id,
                summary,
            },
        );
    }
    let params = [index_param()];
    let effects = [text_error_effect()];
    register_intrinsic(
        builder,
        module,
        &["std", "text"],
        TextIntrinsic {
            name: "find_from",
            type_params: &params,
            params: &["string", "string", "I"],
            output: "Option[usize]",
            public_effects: &effects,
            id: intrinsic::pure::TEXT_FIND_FROM,
            summary: "Find from a checked UTF-8 byte boundary.",
        },
    );
}
