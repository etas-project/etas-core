mod assembly;
mod edit;
mod errors;
mod format;
mod numeric;
mod query;
mod search;
mod slice;
mod split;
mod traversal;
mod unicode;

use crate::registry::builder::StdSymbolImplementation;
use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicLatentEffect, IntrinsicPurity,
    LoweringHint, StdDecl, StdEffectRef, StdGenericParam, StdImplementationDeterminism,
    StdImplementationRef, StdIntrinsicId, StdLibrarySourceId, StdModuleId, StdRegistryBuilder,
    StdSymbolKind, StdType,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "text"], "Deterministic UTF-8 text operations.");
    errors::register(builder, module);
    query::register(builder, module);
    traversal::register(builder, module);
    search::register(builder, module);
    split::register(builder, module);
    slice::register(builder, module);
    edit::register(builder, module);
    assembly::register(builder, module);
    numeric::register(builder, module);
    format::register(builder, module);
    unicode::register(builder);
}

pub(super) struct TextIntrinsic<'a> {
    pub name: &'a str,
    pub type_params: &'a [StdGenericParam],
    pub params: &'a [&'a str],
    pub output: &'a str,
    pub public_effects: &'a [StdEffectRef],
    pub id: u32,
    pub summary: &'a str,
}

pub(super) fn register_intrinsic(
    builder: &mut StdRegistryBuilder,
    module: StdModuleId,
    module_path: &[&str],
    registration: TextIntrinsic<'_>,
) {
    register_intrinsic_impl(builder, module, module_path, registration, None);
}

pub(super) fn register_source_intrinsic(
    builder: &mut StdRegistryBuilder,
    module: StdModuleId,
    module_path: &[&str],
    registration: TextIntrinsic<'_>,
    source: StdLibrarySourceId,
    source_item: &'static str,
) {
    let intrinsic = StdIntrinsicId(registration.id);
    let implementation = StdImplementationRef::source(
        source,
        source_item,
        StdImplementationDeterminism::Deterministic,
        Some(intrinsic),
    );
    register_intrinsic_impl(
        builder,
        module,
        module_path,
        registration,
        Some(implementation),
    );
}

fn register_intrinsic_impl(
    builder: &mut StdRegistryBuilder,
    module: StdModuleId,
    module_path: &[&str],
    registration: TextIntrinsic<'_>,
    implementation: Option<StdImplementationRef>,
) {
    let mut qualified_path = module_path
        .iter()
        .map(|segment| (*segment).to_owned())
        .collect::<Vec<_>>();
    qualified_path.push(registration.name.to_owned());
    let declaration = StdDecl::Flow(FlowDecl::with_type_params_actions(
        registration.name,
        registration.type_params,
        registration.params,
        registration.output,
        registration.public_effects,
        &[],
    ));
    let intrinsic = Some(IntrinsicDescriptor {
        id: StdIntrinsicId(registration.id),
        qualified_path,
        purity: IntrinsicPurity::Pure,
        dispatch: IntrinsicDispatch::PureKernel,
        lowering: LoweringHint::PureBuiltin,
        latent_effect: IntrinsicLatentEffect::None,
        memory_access: crate::IntrinsicMemoryAccess::None,
        runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
    });
    match implementation {
        Some(implementation) => {
            builder.symbol_with_source_implementation(
                module,
                registration.name,
                StdSymbolKind::Flow,
                declaration,
                registration.summary,
                StdSymbolImplementation {
                    intrinsic,
                    implementation,
                },
            );
        }
        None => {
            builder.symbol_with_intrinsic(
                module,
                registration.name,
                StdSymbolKind::Flow,
                declaration,
                registration.summary,
                intrinsic,
            );
        }
    }
}

pub(super) fn index_param() -> StdGenericParam {
    StdGenericParam::bounded("I", &[crate::StdSpecRef::new(&["std", "core", "Index"])])
}

pub(super) fn text_error_effect() -> StdEffectRef {
    StdEffectRef::typed(&["Error"], StdType::parse("std.text.TextError"))
}
