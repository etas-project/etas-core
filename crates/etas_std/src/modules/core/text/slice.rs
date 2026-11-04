use crate::{StdModuleId, StdRegistryBuilder, intrinsic};

use super::{TextIntrinsic, index_param, register_intrinsic, text_error_effect};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    for (name, params, output, checked, id, summary) in [
        (
            "char_get",
            &["string", "I"][..],
            "Option[char]",
            false,
            intrinsic::pure::TEXT_CHAR_GET,
            "Return a Unicode scalar by ordinal.",
        ),
        (
            "byte_slice",
            &["string", "Range[I]"][..],
            "string",
            true,
            intrinsic::pure::TEXT_BYTE_SLICE,
            "Return a checked UTF-8 byte interval.",
        ),
        (
            "char_slice",
            &["string", "Range[I]"][..],
            "string",
            true,
            intrinsic::pure::TEXT_CHAR_SLICE,
            "Return a checked Unicode scalar interval.",
        ),
        (
            "get_byte_slice",
            &["string", "Range[I]"][..],
            "Option[string]",
            false,
            intrinsic::pure::TEXT_GET_BYTE_SLICE,
            "Return an optional UTF-8 byte interval.",
        ),
        (
            "get_char_slice",
            &["string", "Range[I]"][..],
            "Option[string]",
            false,
            intrinsic::pure::TEXT_GET_CHAR_SLICE,
            "Return an optional Unicode scalar interval.",
        ),
        (
            "is_char_boundary",
            &["string", "I"][..],
            "bool",
            false,
            intrinsic::pure::TEXT_IS_CHAR_BOUNDARY,
            "Test a UTF-8 scalar boundary.",
        ),
    ] {
        let type_params = [index_param()];
        let effects = checked
            .then(text_error_effect)
            .into_iter()
            .collect::<Vec<_>>();
        register_intrinsic(
            builder,
            module,
            &["std", "text"],
            TextIntrinsic {
                name,
                type_params: &type_params,
                params,
                output,
                public_effects: &effects,
                id,
                summary,
            },
        );
    }
}
