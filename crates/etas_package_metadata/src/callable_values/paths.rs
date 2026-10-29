use super::*;
use crate::{CallableRequestTarget, EffectArgKind, SpecMethodGenericArgument, TypeKind};

impl CallableValueGraph {
    /// Relocates descriptive paths through the package manager's resolved graph.
    /// Binder labels and artifact-local references retain their identities.
    pub fn remap_paths(
        &mut self,
        mut remap: impl FnMut(CallableValuePathKind, &[String]) -> Vec<String>,
    ) {
        remap_parts(vec![Part::Graph(self)], &mut remap);
    }
}

impl CallableRequestTarget {
    pub fn remap_paths(
        &mut self,
        mut remap: impl FnMut(CallableValuePathKind, &[String]) -> Vec<String>,
    ) {
        remap_parts(vec![Part::Target(self)], &mut remap);
    }
}

fn remap_parts(
    mut work: Vec<Part<'_>>,
    remap: &mut dyn FnMut(CallableValuePathKind, &[String]) -> Vec<String>,
) {
    while let Some(part) = work.pop() {
        match part {
            Part::Graph(graph) => {
                for node in &mut graph.values {
                    if let Some(requests) = &mut node.handler_requests {
                        for action in &mut requests.actions {
                            work.push(Part::Effect(&mut action.action));
                            for binding in action.type_bindings.iter_mut().flatten() {
                                work.push(Part::Type(&mut binding.ty));
                            }
                        }
                        for parameter in &mut requests.parameters {
                            work.push(Part::Target(&mut parameter.parameter));
                            work.extend(parameter.excluded.iter_mut().map(Part::Effect));
                        }
                    }
                    for generic in &mut node.generic_params {
                        for bound in &mut generic.bounds {
                            bound.spec = remap(CallableValuePathKind::Spec, &bound.spec);
                            work.extend(bound.args.iter_mut().map(Part::Type));
                        }
                    }
                    work.extend(node.params.iter_mut().map(Part::Type));
                    work.push(Part::Type(&mut node.output));
                    for row in [
                        &mut node.public_effects,
                        &mut node.requested_actions,
                        &mut node.handled_requested_actions,
                        &mut node.default_handler_effects.direct_effects,
                        &mut node.default_handler_effects.default_actions,
                    ] {
                        work.push(Part::Row(row));
                    }
                    for handler in &mut node.default_handler_effects.handlers {
                        work.push(Part::Effect(&mut handler.action));
                        work.push(Part::Row(&mut handler.produced));
                    }
                    work.push(Part::Trace(&mut node.action_trace));
                }
            }
            Part::Target(CallableRequestTarget::Parameter(_)) => {}
            Part::Target(CallableRequestTarget::SpecMethod(method)) => {
                method.spec = remap(CallableValuePathKind::Spec, &method.spec);
                work.push(Part::Type(&mut method.receiver));
                work.extend(method.spec_args.iter_mut().map(Part::Type));
                for argument in &mut method.generic_arguments {
                    work.push(match argument {
                        SpecMethodGenericArgument::Type(ty) => Part::Type(ty),
                        SpecMethodGenericArgument::EffectRow(row) => Part::Row(row),
                    });
                }
                work.extend(
                    method
                        .callable_arguments
                        .iter_mut()
                        .map(|argument| Part::Graph(&mut argument.sources)),
                );
            }
            Part::Type(ty) => {
                if matches!(
                    ty.kind,
                    TypeKind::Named
                        | TypeKind::Applied
                        | TypeKind::Alias
                        | TypeKind::Nominal
                        | TypeKind::Enum
                ) {
                    ty.path = remap(CallableValuePathKind::Type, &ty.path);
                }
                work.extend(ty.children.iter_mut().map(Part::Type));
                work.extend(ty.fields.iter_mut().map(|field| Part::Type(&mut field.ty)));
                work.extend(
                    ty.effects
                        .iter_mut()
                        .chain(&mut ty.produced_effects)
                        .map(Part::Row),
                );
            }
            Part::Row(row) => work.extend(row.effects.iter_mut().map(Part::Effect)),
            Part::Effect(effect) => {
                effect.path = remap(CallableValuePathKind::Effect, &effect.path);
                for arg in &mut effect.args {
                    if arg.kind == EffectArgKind::Path {
                        arg.path = remap(CallableValuePathKind::Resource, &arg.path);
                    }
                    work.extend(arg.ty.iter_mut().map(Part::Type));
                }
            }
            Part::Trace(trace) => match trace {
                ActionTrace::Event(event) => work.push(Part::Effect(&mut event.action)),
                ActionTrace::Seq(parts) | ActionTrace::Choice(parts) => {
                    work.extend(parts.iter_mut().map(Part::Trace))
                }
                ActionTrace::Repeat(part) => work.push(Part::Trace(part)),
                ActionTrace::UnknownOrder(actions) => {
                    work.extend(actions.iter_mut().map(Part::Effect))
                }
                ActionTrace::Widened {
                    actions,
                    parameter_calls,
                } => {
                    work.extend(actions.iter_mut().map(Part::Effect));
                    work.extend(parameter_calls.iter_mut().map(Part::Target));
                }
                ActionTrace::ParameterCall { parameter } => work.push(Part::Target(parameter)),
                ActionTrace::Empty => {}
            },
        }
    }
}

enum Part<'a> {
    Graph(&'a mut CallableValueGraph),
    Target(&'a mut CallableRequestTarget),
    Type(&'a mut Type),
    Row(&'a mut EffectRow),
    Effect(&'a mut crate::EffectRef),
    Trace(&'a mut ActionTrace),
}
