use super::*;
use crate::{
    CallableRequestTarget, SpecMethodCallableArgument, SpecMethodGenericArgument,
    SpecMethodInvocation,
};

pub(crate) fn ensure_distinct(
    targets: &[CallableRequestTarget],
) -> Result<(), MetadataArtifactError> {
    let mut seen = std::collections::BTreeSet::new();
    for target in targets {
        let key = fingerprint(target);
        if !seen.insert(key) {
            return Err(invalid(
                "effect summary widened trace has duplicate callable targets",
            ));
        }
    }
    Ok(())
}

pub(crate) fn fingerprint(target: &CallableRequestTarget) -> etas_code::CodeFingerprint {
    etas_code::CodeFingerprint::of_bytes(
        "etas.metadata.callable-request-target.v13",
        &to_proto(target).encode_to_vec(),
    )
}

pub(super) fn to_proto(target: &CallableRequestTarget) -> ProtoCallableRequestTarget {
    use proto_callable_request_target::Target;
    ProtoCallableRequestTarget {
        target: Some(match target {
            CallableRequestTarget::Parameter(parameter) => Target::Parameter(parameter.clone()),
            CallableRequestTarget::SpecMethod(invocation) => {
                Target::SpecMethod(Box::new(ProtoSpecMethodInvocation {
                    receiver: Some(type_to_proto(&invocation.receiver)),
                    spec: invocation.spec.clone(),
                    spec_args: invocation.spec_args.iter().map(type_to_proto).collect(),
                    method: invocation.method.clone(),
                    arity: invocation.arity,
                    generic_arguments: invocation
                        .generic_arguments
                        .iter()
                        .map(|argument| {
                            use proto_spec_method_generic_argument::Argument;
                            ProtoSpecMethodGenericArgument {
                                argument: Some(match argument {
                                    SpecMethodGenericArgument::Type(ty) => {
                                        Argument::Type(Box::new(type_to_proto(ty)))
                                    }
                                    SpecMethodGenericArgument::EffectRow(row) => {
                                        Argument::EffectRow(effect_row_to_proto(row))
                                    }
                                }),
                            }
                        })
                        .collect(),
                    callable_arguments: invocation
                        .callable_arguments
                        .iter()
                        .map(|argument| ProtoSpecMethodCallableArgument {
                            position: argument.position,
                            sources: Some(callable_values::to_proto(&argument.sources)),
                        })
                        .collect(),
                }))
            }
        }),
    }
}

pub(super) fn from_proto(
    target: ProtoCallableRequestTarget,
    depth: usize,
    budget: &mut MetadataGraphBudget,
) -> Result<CallableRequestTarget, MetadataArtifactError> {
    use proto_callable_request_target::Target;
    budget.enter(depth, "callable evidence target")?;
    let target = match target
        .target
        .ok_or_else(|| invalid("callable evidence target is missing its kind"))?
    {
        Target::Parameter(parameter) => CallableRequestTarget::Parameter(parameter),
        Target::SpecMethod(invocation) => {
            CallableRequestTarget::SpecMethod(Box::new(SpecMethodInvocation {
                receiver: type_from_proto_with_budget(
                    invocation
                        .receiver
                        .ok_or_else(|| invalid("spec invocation receiver type is missing"))?,
                    depth + 1,
                    budget,
                )?,
                spec: invocation.spec,
                method: invocation.method,
                arity: invocation.arity,
                spec_args: invocation
                    .spec_args
                    .into_iter()
                    .map(|ty| type_from_proto_with_budget(ty, depth + 1, budget))
                    .collect::<Result<_, _>>()?,
                generic_arguments: invocation
                    .generic_arguments
                    .into_iter()
                    .map(|argument| {
                        use proto_spec_method_generic_argument::Argument;
                        match argument.argument.ok_or_else(|| {
                            invalid("spec invocation generic argument has no kind")
                        })? {
                            Argument::Type(ty) => {
                                type_from_proto_with_budget(*ty, depth + 1, budget)
                                    .map(|ty| SpecMethodGenericArgument::Type(Box::new(ty)))
                            }
                            Argument::EffectRow(row) => {
                                effect_row_from_proto_with_budget(row, depth + 1, budget)
                                    .map(SpecMethodGenericArgument::EffectRow)
                            }
                        }
                    })
                    .collect::<Result<_, _>>()?,
                callable_arguments: invocation
                    .callable_arguments
                    .into_iter()
                    .map(|argument| {
                        Ok(SpecMethodCallableArgument {
                            position: argument.position,
                            sources: callable_values::from_proto(argument.sources.ok_or_else(
                                || invalid("spec invocation callback sources are missing"),
                            )?)?,
                        })
                    })
                    .collect::<Result<_, MetadataArtifactError>>()?,
            }))
        }
    };
    target.validate_shape().map_err(invalid)?;
    Ok(target)
}
