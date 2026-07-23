use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdModuleId, StdRegistryBuilder, StdSymbolKind, TypeParam,
};

pub(crate) struct IntrinsicFlowRegistration<'a> {
    pub name: &'a str,
    pub type_params: &'a [TypeParam],
    pub params: &'a [&'a str],
    pub output: &'a str,
    pub public_effects: &'a [&'a str],
    pub requested_actions: &'a [&'a str],
    pub intrinsic_id: u32,
    pub summary: &'a str,
    pub purity: IntrinsicPurity,
    pub dispatch: IntrinsicDispatch,
    pub lowering: LoweringHint,
}

pub(crate) fn register_intrinsic_flow(
    builder: &mut StdRegistryBuilder,
    module: StdModuleId,
    module_path: &[&str],
    registration: IntrinsicFlowRegistration<'_>,
) {
    let mut qualified_path = module_path
        .iter()
        .map(|segment| (*segment).to_owned())
        .collect::<Vec<_>>();
    qualified_path.push(registration.name.to_owned());
    builder.symbol_with_intrinsic(
        module,
        registration.name,
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_type_params_actions(
            registration.name,
            registration.type_params,
            registration.params,
            registration.output,
            registration.public_effects,
            registration.requested_actions,
        )),
        registration.summary,
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(registration.intrinsic_id),
            qualified_path,
            purity: registration.purity,
            dispatch: registration.dispatch,
            lowering: registration.lowering,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
