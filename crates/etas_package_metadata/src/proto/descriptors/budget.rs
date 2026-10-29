use etas_code::descriptor::*;

use super::super::budget::MetadataGraphBudget;
use super::{invalid, limits, schema as p};
use crate::MetadataArtifactError as Error;

pub(in crate::proto) fn validate_model(
    set: &DefinitionDescriptorSet,
    budget: &mut MetadataGraphBudget,
) -> Result<(), Error> {
    let graph = &set.graph;
    if let Some(headers) = &set.headers {
        budget.charge(
            headers
                .resource_units(limits())
                .map_err(|error| invalid(error.to_string()))?,
            "definition headers",
        )?;
    }
    budget.charge(set.declarations.len(), "descriptor declarations")?;
    budget.charge(1, "definition descriptors")?;
    budget.charge(graph.types.len(), "descriptor types")?;
    budget.charge(graph.rows.len(), "descriptor rows")?;
    budget.charge(graph.binders.len(), "descriptor binders")?;
    budget.charge(graph.traces.len(), "descriptor traces")?;
    for ty in &graph.types {
        let count = match ty {
            TypeInfo::Primitive(_) | TypeInfo::Parameter(_) => 0,
            TypeInfo::Builtin { arguments, .. } | TypeInfo::Tuple(arguments) => arguments.len(),
            TypeInfo::Record(fields) => fields.len(),
            TypeInfo::Nominal {
                parameters,
                representation,
                ..
            } => parameters
                .len()
                .saturating_add(usize::from(representation.is_some())),
            TypeInfo::Alias { parameters, .. } => parameters.len().saturating_add(1),
            TypeInfo::Enum {
                parameters,
                variants,
                ..
            } => {
                budget.charge(parameters.len(), "enum descriptor parameters")?;
                budget.charge(variants.len(), "enum descriptor variants")?;
                for variant in variants {
                    budget.charge(
                        match &variant.payload {
                            VariantPayloadInfo::None => 0,
                            VariantPayloadInfo::Positional(fields) => fields.len(),
                            VariantPayloadInfo::Named(fields) => fields.len(),
                        },
                        "enum descriptor fields",
                    )?;
                }
                0
            }
            TypeInfo::Applied { arguments, .. } => arguments.len().saturating_add(1),
            TypeInfo::Function(signature) => signature
                .generics
                .len()
                .saturating_add(signature.parameters.len())
                .saturating_add(1)
                .saturating_add(usize::from(signature.declared_output.is_some()))
                .saturating_add(usize::from(signature.effects.is_some())),
            TypeInfo::Handler {
                produced, result, ..
            } => 1 + usize::from(produced.is_some()) + usize::from(result.is_some()),
        };
        budget.charge(count, "descriptor type references")?;
    }
    for row in &graph.rows {
        budget.charge(usize::from(row.tail.is_some()), "descriptor row tail")?;
        for effect in &row.effects {
            budget.charge(
                1usize.saturating_add(effect.arguments.len()),
                "descriptor effect arguments",
            )?;
        }
    }
    for binder in &graph.binders {
        for bound in &binder.bounds {
            budget.charge(
                1usize.saturating_add(bound.arguments.len()),
                "descriptor spec bound",
            )?;
        }
    }
    for trace in &graph.traces {
        budget.charge(
            match trace {
                TraceInfo::Empty => 0,
                TraceInfo::Event(effect) => 1usize.saturating_add(effect.arguments.len()),
                TraceInfo::Seq(children) | TraceInfo::Choice(children) => children.len(),
                TraceInfo::Widened {
                    parameter_calls, ..
                } => parameter_calls.iter().fold(1usize, |count, call| {
                    count.saturating_add(model_call_references(call))
                }),
                TraceInfo::ParameterCall(call)
                | TraceInfo::Outcome(CallableOutcomeInfo::ParameterNonReturn(call)) => {
                    model_call_references(call)
                }
                TraceInfo::Outcome(_) | TraceInfo::Repeat(_) | TraceInfo::UnknownOrder(_) => 1,
            },
            "descriptor trace references",
        )?;
    }
    set.validate(limits())
        .map_err(|error| invalid(error.to_string()))
}

pub(in crate::proto) fn validate_wire(
    graph: &p::Graph,
    budget: &mut MetadataGraphBudget,
) -> Result<(), Error> {
    if let Some(headers) = &graph.headers {
        super::header_budget::validate(headers, budget)?;
    }
    budget.charge(graph.declarations.len(), "descriptor declarations")?;
    budget.charge(1, "definition descriptors")?;
    budget.charge(graph.types.len(), "descriptor types")?;
    budget.charge(graph.rows.len(), "descriptor rows")?;
    budget.charge(graph.binders.len(), "descriptor binders")?;
    budget.charge(graph.traces.len(), "descriptor traces")?;
    for ty in &graph.types {
        use p::ty::Kind as K;
        let count = match &ty.kind {
            None | Some(K::Primitive(_) | K::Parameter(_)) => 0,
            Some(K::Builtin(value)) => value.arguments.len(),
            Some(K::Tuple(value)) => value.values.len(),
            Some(K::Record(value)) => value.values.len(),
            Some(K::Nominal(value) | K::Alias(value) | K::Enum(value)) => {
                budget.charge(value.parameters.len(), "declaration descriptor parameters")?;
                budget.charge(value.variants.len(), "enum descriptor variants")?;
                for variant in &value.variants {
                    budget.charge(
                        match &variant.payload {
                            Some(p::variant::Payload::Positional(value)) => value.values.len(),
                            Some(p::variant::Payload::Named(value)) => value.values.len(),
                            None | Some(p::variant::Payload::None(_)) => 0,
                        },
                        "enum descriptor fields",
                    )?;
                }
                usize::from(value.representation.is_some())
            }
            Some(K::Applied(value)) => value.arguments.len().saturating_add(1),
            Some(K::Function(value)) => value
                .generics
                .len()
                .saturating_add(value.parameters.len())
                .saturating_add(1)
                .saturating_add(usize::from(value.declared_output.is_some()))
                .saturating_add(usize::from(value.effects.is_some())),
            Some(K::Handler(value)) => {
                1 + usize::from(value.produced.is_some()) + usize::from(value.result.is_some())
            }
        };
        budget.charge(count, "descriptor type references")?;
    }
    for row in &graph.rows {
        budget.charge(usize::from(row.tail.is_some()), "descriptor row tail")?;
        for effect in &row.effects {
            budget.charge(
                1usize.saturating_add(effect.arguments.len()),
                "descriptor effect arguments",
            )?;
        }
    }
    for binder in &graph.binders {
        for bound in &binder.bounds {
            budget.charge(
                1usize.saturating_add(bound.arguments.len()),
                "descriptor spec bound",
            )?;
        }
    }
    for trace in &graph.traces {
        use p::trace::Kind as K;
        budget.charge(
            match &trace.kind {
                None | Some(K::Empty(_)) => 0,
                Some(K::Event(value)) => 1usize.saturating_add(value.arguments.len()),
                Some(K::Seq(value) | K::Choice(value)) => value.values.len(),
                Some(K::Widened(value)) => {
                    value.parameter_calls.iter().fold(1usize, |count, call| {
                        count.saturating_add(wire_call_references(call))
                    })
                }
                Some(K::ParameterCall(call)) => wire_call_references(call),
                Some(K::Outcome(outcome)) => match &outcome.kind {
                    Some(p::outcome::Kind::ParameterNonReturn(call)) => wire_call_references(call),
                    _ => 1,
                },
                Some(K::Repeat(_) | K::UnknownOrder(_)) => 1,
            },
            "descriptor trace references",
        )?;
    }
    Ok(())
}

fn model_call_references(call: &ParameterCallInfo) -> usize {
    1usize
        .saturating_add(call.arguments.as_ref().map_or(0, Vec::len))
        .saturating_add(call.handlers.as_ref().map_or(0, Vec::len))
}

fn wire_call_references(call: &p::ParameterCall) -> usize {
    1usize
        .saturating_add(
            call.arguments
                .as_ref()
                .map_or(0, |arguments| arguments.values.len()),
        )
        .saturating_add(
            call.handlers
                .as_ref()
                .map_or(0, |handlers| handlers.scopes.len()),
        )
}
