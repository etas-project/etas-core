use crate::{StdLibrarySourceId, StdModuleId, StdRegistryBuilder, intrinsic};

use super::{TextIntrinsic, register_source_intrinsic};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    register_source_intrinsic(
        builder,
        module,
        &["std", "text"],
        TextIntrinsic {
            name: "format",
            type_params: &[],
            params: &["string", "Array[string]"],
            output: "Result[string, std.text.TextError]",
            public_effects: &[],
            id: intrinsic::pure::TEXT_FORMAT,
            summary: "Apply checked positional text formatting.",
        },
        StdLibrarySourceId::TextFormat,
        "format",
    );
}
