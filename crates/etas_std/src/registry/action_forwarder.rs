use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdEffectRef, StdRegistry,
};

use super::StdRegistryValidationError;

pub(super) fn validate(
    registry: &StdRegistry,
    flow: &FlowDecl,
    descriptor: &IntrinsicDescriptor,
) -> Result<(), StdRegistryValidationError> {
    let LoweringHint::ForwardAction { stable_id } = descriptor.lowering else {
        return Ok(());
    };
    let action = registry.symbols().find_map(|symbol| match &symbol.decl {
        StdDecl::EffectAction(action) if action.stable_id == Some(stable_id) => {
            Some((symbol, action))
        }
        _ => None,
    });
    let valid = action.is_some_and(|(symbol, action)| {
        descriptor.dispatch == IntrinsicDispatch::Host
            && descriptor.purity == IntrinsicPurity::Host
            && flow.type_params == action.type_params
            && flow.params == action.params
            && flow.output == action.output
            && action.default_handler_effects.as_ref() == Some(&flow.public_effects)
            && flow.requested_actions
                == [StdEffectRef::new(
                    &symbol
                        .qualified_path
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                )]
            && action.effect_args.is_empty()
    });
    if !valid {
        return Err(StdRegistryValidationError {
        symbol: descriptor.qualified_path.join("."),
        reason: "action forwarder must reference a registered default handler with the same generic/payload/result ABI and exact produced/requested rows".into(),
    });
    }
    Ok(())
}
