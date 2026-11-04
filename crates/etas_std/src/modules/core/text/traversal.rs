use crate::{StdModuleId, StdRegistryBuilder, intrinsic};

use super::{TextIntrinsic, register_intrinsic};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    for (name, output, id, summary) in [
        (
            "chars",
            "Iter[char]",
            intrinsic::pure::TEXT_CHARS,
            "Lazily traverse Unicode scalars.",
        ),
        (
            "char_indices",
            "Iter[(usize, char)]",
            intrinsic::pure::TEXT_CHAR_INDICES,
            "Lazily traverse byte positions and Unicode scalars.",
        ),
        (
            "lines",
            "Iter[string]",
            intrinsic::pure::TEXT_LINES,
            "Lazily traverse LF or CRLF-delimited lines.",
        ),
    ] {
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
