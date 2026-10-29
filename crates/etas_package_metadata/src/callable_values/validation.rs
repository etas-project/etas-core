use super::*;
use crate::{EffectArgKind, GenericParamKind, TypeKind};
use std::collections::{BTreeMap, BTreeSet};

impl CallableValueGraph {
    pub(super) fn validate_descriptor_bindings(
        &self,
        node: &CallableValueNode,
        root: Option<(usize, &[crate::GenericParamKind])>,
    ) -> Result<(), crate::MetadataArtifactError> {
        let mut bindings = BTreeMap::new();
        for binding in &node.generic_bindings {
            let kind = match binding.scope {
                CallableValueScope::Root => root
                    .map(|(_, kinds)| {
                        kinds
                            .get(binding.position as usize)
                            .copied()
                            .ok_or_else(|| {
                                invalid("callable generic binding position is out of range")
                            })
                    })
                    .transpose()?,
                CallableValueScope::Value(index) => Some(
                    self.value(index)?
                        .generic_params
                        .get(binding.position as usize)
                        .ok_or_else(|| {
                            invalid("callable generic binding position is out of range")
                        })?
                        .kind,
                ),
            };
            if binding.label.is_empty() || bindings.insert(binding.label.as_str(), kind).is_some() {
                return Err(invalid(
                    "callable generic binding label is empty or duplicated",
                ));
            }
        }
        let mut work = Vec::new();
        if let Some(requests) = &node.handler_requests {
            requests.validate_shape()?;
            for action in &requests.actions {
                work.push(Part::Effect(&action.action));
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
        while let Some(part) = work.pop() {
            match part {
                Part::Target(crate::CallableRequestTarget::Parameter(_)) => {}
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
                        TypeKind::Var => {
                            require_binding(&bindings, &ty.name, GenericParamKind::Type)?
                        }
                        TypeKind::Named
                        | TypeKind::Applied
                        | TypeKind::Alias
                        | TypeKind::Nominal
                        | TypeKind::Enum => valid_path(&ty.path)?,
                        _ => {}
                    }
                    crate::type_validation::validate_type_shape(
                        ty.kind,
                        &ty.name,
                        &ty.path,
                        &ty.children,
                        &ty.fields,
                        &ty.effects,
                        &ty.produced_effects,
                    )?;
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
                        require_binding(&bindings, tail, GenericParamKind::Effect)?;
                    }
                    work.extend(row.effects.iter().map(Part::Effect));
                }
                Part::Effect(effect) => {
                    valid_path(&effect.path)?;
                    for arg in &effect.args {
                        match arg.kind {
                            EffectArgKind::Type if arg.ty.is_none() => {
                                return Err(invalid(
                                    "callable value effect type argument is missing",
                                ));
                            }
                            EffectArgKind::Path => valid_path(&arg.path)?,
                            _ => {}
                        }
                        work.extend(arg.ty.iter().map(Part::Type));
                    }
                }
                Part::Trace(trace) => match trace {
                    ActionTrace::Event(event) => work.push(Part::Effect(&event.action)),
                    ActionTrace::Seq(parts) | ActionTrace::Choice(parts) => {
                        work.extend(parts.iter().map(Part::Trace))
                    }
                    ActionTrace::Repeat(part) => work.push(Part::Trace(part)),
                    ActionTrace::UnknownOrder(actions) => {
                        work.extend(actions.iter().map(Part::Effect))
                    }
                    ActionTrace::Widened {
                        actions,
                        parameter_calls,
                    } => {
                        work.extend(actions.iter().map(Part::Effect));
                        work.extend(parameter_calls.iter().map(Part::Target));
                    }
                    ActionTrace::ParameterCall { parameter } => work.push(Part::Target(parameter)),
                    ActionTrace::Empty => {}
                },
            }
        }
        Ok(())
    }
}

enum Part<'a> {
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
