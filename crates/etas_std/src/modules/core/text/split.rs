use crate::{StdModuleId, StdRegistryBuilder, intrinsic};

use super::{TextIntrinsic, register_intrinsic};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    for (name, params, output, id, summary) in [
        (
            "split",
            &["string", "string"][..],
            "Array[string]",
            intrinsic::pure::TEXT_SPLIT,
            "Eager exact split preserving empty segments.",
        ),
        (
            "split_iter",
            &["string", "string"][..],
            "Iter[string]",
            intrinsic::pure::TEXT_SPLIT_ITER,
            "Lazily split exact separators.",
        ),
        (
            "split_once",
            &["string", "string"][..],
            "Option[(string, string)]",
            intrinsic::pure::TEXT_SPLIT_ONCE,
            "Split around the first exact match.",
        ),
        (
            "rsplit_once",
            &["string", "string"][..],
            "Option[(string, string)]",
            intrinsic::pure::TEXT_RSPLIT_ONCE,
            "Split around the last exact match.",
        ),
        (
            "splitn",
            &["string", "string", "usize"][..],
            "Array[string]",
            intrinsic::pure::TEXT_SPLITN,
            "Split into at most the requested number of segments.",
        ),
        (
            "split_whitespace",
            &["string"][..],
            "Iter[string]",
            intrinsic::pure::TEXT_SPLIT_WHITESPACE,
            "Lazily split Unicode White_Space runs.",
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
