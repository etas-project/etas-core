use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicLatentEffect, IntrinsicPurity,
    LoweringHint, StdDecl, StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, TypeDecl,
    TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let option = builder.module(&["std", "option"], "Standard optional value support.");
    let option_type = builder.symbol(
        option,
        "Option",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Option", &["T"], TypeDeclKind::Enum)),
        "Standard option declaration.",
    );
    builder.prelude("Option", option_type);
    let some = transparent_constructor(
        builder,
        option,
        "Some",
        &["T"],
        intrinsic::pure::OPTION_SOME,
        &["std", "option", "Some"],
    );
    builder.prelude("Some", some);
    let none = builder.symbol(
        option,
        "None",
        StdSymbolKind::Constructor,
        StdDecl::Type(TypeDecl::generic("None", &["T"], TypeDeclKind::Enum)),
        "Standard option declaration.",
    );
    builder.prelude("None", none);
    pure_helper(
        builder,
        option,
        "is_some",
        &["Option[T]"],
        "bool",
        intrinsic::pure::OPTION_IS_SOME,
        &["std", "option", "is_some"],
    );
    pure_helper(
        builder,
        option,
        "is_none",
        &["Option[T]"],
        "bool",
        intrinsic::pure::OPTION_IS_NONE,
        &["std", "option", "is_none"],
    );
    pure_helper(
        builder,
        option,
        "unwrap",
        &["Option[T]"],
        "T",
        intrinsic::pure::OPTION_UNWRAP,
        &["std", "option", "unwrap"],
    );

    let result = builder.module(&["std", "result"], "Standard result/error value support.");
    let result_type = builder.symbol(
        result,
        "Result",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Result", &["T", "E"], TypeDeclKind::Enum)),
        "Standard result declaration.",
    );
    builder.prelude("Result", result_type);
    let ok = transparent_constructor(
        builder,
        result,
        "Ok",
        &["T", "E"],
        intrinsic::pure::RESULT_OK,
        &["std", "result", "Ok"],
    );
    builder.prelude("Ok", ok);
    let err = transparent_constructor(
        builder,
        result,
        "Err",
        &["T", "E"],
        intrinsic::pure::RESULT_ERR,
        &["std", "result", "Err"],
    );
    builder.prelude("Err", err);
    pure_helper(
        builder,
        result,
        "unwrap",
        &["Result[T, E]"],
        "T",
        intrinsic::pure::OPTION_UNWRAP,
        &["std", "result", "unwrap"],
    );
    pure_helper(
        builder,
        result,
        "is_ok",
        &["Result[T, E]"],
        "bool",
        intrinsic::pure::RESULT_IS_OK,
        &["std", "result", "is_ok"],
    );
    pure_helper(
        builder,
        result,
        "is_err",
        &["Result[T, E]"],
        "bool",
        intrinsic::pure::RESULT_IS_ERR,
        &["std", "result", "is_err"],
    );
}

fn pure_helper(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    params: &[&str],
    output: &str,
    id: u32,
    path: &[&str],
) -> crate::StdSymbolId {
    builder.symbol_with_intrinsic(
        module,
        name,
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure(name, params, output)),
        "Pure standard helper.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(id),
            qualified_path: path.iter().map(|segment| (*segment).to_owned()).collect(),
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::PureKernel,
            lowering: LoweringHint::PureBuiltin,
            latent_effect: if name == "unwrap" {
                IntrinsicLatentEffect::TransparentFirstArg
            } else {
                IntrinsicLatentEffect::None
            },
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    )
}

fn transparent_constructor(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    params: &[&str],
    id: u32,
    path: &[&str],
) -> crate::StdSymbolId {
    builder.symbol_with_intrinsic(
        module,
        name,
        StdSymbolKind::Constructor,
        StdDecl::Type(TypeDecl::generic(name, params, TypeDeclKind::Enum)),
        "Standard value constructor.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(id),
            qualified_path: path.iter().map(|segment| (*segment).to_owned()).collect(),
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::PureKernel,
            lowering: LoweringHint::PureBuiltin,
            latent_effect: IntrinsicLatentEffect::TransparentFirstArg,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    )
}
