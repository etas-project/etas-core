use crate::{
    FlowDecl, FlowSourceMethod, FlowSourceMethodKind, FlowSourceMethodOperation, StdEffectRef,
    StdRegistryValidationError, StdSpecRef, StdType,
};

pub(super) fn validate(
    owner: &str,
    flow: &FlowDecl,
    method: &FlowSourceMethod,
) -> Result<(), StdRegistryValidationError> {
    if method.operation != FlowSourceMethodOperation::CheckedIndex {
        return Ok(());
    }
    let shape_matches = match (&method.receiver, flow.params.as_slice()) {
        (
            StdType::Array(element) | StdType::List(element) | StdType::Slice(element),
            [receiver, StdType::Var(index)],
        ) => {
            receiver == &method.receiver
                && element.as_ref() == &flow.output
                && flow.type_params.iter().any(|param| {
                    param.name == *index
                        && param
                            .bounds
                            .contains(&StdSpecRef::new(&["std", "core", "Index"]))
                })
        }
        _ => false,
    };
    let expected_effects = [StdEffectRef::typed(
        &["Error"],
        StdType::parse("IndexError"),
    )];
    if method.kind != FlowSourceMethodKind::ValueMethod
        || !shape_matches
        || flow.public_effects != expected_effects
        || !flow.requested_actions.is_empty()
    {
        return Err(StdRegistryValidationError {
            symbol: owner.to_owned(),
            reason: "checked index method requires a sequence receiver, an Index-bounded argument, its element result and Error<IndexError>".to_owned(),
        });
    }
    Ok(())
}
