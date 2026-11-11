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

impl crate::PublishedMethod {
    pub fn remap_paths(
        &mut self,
        mut remap: impl FnMut(CallableValuePathKind, &[String]) -> Vec<String>,
    ) {
        let mut parts = vec![
            Part::Type(&mut self.self_type),
            Part::Graph(&mut self.callable),
        ];
        for requirement in self
            .requirements
            .trace_requirements
            .iter_mut()
            .chain(&mut self.requirements.trace_obligations)
        {
            let crate::MethodTraceRequirement::Clause(clause) = requirement else {
                continue;
            };
            for bound in &mut clause.selector_bounds {
                bound.spec_path = remap(CallableValuePathKind::Spec, &bound.spec_path);
                bound.effect_path = remap(CallableValuePathKind::Effect, &bound.effect_path);
                parts.extend(bound.spec_args.iter_mut().map(Part::Type));
            }
            parts.extend(
                [
                    &mut clause.pattern,
                    &mut clause.guard,
                    &mut clause.target,
                    &mut clause.obligation,
                ]
                .into_iter()
                .flatten()
                .map(Part::Row),
            );
        }
        for param in &mut self.owner_params {
            for bound in &mut param.bounds {
                bound.spec = remap(CallableValuePathKind::Spec, &bound.spec);
                parts.extend(bound.args.iter_mut().map(Part::Type));
            }
        }
        remap_parts(parts, &mut remap);
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

impl crate::DefaultBehavior {
    pub fn remap_paths(
        &mut self,
        mut remap: impl FnMut(CallableValuePathKind, &[String]) -> Vec<String>,
    ) {
        remap_parts(vec![Part::Behavior(self)], &mut remap);
    }
}

fn remap_parts(
    mut work: Vec<Part<'_>>,
    remap: &mut dyn FnMut(CallableValuePathKind, &[String]) -> Vec<String>,
) {
    while let Some(part) = work.pop() {
        match part {
            Part::Behavior(behavior) => {
                work.push(Part::Footprint(&mut behavior.direct));
                for entry in &mut behavior.guarded {
                    work.extend(entry.guards.iter_mut().map(Part::Effect));
                    work.push(Part::Footprint(&mut entry.footprint));
                }
            }
            Part::Footprint(footprint) => {
                work.extend(
                    [
                        &mut footprint.escaping_effects,
                        &mut footprint.requested_actions,
                        &mut footprint.default_actions,
                        &mut footprint.handled_actions,
                    ]
                    .into_iter()
                    .map(Part::Row),
                );
                if let Some(requests) = &mut footprint.handler_requests {
                    for action in &mut requests.actions {
                        work.push(Part::Effect(&mut action.action));
                        if let Some(signature) = &mut action.signature {
                            work.extend(signature.params.iter_mut().map(Part::Type));
                            work.push(Part::Type(&mut signature.output));
                        }
                        for binding in action.type_bindings.iter_mut().flatten() {
                            work.push(Part::Type(&mut binding.ty));
                        }
                    }
                    for parameter in &mut requests.parameters {
                        work.push(Part::Target(&mut parameter.parameter));
                        work.extend(parameter.excluded.iter_mut().map(Part::Effect));
                    }
                }
                for requirement in footprint
                    .requirements
                    .trace_requirements
                    .iter_mut()
                    .chain(&mut footprint.requirements.trace_obligations)
                {
                    if let crate::MethodTraceRequirement::Clause(clause) = requirement {
                        for bound in &mut clause.selector_bounds {
                            bound.spec_path = remap(CallableValuePathKind::Spec, &bound.spec_path);
                            bound.effect_path =
                                remap(CallableValuePathKind::Effect, &bound.effect_path);
                            work.extend(bound.spec_args.iter_mut().map(Part::Type));
                        }
                        work.extend(
                            [
                                &mut clause.pattern,
                                &mut clause.guard,
                                &mut clause.target,
                                &mut clause.obligation,
                            ]
                            .into_iter()
                            .flatten()
                            .map(Part::Row),
                        );
                    }
                }
            }
            Part::Graph(graph) => {
                for object in &mut graph.objects {
                    work.push(Part::Type(&mut object.object_type));
                    work.push(Part::Type(&mut object.source_type));
                    for witness in &mut object.witnesses {
                        witness.bound.spec =
                            remap(CallableValuePathKind::Spec, &witness.bound.spec);
                        work.extend(witness.bound.args.iter_mut().map(Part::Type));
                    }
                }
                for node in &mut graph.values {
                    if let Some(behavior) = &mut node.default_handler_effects.behavior {
                        work.push(Part::Behavior(behavior));
                    }
                    if let Some(requests) = &mut node.handler_requests {
                        for action in &mut requests.actions {
                            work.push(Part::Effect(&mut action.action));
                            if let Some(signature) = &mut action.signature {
                                work.extend(signature.params.iter_mut().map(Part::Type));
                                work.push(Part::Type(&mut signature.output));
                            }
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
            Part::Target(CallableRequestTarget::AgentTools(context)) => {
                work.push(Part::Type(context))
            }
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
                for bound in &mut ty.bounds {
                    bound.spec = remap(CallableValuePathKind::Spec, &bound.spec);
                    work.extend(bound.args.iter_mut().map(Part::Type));
                }
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
                ActionTrace::DefaultHandler { guards, body } => {
                    work.extend(guards.iter_mut().map(Part::Effect));
                    work.push(Part::Trace(body));
                }
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
    Behavior(&'a mut crate::DefaultBehavior),
    Footprint(&'a mut crate::EffectFootprint),
    Graph(&'a mut CallableValueGraph),
    Target(&'a mut CallableRequestTarget),
    Type(&'a mut Type),
    Row(&'a mut EffectRow),
    Effect(&'a mut crate::EffectRef),
    Trace(&'a mut ActionTrace),
}
