use super::*;
use crate::{EffectArgKind, GenericParamKind, TypeKind};
use std::collections::{BTreeMap, BTreeSet};

impl CallableValueGraph {
    pub(super) fn validate_descriptor_bindings(
        &self,
        node: &CallableValueNode,
        root: Option<(usize, &[crate::GenericParamKind])>,
    ) -> Result<(), crate::MetadataArtifactError> {
        if node
            .behavior
            .independent_determinism
            .is_some_and(|value| value > node.behavior.determinism)
        {
            return Err(invalid(
                "callable independent determinism exceeds its summary",
            ));
        }
        let bindings = self.descriptor_bindings(&node.generic_bindings, root)?;
        let mut work = Vec::new();
        if let Some(requests) = &node.handler_requests {
            requests.validate_shape()?;
            for action in &requests.actions {
                work.push(Part::Effect(&action.action));
                if let Some(signature) = &action.signature {
                    work.extend(signature.params.iter().map(Part::Type));
                    work.push(Part::Type(&signature.output));
                }
                for binding in action.type_bindings.iter().flatten() {
                    work.push(Part::Type(&binding.ty));
                }
            }
            for parameter in &requests.parameters {
                work.push(Part::Target(&parameter.parameter));
                work.extend(parameter.excluded.iter().map(Part::Effect));
            }
        }
        for generic in &node.generic_params {
            for bound in &generic.bounds {
                valid_path(&bound.spec)?;
                work.extend(bound.args.iter().map(Part::Type));
            }
        }
        work.extend(node.params.iter().map(Part::Type));
        work.push(Part::Type(&node.output));
        for row in [
            &node.public_effects,
            &node.requested_actions,
            &node.handled_requested_actions,
            &node.default_handler_effects.direct_effects,
            &node.default_handler_effects.default_actions,
        ] {
            work.push(Part::Row(row));
        }
        for handler in &node.default_handler_effects.handlers {
            work.push(Part::Effect(&handler.action));
            work.push(Part::Row(&handler.produced));
        }
        work.push(Part::Trace(&node.action_trace));
        if let Some(behavior) = &node.default_handler_effects.behavior {
            behavior.validate_resources()?;
            for entry in &behavior.guarded {
                work.extend(entry.guards.iter().map(Part::Effect));
            }
            for footprint in behavior.footprints() {
                work.push(Part::Footprint(footprint));
            }
        }
        let has_parameter_calls = validate_parts(&bindings, work)?;
        if has_parameter_calls && node.behavior.independent_determinism.is_none() {
            return Err(invalid(
                "callable parameter trace is missing independent determinism",
            ));
        }
        Ok(())
    }

    pub(super) fn validate_object_descriptors(
        &self,
        object: &ExistentialValueNode,
        root: Option<(usize, &[GenericParamKind])>,
    ) -> Result<(), crate::MetadataArtifactError> {
        let bindings = self.descriptor_bindings(&object.generic_bindings, root)?;
        let mut work = vec![
            Part::Type(&object.object_type),
            Part::Type(&object.source_type),
        ];
        for witness in &object.witnesses {
            valid_path(&witness.bound.spec)?;
            work.extend(witness.bound.args.iter().map(Part::Type));
        }
        validate_parts(&bindings, work)?;
        Ok(())
    }

    fn descriptor_bindings<'a>(
        &self,
        values: &'a [CallableValueGenericBinding],
        root: Option<(usize, &[GenericParamKind])>,
    ) -> Result<BTreeMap<&'a str, Option<GenericParamKind>>, crate::MetadataArtifactError> {
        let mut bindings = BTreeMap::new();
        for binding in values {
            let kind = match binding.scope {
                CallableValueScope::Root => root
                    .map(|(_, kinds)| {
                        kinds
                            .get(binding.position as usize)
                            .copied()
                            .ok_or_else(|| {
                                invalid("value generic binding position is out of range")
                            })
                    })
                    .transpose()?,
                CallableValueScope::Value(index) => Some(
                    self.value(index)?
                        .generic_params
                        .get(binding.position as usize)
                        .ok_or_else(|| invalid("value generic binding position is out of range"))?
                        .kind,
                ),
            };
            if binding.label.is_empty() || bindings.insert(binding.label.as_str(), kind).is_some() {
                return Err(invalid(
                    "value generic binding label is empty or duplicated",
                ));
            }
        }
        Ok(bindings)
    }
}

fn validate_parts(
    bindings: &BTreeMap<&str, Option<GenericParamKind>>,
    mut work: Vec<Part<'_>>,
) -> Result<bool, crate::MetadataArtifactError> {
    let mut has_parameter_calls = false;
    while let Some(part) = work.pop() {
        match part {
            Part::Footprint(footprint) => {
                work.extend(
                    [
                        &footprint.escaping_effects,
                        &footprint.requested_actions,
                        &footprint.default_actions,
                        &footprint.handled_actions,
                    ]
                    .into_iter()
                    .map(Part::Row),
                );
                if let Some(requests) = &footprint.handler_requests {
                    for action in &requests.actions {
                        work.push(Part::Effect(&action.action));
                        if let Some(signature) = &action.signature {
                            work.extend(signature.params.iter().map(Part::Type));
                            work.push(Part::Type(&signature.output));
                        }
                        for binding in action.type_bindings.iter().flatten() {
                            work.push(Part::Type(&binding.ty));
                        }
                    }
                    for parameter in &requests.parameters {
                        work.push(Part::Target(&parameter.parameter));
                        work.extend(parameter.excluded.iter().map(Part::Effect));
                    }
                }
                for requirement in footprint
                    .requirements
                    .trace_requirements
                    .iter()
                    .chain(&footprint.requirements.trace_obligations)
                {
                    if let crate::MethodTraceRequirement::Clause(clause) = requirement {
                        for row in [
                            &clause.pattern,
                            &clause.guard,
                            &clause.target,
                            &clause.obligation,
                        ]
                        .into_iter()
                        .flatten()
                        {
                            work.push(Part::Row(row));
                        }
                        for bound in &clause.selector_bounds {
                            work.extend(bound.spec_args.iter().map(Part::Type));
                        }
                    }
                }
            }
            Part::Target(crate::CallableRequestTarget::Parameter(_)) => {}
            Part::Target(crate::CallableRequestTarget::AgentTools(context)) => {
                work.push(Part::Type(context))
            }
            Part::Target(crate::CallableRequestTarget::SpecMethod(method)) => {
                valid_path(&method.spec)?;
                work.push(Part::Type(&method.receiver));
                work.extend(method.spec_args.iter().map(Part::Type));
                for argument in &method.generic_arguments {
                    work.push(match argument {
                        crate::SpecMethodGenericArgument::Type(ty) => Part::Type(ty),
                        crate::SpecMethodGenericArgument::EffectRow(row) => Part::Row(row),
                    });
                }
            }
            Part::Type(ty) => {
                match ty.kind {
                    TypeKind::Var => require_binding(bindings, &ty.name, GenericParamKind::Type)?,
                    TypeKind::Named
                    | TypeKind::Applied
                    | TypeKind::Alias
                    | TypeKind::Nominal
                    | TypeKind::Enum => valid_path(&ty.path)?,
                    _ => {}
                }
                crate::type_validation::validate_type_shape(ty)?;
                for bound in &ty.bounds {
                    valid_path(&bound.spec)?;
                    work.extend(bound.args.iter().map(Part::Type));
                }
                let mut fields = BTreeSet::new();
                for field in &ty.fields {
                    if field.name.is_empty() || !fields.insert(&field.name) {
                        return Err(invalid(
                            "callable value record field is empty or duplicated",
                        ));
                    }
                    work.push(Part::Type(&field.ty));
                }
                work.extend(ty.children.iter().map(Part::Type));
                work.extend(ty.effects.iter().chain(&ty.produced_effects).map(Part::Row));
            }
            Part::Row(row) => {
                if let Some(tail) = &row.tail {
                    require_binding(bindings, tail, GenericParamKind::Effect)?;
                }
                work.extend(row.effects.iter().map(Part::Effect));
            }
            Part::Effect(effect) => {
                valid_path(&effect.path)?;
                for arg in &effect.args {
                    match arg.kind {
                        EffectArgKind::Type if arg.ty.is_none() => {
                            return Err(invalid("callable value effect type argument is missing"));
                        }
                        EffectArgKind::Path => valid_path(&arg.path)?,
                        _ => {}
                    }
                    work.extend(arg.ty.iter().map(Part::Type));
                }
            }
            Part::Trace(trace) => match trace {
                ActionTrace::Event(event) => work.push(Part::Effect(&event.action)),
                ActionTrace::DefaultHandler { guards, body } => {
                    work.extend(guards.iter().map(Part::Effect));
                    work.push(Part::Trace(body));
                }
                ActionTrace::Seq(parts) | ActionTrace::Choice(parts) => {
                    work.extend(parts.iter().map(Part::Trace))
                }
                ActionTrace::Repeat(part) => work.push(Part::Trace(part)),
                ActionTrace::UnknownOrder(actions) => work.extend(actions.iter().map(Part::Effect)),
                ActionTrace::Widened {
                    actions,
                    parameter_calls,
                } => {
                    work.extend(actions.iter().map(Part::Effect));
                    has_parameter_calls |= !parameter_calls.is_empty();
                    work.extend(parameter_calls.iter().map(Part::Target));
                }
                ActionTrace::ParameterCall { parameter } => {
                    has_parameter_calls = true;
                    work.push(Part::Target(parameter));
                }
                ActionTrace::Empty => {}
            },
        }
    }
    Ok(has_parameter_calls)
}

enum Part<'a> {
    Footprint(&'a crate::EffectFootprint),
    Target(&'a crate::CallableRequestTarget),
    Type(&'a Type),
    Row(&'a EffectRow),
    Effect(&'a crate::EffectRef),
    Trace(&'a ActionTrace),
}

fn valid_path(path: &[String]) -> Result<(), crate::MetadataArtifactError> {
    if path.is_empty() || path.iter().any(String::is_empty) {
        return Err(invalid("callable value descriptor contains an empty path"));
    }
    Ok(())
}

fn require_binding(
    bindings: &BTreeMap<&str, Option<GenericParamKind>>,
    label: &str,
    kind: GenericParamKind,
) -> Result<(), crate::MetadataArtifactError> {
    match bindings.get(label) {
        Some(Some(actual)) if *actual != kind => {
            Err(invalid("callable value generic binding has the wrong kind"))
        }
        Some(_) if !label.is_empty() => Ok(()),
        _ => Err(invalid(
            "callable value descriptor contains an unbound generic label",
        )),
    }
}
