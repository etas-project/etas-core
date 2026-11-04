use crate::{
    FlowDecl, StdDecl, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, intrinsic,
};

use super::{TextIntrinsic, index_param, register_intrinsic, text_error_effect};

pub(super) fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "text", "unicode"],
        "Pinned Unicode text algorithms.",
    );
    let owner = builder.symbol(
        module,
        "Normalization",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Normalization", &[], TypeDeclKind::Enum)),
        "A Unicode normalization form.",
    );
    for name in ["NFC", "NFD", "NFKC", "NFKD"] {
        let constructor = builder
            .enum_constructor(
                owner,
                FlowDecl::pure(name, &[], "std.text.unicode.Normalization"),
                "Unicode normalization form.",
            )
            .expect("Normalization owner is registered");
        builder
            .re_export(module, name, constructor)
            .expect("normalization constructor export is unique");
    }
    for (name, params, output, id, summary) in [
        (
            "graphemes",
            &["string"][..],
            "Iter[string]",
            intrinsic::pure::TEXT_GRAPHEMES,
            "Lazily traverse extended grapheme clusters.",
        ),
        (
            "grapheme_indices",
            &["string"][..],
            "Iter[(usize, string)]",
            intrinsic::pure::TEXT_GRAPHEME_INDICES,
            "Lazily traverse cluster byte positions.",
        ),
        (
            "grapheme_count",
            &["string"][..],
            "usize",
            intrinsic::pure::TEXT_GRAPHEME_COUNT,
            "Count extended grapheme clusters.",
        ),
        (
            "reverse_graphemes",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_REVERSE_GRAPHEMES,
            "Reverse extended grapheme clusters.",
        ),
        (
            "normalize",
            &["string", "std.text.unicode.Normalization"][..],
            "string",
            intrinsic::pure::TEXT_NORMALIZE,
            "Normalize with pinned Unicode data.",
        ),
        (
            "is_normalized",
            &["string", "std.text.unicode.Normalization"][..],
            "bool",
            intrinsic::pure::TEXT_IS_NORMALIZED,
            "Test pinned Unicode normalization.",
        ),
        (
            "casefold",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_CASEFOLD,
            "Apply Unicode default full case folding.",
        ),
    ] {
        register_intrinsic(
            builder,
            module,
            &["std", "text", "unicode"],
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
        &["std", "text", "unicode"],
        TextIntrinsic {
            name: "grapheme_slice",
            type_params: &type_params,
            params: &["string", "Range[I]"],
            output: "string",
            public_effects: &effects,
            id: intrinsic::pure::TEXT_GRAPHEME_SLICE,
            summary: "Return a checked extended-grapheme interval.",
        },
    );
}
