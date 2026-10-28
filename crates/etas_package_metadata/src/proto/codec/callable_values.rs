use super::*;
use crate::{
    CallableValueGenericBinding, CallableValueGraph, CallableValueNode, CallableValueParameter,
    CallableValueParameterBinding, CallableValueScope, CallableValueSource,
};

pub(super) fn to_proto(graph: &CallableValueGraph) -> ProtoCallableValueGraph {
    ProtoCallableValueGraph {
        sources: graph.sources.iter().copied().map(source_to_proto).collect(),
        values: graph
            .values
            .iter()
            .map(|node| ProtoCallableValueNode {
                handler_requests: node
                    .handler_requests
                    .as_ref()
                    .map(request_template_to_proto),
                behavior: Some(ProtoCallableValueBehavior {
                    determinism: Some(node.behavior.determinism as i32),
                    host_requirements: node
                        .behavior
                        .host_requirements
                        .iter()
                        .map(|kind| *kind as i32)
                        .collect(),
                    interpreter_features: node
                        .behavior
                        .interpreter_features
                        .iter()
                        .map(|kind| *kind as i32)
                        .collect(),
                }),
                parent_scope: Some(scope_to_proto(node.parent_scope)),
                generic_params: node
                    .generic_params
                    .iter()
                    .map(generic_param_to_proto)
                    .collect(),
                generic_bindings: node
                    .generic_bindings
                    .iter()
                    .map(|binding| ProtoCallableValueGenericBinding {
                        label: binding.label.clone(),
                        scope: Some(scope_to_proto(binding.scope)),
                        position: binding.position,
                    })
                    .collect(),
                param_names: node.param_names.clone(),
                params: node.params.iter().map(type_to_proto).collect(),
                output: Some(type_to_proto(&node.output)),
                public_effects: Some(effect_row_to_proto(&node.public_effects)),
                requested_actions: Some(effect_row_to_proto(&node.requested_actions)),
                handled_requested_actions: Some(effect_row_to_proto(
                    &node.handled_requested_actions,
                )),
                default_handler_effects: Some(default_handler_effects_to_proto(
                    &node.default_handler_effects,
                )),
                parameter_bindings: node
                    .parameter_bindings
                    .iter()
                    .map(|binding| ProtoCallableValueParameterBinding {
                        label: binding.label.clone(),
                        parameter: Some(parameter_to_proto(binding.parameter)),
                    })
                    .collect(),
                action_trace: Some(action_trace_to_proto(&node.action_trace)),
                returned_sources: node.returned_sources.as_ref().map(|sources| {
                    ProtoCallableValueSources {
                        sources: sources.iter().copied().map(source_to_proto).collect(),
                    }
                }),
            })
            .collect(),
    }
}

pub(super) fn from_proto(
    graph: ProtoCallableValueGraph,
) -> Result<CallableValueGraph, MetadataArtifactError> {
    let graph = CallableValueGraph {
        sources: graph
            .sources
            .into_iter()
            .map(source_from_proto)
            .collect::<Result<_, _>>()?,
        values: graph
            .values
            .into_iter()
            .map(|node| {
                Ok(CallableValueNode {
                    handler_requests: node
                        .handler_requests
                        .map(request_template_from_proto)
                        .transpose()?,
                    behavior: behavior_from_proto(
                        node.behavior
                            .ok_or_else(|| invalid("callable value is missing behavior facts"))?,
                    )?,
                    parent_scope: scope_from_proto(
                        node.parent_scope
                            .ok_or_else(|| invalid("callable value is missing parent scope"))?,
                    )?,
                    generic_params: node
                        .generic_params
                        .into_iter()
                        .map(generic_param_from_proto)
                        .collect::<Result<_, _>>()?,
                    generic_bindings: node
                        .generic_bindings
                        .into_iter()
                        .map(|binding| {
                            Ok(CallableValueGenericBinding {
                                label: binding.label,
                                scope: scope_from_proto(binding.scope.ok_or_else(|| {
                                    invalid("callable generic binding is missing scope")
                                })?)?,
                                position: binding.position,
                            })
                        })
                        .collect::<Result<_, MetadataArtifactError>>()?,
                    param_names: node.param_names,
                    params: node
                        .params
                        .into_iter()
                        .map(type_from_proto)
                        .collect::<Result<_, _>>()?,
                    output: type_from_proto(
                        node.output
                            .ok_or_else(|| invalid("callable value is missing output type"))?,
                    )?,
                    public_effects: effect_row_from_proto(
                        node.public_effects
                            .ok_or_else(|| invalid("callable value is missing public row"))?,
                    )?,
                    requested_actions: effect_row_from_proto(
                        node.requested_actions
                            .ok_or_else(|| invalid("callable value is missing requested row"))?,
                    )?,
                    handled_requested_actions: effect_row_from_proto(
                        node.handled_requested_actions
                            .ok_or_else(|| invalid("callable value is missing handled row"))?,
                    )?,
                    default_handler_effects: default_handler_effects_from_proto(
                        node.default_handler_effects.ok_or_else(|| {
                            invalid("callable value is missing default handler facts")
                        })?,
                    )?,
                    parameter_bindings: node
                        .parameter_bindings
                        .into_iter()
                        .map(|binding| {
                            Ok(CallableValueParameterBinding {
                                label: binding.label,
                                parameter: parameter_from_proto(binding.parameter.ok_or_else(
                                    || invalid("callable parameter binding is missing parameter"),
                                )?)?,
                            })
                        })
                        .collect::<Result<_, MetadataArtifactError>>()?,
                    action_trace: action_trace_from_proto(
                        node.action_trace
                            .ok_or_else(|| invalid("callable value is missing action trace"))?,
                    )?,
                    returned_sources: node
                        .returned_sources
                        .map(|sources| sources.sources.into_iter().map(source_from_proto).collect())
                        .transpose()?,
                })
            })
            .collect::<Result<_, MetadataArtifactError>>()?,
    };
    graph.validate_references(None)?;
    Ok(graph)
}

fn scope_to_proto(scope: CallableValueScope) -> ProtoCallableValueScope {
    ProtoCallableValueScope {
        kind: Some(match scope {
            CallableValueScope::Root => super::super::schema::scope::Kind::Root(true),
            CallableValueScope::Value(index) => super::super::schema::scope::Kind::Value(index),
        }),
    }
}

fn behavior_from_proto(
    behavior: ProtoCallableValueBehavior,
) -> Result<crate::CallableValueBehavior, MetadataArtifactError> {
    Ok(crate::CallableValueBehavior {
        determinism: crate::CallableValueDeterminism::try_from(
            behavior
                .determinism
                .ok_or_else(|| invalid("callable value is missing determinism"))?,
        )
        .map_err(|_| invalid("unknown callable determinism"))?,
        host_requirements: behavior
            .host_requirements
            .into_iter()
            .map(|kind| {
                crate::CallableValueHostRequirement::try_from(kind)
                    .map_err(|_| invalid("unknown callable host requirement"))
            })
            .collect::<Result<_, _>>()?,
        interpreter_features: behavior
            .interpreter_features
            .into_iter()
            .map(|kind| {
                crate::CallableValueInterpreterFeature::try_from(kind)
                    .map_err(|_| invalid("unknown callable interpreter feature"))
            })
            .collect::<Result<_, _>>()?,
    })
}

fn scope_from_proto(
    scope: ProtoCallableValueScope,
) -> Result<CallableValueScope, MetadataArtifactError> {
    match scope.kind {
        Some(super::super::schema::scope::Kind::Root(true)) => Ok(CallableValueScope::Root),
        Some(super::super::schema::scope::Kind::Value(index)) => {
            Ok(CallableValueScope::Value(index))
        }
        _ => Err(invalid("callable value scope is missing or invalid")),
    }
}

fn parameter_to_proto(parameter: CallableValueParameter) -> ProtoCallableValueParameter {
    ProtoCallableValueParameter {
        scope: Some(scope_to_proto(parameter.scope)),
        position: parameter.position,
    }
}

fn parameter_from_proto(
    parameter: ProtoCallableValueParameter,
) -> Result<CallableValueParameter, MetadataArtifactError> {
    Ok(CallableValueParameter {
        scope: scope_from_proto(
            parameter
                .scope
                .ok_or_else(|| invalid("callable parameter is missing scope"))?,
        )?,
        position: parameter.position,
    })
}

fn source_to_proto(source: CallableValueSource) -> ProtoCallableValueSource {
    ProtoCallableValueSource {
        kind: Some(match source {
            CallableValueSource::Parameter(parameter) => {
                super::super::schema::source::Kind::Parameter(parameter_to_proto(parameter))
            }
            CallableValueSource::Value(index) => super::super::schema::source::Kind::Value(index),
        }),
    }
}

fn source_from_proto(
    source: ProtoCallableValueSource,
) -> Result<CallableValueSource, MetadataArtifactError> {
    match source.kind {
        Some(super::super::schema::source::Kind::Parameter(parameter)) => Ok(
            CallableValueSource::Parameter(parameter_from_proto(parameter)?),
        ),
        Some(super::super::schema::source::Kind::Value(index)) => {
            Ok(CallableValueSource::Value(index))
        }
        None => Err(invalid("callable return source is missing identity")),
    }
}

#[cfg(test)]
mod tests;
