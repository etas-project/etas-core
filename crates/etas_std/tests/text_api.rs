use std::collections::BTreeSet;

use etas_std::{
    IntrinsicCompletionSemantics, IntrinsicDispatch, StdDecl, StdEffectRef, StdGenericParam,
    StdImplementationDeterminism, StdImplementationRef, StdIntrinsicId, StdIterableItem,
    StdLibrarySourceId, StdRegistry, StdSpecRef, StdSymbolKind, StdType, StdTypeConstruction,
    TypeDeclKind, intrinsic, intrinsic_completion_semantics, standard_registry,
};

fn assert_flow(
    registry: &StdRegistry,
    module: &[&str],
    name: &str,
    params: &[&str],
    output: &str,
    id: u32,
    checks: (bool, bool),
) {
    let (index_param, raises_text_error) = checks;
    let mut path = module
        .iter()
        .map(|part| (*part).to_owned())
        .collect::<Vec<_>>();
    path.push(name.to_owned());
    let symbol = registry
        .lookup_qualified(&path)
        .unwrap_or_else(|| panic!("missing standard flow {}", path.join(".")));
    assert_eq!(symbol.kind, StdSymbolKind::Flow, "{}", path.join("."));
    let StdDecl::Flow(flow) = &symbol.decl else {
        panic!("{} is not a flow declaration", path.join("."));
    };
    assert_eq!(
        flow.params,
        params
            .iter()
            .map(|ty| StdType::parse(ty))
            .collect::<Vec<_>>(),
        "{} parameters",
        path.join(".")
    );
    assert_eq!(
        flow.output,
        StdType::parse(output),
        "{} output",
        path.join(".")
    );
    if index_param {
        assert_eq!(
            flow.type_params,
            [StdGenericParam::bounded(
                "I",
                &[StdSpecRef::new(&["std", "core", "Index"])],
            )],
            "{} generic parameters",
            path.join(".")
        );
    } else {
        assert!(
            flow.type_params.is_empty(),
            "{} generic parameters",
            path.join(".")
        );
    }
    let effects = raises_text_error
        .then(|| StdEffectRef::typed(&["Error"], StdType::parse("std.text.TextError")))
        .into_iter()
        .collect::<Vec<_>>();
    assert_eq!(
        flow.public_effects,
        effects,
        "{} effect row",
        path.join(".")
    );
    assert!(
        flow.requested_actions.is_empty(),
        "{} requested actions",
        path.join(".")
    );

    let descriptor = symbol
        .intrinsic
        .as_ref()
        .unwrap_or_else(|| panic!("{} has no intrinsic descriptor", path.join(".")));
    assert_eq!(
        descriptor.id,
        StdIntrinsicId(id),
        "{} intrinsic",
        path.join(".")
    );
    assert_eq!(descriptor.qualified_path, path, "descriptor path");
    assert_eq!(descriptor.dispatch, IntrinsicDispatch::PureKernel);
    assert_eq!(registry.intrinsic(StdIntrinsicId(id)), Some(descriptor));
    assert_eq!(
        intrinsic_completion_semantics(StdIntrinsicId(id)),
        Some(if raises_text_error {
            IntrinsicCompletionSemantics::ReturnsOrRaisesDeclaredErrors
        } else {
            IntrinsicCompletionSemantics::Returns
        }),
        "{} completion semantics",
        descriptor.qualified_path.join(".")
    );
}

fn flow_exports(registry: &StdRegistry, path: &[&str]) -> BTreeSet<String> {
    let module = registry
        .modules()
        .find(|module| {
            module.path
                == path
                    .iter()
                    .map(|part| (*part).to_owned())
                    .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| panic!("missing module {}", path.join(".")));
    registry
        .module_exports(module.id)
        .filter(|(_, symbol)| symbol.kind == StdSymbolKind::Flow)
        .map(|(name, _)| name.to_owned())
        .collect()
}

#[test]
fn text_and_char_registry_match_the_complete_v1_contract() {
    let registry = standard_registry();
    let text = ["std", "text"];
    let unicode = ["std", "text", "unicode"];
    let char_module = ["std", "char"];
    let mut text_names = BTreeSet::new();
    let mut unicode_names = BTreeSet::new();
    let mut char_names = BTreeSet::new();

    for (name, params, output, id) in [
        ("len", &["string"][..], "usize", intrinsic::pure::TEXT_LEN),
        (
            "char_count",
            &["string"][..],
            "usize",
            intrinsic::pure::TEXT_CHAR_COUNT,
        ),
        (
            "is_empty",
            &["string"][..],
            "bool",
            intrinsic::pure::TEXT_IS_EMPTY,
        ),
        (
            "is_ascii",
            &["string"][..],
            "bool",
            intrinsic::pure::TEXT_IS_ASCII,
        ),
        (
            "chars",
            &["string"][..],
            "Iter[char]",
            intrinsic::pure::TEXT_CHARS,
        ),
        (
            "char_indices",
            &["string"][..],
            "Iter[(usize, char)]",
            intrinsic::pure::TEXT_CHAR_INDICES,
        ),
        (
            "lines",
            &["string"][..],
            "Iter[string]",
            intrinsic::pure::TEXT_LINES,
        ),
        (
            "contains",
            &["string", "string"][..],
            "bool",
            intrinsic::pure::TEXT_CONTAINS,
        ),
        (
            "starts_with",
            &["string", "string"][..],
            "bool",
            intrinsic::pure::TEXT_STARTS_WITH,
        ),
        (
            "ends_with",
            &["string", "string"][..],
            "bool",
            intrinsic::pure::TEXT_ENDS_WITH,
        ),
        (
            "find",
            &["string", "string"][..],
            "Option[usize]",
            intrinsic::pure::TEXT_FIND,
        ),
        (
            "rfind",
            &["string", "string"][..],
            "Option[usize]",
            intrinsic::pure::TEXT_RFIND,
        ),
        (
            "find_all",
            &["string", "string"][..],
            "Iter[(usize, usize)]",
            intrinsic::pure::TEXT_FIND_ALL,
        ),
        (
            "count",
            &["string", "string"][..],
            "usize",
            intrinsic::pure::TEXT_COUNT,
        ),
        (
            "split",
            &["string", "string"][..],
            "Array[string]",
            intrinsic::pure::TEXT_SPLIT,
        ),
        (
            "split_iter",
            &["string", "string"][..],
            "Iter[string]",
            intrinsic::pure::TEXT_SPLIT_ITER,
        ),
        (
            "split_once",
            &["string", "string"][..],
            "Option[(string, string)]",
            intrinsic::pure::TEXT_SPLIT_ONCE,
        ),
        (
            "rsplit_once",
            &["string", "string"][..],
            "Option[(string, string)]",
            intrinsic::pure::TEXT_RSPLIT_ONCE,
        ),
        (
            "splitn",
            &["string", "string", "usize"][..],
            "Array[string]",
            intrinsic::pure::TEXT_SPLITN,
        ),
        (
            "split_whitespace",
            &["string"][..],
            "Iter[string]",
            intrinsic::pure::TEXT_SPLIT_WHITESPACE,
        ),
        (
            "trim",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_TRIM,
        ),
        (
            "trim_start",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_TRIM_START,
        ),
        (
            "trim_end",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_TRIM_END,
        ),
        (
            "strip_prefix",
            &["string", "string"][..],
            "Option[string]",
            intrinsic::pure::TEXT_STRIP_PREFIX,
        ),
        (
            "strip_suffix",
            &["string", "string"][..],
            "Option[string]",
            intrinsic::pure::TEXT_STRIP_SUFFIX,
        ),
        (
            "lowercase",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_LOWERCASE,
        ),
        (
            "uppercase",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_UPPERCASE,
        ),
        (
            "ascii_lowercase",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_ASCII_LOWERCASE,
        ),
        (
            "ascii_uppercase",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_ASCII_UPPERCASE,
        ),
        (
            "eq_ignore_ascii_case",
            &["string", "string"][..],
            "bool",
            intrinsic::pure::TEXT_EQ_IGNORE_ASCII_CASE,
        ),
        (
            "replace",
            &["string", "string", "string"][..],
            "string",
            intrinsic::pure::TEXT_REPLACE,
        ),
        (
            "replace_n",
            &["string", "string", "string", "usize"][..],
            "string",
            intrinsic::pure::TEXT_REPLACE_N,
        ),
        (
            "concat",
            &["Array[string]"][..],
            "string",
            intrinsic::pure::TEXT_CONCAT,
        ),
        (
            "join",
            &["Array[string]", "string"][..],
            "string",
            intrinsic::pure::TEXT_JOIN,
        ),
        (
            "pad_start",
            &["string", "usize", "char"][..],
            "string",
            intrinsic::pure::TEXT_PAD_START,
        ),
        (
            "pad_end",
            &["string", "usize", "char"][..],
            "string",
            intrinsic::pure::TEXT_PAD_END,
        ),
        (
            "reverse_chars",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_REVERSE_CHARS,
        ),
        (
            "format",
            &["string", "Array[string]"][..],
            "Result[string, std.text.TextError]",
            intrinsic::pure::TEXT_FORMAT,
        ),
        (
            "parse_bool",
            &["string"][..],
            "Result[bool, std.text.TextError]",
            intrinsic::pure::TEXT_PARSE_BOOL,
        ),
        (
            "from_bool",
            &["bool"][..],
            "string",
            intrinsic::pure::TEXT_FROM_BOOL,
        ),
        (
            "from_char",
            &["char"][..],
            "string",
            intrinsic::pure::TEXT_FROM_CHAR,
        ),
    ] {
        assert_flow(&registry, &text, name, params, output, id, (false, false));
        assert!(text_names.insert(name.to_owned()));
    }

    for (name, params, output, id, raises) in [
        (
            "find_from",
            &["string", "string", "I"][..],
            "Option[usize]",
            intrinsic::pure::TEXT_FIND_FROM,
            true,
        ),
        (
            "char_get",
            &["string", "I"][..],
            "Option[char]",
            intrinsic::pure::TEXT_CHAR_GET,
            false,
        ),
        (
            "byte_slice",
            &["string", "Range[I]"][..],
            "string",
            intrinsic::pure::TEXT_BYTE_SLICE,
            true,
        ),
        (
            "char_slice",
            &["string", "Range[I]"][..],
            "string",
            intrinsic::pure::TEXT_CHAR_SLICE,
            true,
        ),
        (
            "get_byte_slice",
            &["string", "Range[I]"][..],
            "Option[string]",
            intrinsic::pure::TEXT_GET_BYTE_SLICE,
            false,
        ),
        (
            "get_char_slice",
            &["string", "Range[I]"][..],
            "Option[string]",
            intrinsic::pure::TEXT_GET_CHAR_SLICE,
            false,
        ),
        (
            "is_char_boundary",
            &["string", "I"][..],
            "bool",
            intrinsic::pure::TEXT_IS_CHAR_BOUNDARY,
            false,
        ),
        (
            "replace_range",
            &["string", "Range[I]", "string"][..],
            "string",
            intrinsic::pure::TEXT_REPLACE_RANGE,
            true,
        ),
    ] {
        assert_flow(&registry, &text, name, params, output, id, (true, raises));
        assert!(text_names.insert(name.to_owned()));
    }
    assert_flow(
        &registry,
        &text,
        "repeat",
        &["string", "usize"],
        "string",
        intrinsic::pure::TEXT_REPEAT,
        (false, true),
    );
    assert!(text_names.insert("repeat".into()));

    for (index, ty) in [
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
        "f32", "f64",
    ]
    .into_iter()
    .enumerate()
    {
        let parse = format!("parse_{ty}");
        let render = format!("from_{ty}");
        assert_flow(
            &registry,
            &text,
            &parse,
            &["string"],
            &format!("Result[{ty}, std.text.TextError]"),
            intrinsic::pure::TEXT_PARSE_I8 + index as u32,
            (false, false),
        );
        assert_flow(
            &registry,
            &text,
            &render,
            &[ty],
            "string",
            intrinsic::pure::TEXT_FROM_I8 + index as u32,
            (false, false),
        );
        assert!(text_names.insert(parse));
        assert!(text_names.insert(render));
    }

    for (name, params, output, id) in [
        (
            "graphemes",
            &["string"][..],
            "Iter[string]",
            intrinsic::pure::TEXT_GRAPHEMES,
        ),
        (
            "grapheme_indices",
            &["string"][..],
            "Iter[(usize, string)]",
            intrinsic::pure::TEXT_GRAPHEME_INDICES,
        ),
        (
            "grapheme_count",
            &["string"][..],
            "usize",
            intrinsic::pure::TEXT_GRAPHEME_COUNT,
        ),
        (
            "reverse_graphemes",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_REVERSE_GRAPHEMES,
        ),
        (
            "normalize",
            &["string", "std.text.unicode.Normalization"][..],
            "string",
            intrinsic::pure::TEXT_NORMALIZE,
        ),
        (
            "is_normalized",
            &["string", "std.text.unicode.Normalization"][..],
            "bool",
            intrinsic::pure::TEXT_IS_NORMALIZED,
        ),
        (
            "casefold",
            &["string"][..],
            "string",
            intrinsic::pure::TEXT_CASEFOLD,
        ),
    ] {
        assert_flow(
            &registry,
            &unicode,
            name,
            params,
            output,
            id,
            (false, false),
        );
        assert!(unicode_names.insert(name.to_owned()));
    }
    assert_flow(
        &registry,
        &unicode,
        "grapheme_slice",
        &["string", "Range[I]"],
        "string",
        intrinsic::pure::TEXT_GRAPHEME_SLICE,
        (true, true),
    );
    assert!(unicode_names.insert("grapheme_slice".into()));

    for (name, params, output, id) in [
        (
            "is_whitespace",
            &["char"][..],
            "bool",
            intrinsic::pure::CHAR_IS_WHITESPACE,
        ),
        (
            "is_alphabetic",
            &["char"][..],
            "bool",
            intrinsic::pure::CHAR_IS_ALPHABETIC,
        ),
        (
            "is_alphanumeric",
            &["char"][..],
            "bool",
            intrinsic::pure::CHAR_IS_ALPHANUMERIC,
        ),
        (
            "is_control",
            &["char"][..],
            "bool",
            intrinsic::pure::CHAR_IS_CONTROL,
        ),
        (
            "is_ascii_digit",
            &["char"][..],
            "bool",
            intrinsic::pure::CHAR_IS_ASCII_DIGIT,
        ),
        ("to_u32", &["char"][..], "u32", intrinsic::pure::CHAR_TO_U32),
        (
            "from_u32",
            &["u32"][..],
            "Option[char]",
            intrinsic::pure::CHAR_FROM_U32,
        ),
    ] {
        assert_flow(
            &registry,
            &char_module,
            name,
            params,
            output,
            id,
            (false, false),
        );
        assert!(char_names.insert(name.to_owned()));
    }

    assert_eq!(flow_exports(&registry, &text), text_names);
    assert_eq!(flow_exports(&registry, &unicode), unicode_names);
    assert_eq!(flow_exports(&registry, &char_module), char_names);

    let text_error = registry
        .lookup_qualified(&["std", "text", "TextError"])
        .unwrap();
    let StdDecl::Type(text_error_decl) = &text_error.decl else {
        panic!("TextError declaration");
    };
    assert_eq!(text_error_decl.kind, TypeDeclKind::Enum);
    assert_eq!(
        registry
            .enum_constructors(text_error.id)
            .map(|constructor| constructor.name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "InvalidIndex",
            "InvalidRange",
            "NotCharBoundary",
            "ParseFailure",
            "NumericOverflow",
            "LengthOverflow",
            "InvalidFormat",
        ])
    );

    let normalization = registry
        .lookup_qualified(&["std", "text", "unicode", "Normalization"])
        .unwrap();
    assert_eq!(
        registry
            .enum_constructors(normalization.id)
            .map(|constructor| constructor.name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["NFC", "NFD", "NFKC", "NFKD"])
    );

    let iter = registry
        .lookup_qualified(&["std", "collections", "Iter"])
        .unwrap();
    let StdDecl::Type(iter) = &iter.decl else {
        panic!("Iter declaration");
    };
    assert_eq!(iter.params.len(), 1);
    assert_eq!(
        iter.iterable_item,
        Some(StdIterableItem::Element { parameter: 0 })
    );
    assert_eq!(iter.construction, StdTypeConstruction::IntrinsicOnly);

    for (name, source) in [
        ("concat", StdLibrarySourceId::TextAssembly),
        ("join", StdLibrarySourceId::TextAssembly),
        ("repeat", StdLibrarySourceId::TextAssembly),
        ("pad_start", StdLibrarySourceId::TextAssembly),
        ("pad_end", StdLibrarySourceId::TextAssembly),
        ("reverse_chars", StdLibrarySourceId::TextAssembly),
        ("format", StdLibrarySourceId::TextFormat),
    ] {
        let symbol = registry.lookup_qualified(&["std", "text", name]).unwrap();
        let StdImplementationRef::Source {
            target,
            optimized_intrinsic,
        } = symbol.implementation.unwrap()
        else {
            panic!("std.text.{name} is not linked to bundled source");
        };
        assert_eq!(target.source, source);
        assert_eq!(target.item, name);
        assert_eq!(
            target.determinism,
            StdImplementationDeterminism::Deterministic
        );
        assert_eq!(
            optimized_intrinsic,
            symbol.intrinsic.as_ref().map(|value| value.id)
        );
    }

    for name in [
        "to_string_i32",
        "to_string_usize",
        "ParseError",
        "scalar_at",
    ] {
        assert!(registry.lookup_qualified(&["std", "text", name]).is_none());
    }
    for retired in [16, 19, 20, 21] {
        assert!(registry.intrinsic(StdIntrinsicId(retired)).is_none());
        assert_eq!(
            intrinsic_completion_semantics(StdIntrinsicId(retired)),
            None
        );
    }
}
