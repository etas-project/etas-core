use super::StdRegistryValidationError;
use crate::{
    EffectActionArgKind, EffectActionDecl, FlowDecl, IntrinsicDescriptor, IntrinsicDispatch,
    IntrinsicPurity, LoweringHint, StdDecl, StdEffectRef, StdRegistry, StdStaticArg, StdType,
};

pub(super) fn validate(
    registry: &StdRegistry,
    flow: &FlowDecl,
    descriptor: &IntrinsicDescriptor,
) -> Result<(), StdRegistryValidationError> {
    let (stable_id, payload, reducer) = match &descriptor.lowering {
        LoweringHint::DefaultAction { stable_id, payload } => (stable_id, payload, None),
        LoweringHint::ActionReducer {
            stable_id,
            payload,
            reducer,
        } => (stable_id, payload, Some(reducer)),
        _ => return Ok(()),
    };
    let action = registry.symbols().find_map(|symbol| match &symbol.decl {
        StdDecl::EffectAction(action) if action.stable_id == Some(*stable_id) => {
            Some((symbol, action))
        }
        _ => None,
    });
    let valid = action.is_some_and(|(symbol, action)| {
        let projected = payload.iter().map(|index| flow.params.get(*index).cloned()).collect::<Option<Vec<_>>>();
        descriptor.dispatch == IntrinsicDispatch::Host
            && descriptor.purity == IntrinsicPurity::Host
            && flow.type_params == action.type_params
            && projected.as_ref() == Some(&action.params)
            && match reducer {
                None => flow.output == action.output,
                Some(crate::StdActionReducer::ReadUntilLimit { limit_parameter, timeout_parameter }) => {
                    flow.output == StdType::parse("bytes")
                        && action.output == StdType::parse("std.stream.StreamRead")
                        && read_result_valid(registry)
                        && payload.len() == 1
                        && flow.params.len() == 3
                        && flow.params.get(*limit_parameter) == Some(&StdType::parse("std.stream.ByteLimit"))
                        && flow.params.get(*timeout_parameter) == Some(&StdType::parse("Option[std.stream.Timeout]"))
                        && payload.iter().all(|index| index != limit_parameter && index != timeout_parameter)
                        && limit_parameter != timeout_parameter
                }
            }
            && action.default_handler_effects.as_ref() == Some(&flow.public_effects)
            && matches!(flow.requested_actions.as_slice(), [effect] if
                (effect.path == [action.owner.clone(), action.name.clone()] || effect.path == symbol.qualified_path)
                && selectors_match(action, effect))
    });
    if !valid {
        return Err(StdRegistryValidationError {
            symbol: descriptor.qualified_path.join("."),
            reason: "default action adapter requires a declared payload projection, matching generic/result ABI and produced/requested rows".into(),
        });
    }
    Ok(())
}

fn selectors_match(action: &EffectActionDecl, effect: &StdEffectRef) -> bool {
    effect.args.len() == action.effect_args.len()
        && effect.args.iter().zip(&action.effect_args).enumerate().all(
            |(index, (arg, kind))| match (kind, arg) {
                (EffectActionArgKind::Type, StdStaticArg::Type(StdType::Var(name))) => {
                    action.selector_param_names.get(index) == Some(name)
                }
                (
                    EffectActionArgKind::StringPattern,
                    StdStaticArg::String(_) | StdStaticArg::Int(_) | StdStaticArg::Wildcard,
                ) => true,
                (
                    EffectActionArgKind::StaticResourcePath { .. }
                    | EffectActionArgKind::MemoryPlace,
                    StdStaticArg::Path(_) | StdStaticArg::Wildcard,
                ) => true,
                _ => false,
            },
        )
}

fn read_result_valid(registry: &StdRegistry) -> bool {
    let Some(owner) = registry.lookup_qualified(&["std", "stream", "StreamRead"]) else {
        return false;
    };
    if !matches!(&owner.decl, StdDecl::Type(decl) if decl.kind == crate::TypeDeclKind::Enum) {
        return false;
    }
    let constructors = registry.enum_constructors(owner.id).collect::<Vec<_>>();
    constructors.len() == 2 && [("Data", vec![StdType::parse("bytes")]), ("Eof", vec![])]
        .iter().all(|(name, params)| constructors.iter().any(|symbol|
            matches!(&symbol.decl, StdDecl::Flow(flow) if flow.name == *name && flow.params == *params && flow.output == StdType::parse("std.stream.StreamRead"))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_reducer_requires_the_declared_action_and_reduction_abi() {
        let registry = crate::standard_registry();
        let symbol = registry
            .lookup_qualified(&["std", "stream", "read_until_limit"])
            .unwrap();
        let StdDecl::Flow(flow) = &symbol.decl else {
            panic!("flow");
        };
        let descriptor = symbol.intrinsic.as_ref().unwrap();
        validate(&registry, flow, descriptor).unwrap();
        for change in 0..5 {
            let mut flow = flow.clone();
            let mut descriptor = descriptor.clone();
            match change {
                0 => flow.output = StdType::parse("std.stream.StreamRead"),
                1 => flow.params[1] = StdType::parse("usize"),
                2 => flow.params[2] = StdType::parse("string"),
                3 => {
                    descriptor.lowering = LoweringHint::ActionReducer {
                        stable_id: 131,
                        payload: vec![1],
                        reducer: crate::StdActionReducer::ReadUntilLimit {
                            limit_parameter: 1,
                            timeout_parameter: 2,
                        },
                    }
                }
                4 => {
                    descriptor.lowering = LoweringHint::ActionReducer {
                        stable_id: 131,
                        payload: vec![0],
                        reducer: crate::StdActionReducer::ReadUntilLimit {
                            limit_parameter: 2,
                            timeout_parameter: 1,
                        },
                    }
                }
                _ => unreachable!(),
            }
            assert!(
                validate(&registry, &flow, &descriptor).is_err(),
                "invalid reducer {change}"
            );
        }
    }
}
