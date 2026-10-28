use super::*;
use crate::{EffectArgKind, TypeKind};

impl CallableValueGraph {
    /// Relocates descriptive paths through the package manager's resolved graph.
    /// Binder labels and artifact-local references retain their identities.
    pub fn remap_paths(
        &mut self,
        mut remap: impl FnMut(CallableValuePathKind, &[String]) -> Vec<String>,
    ) {
        for node in &mut self.values {
            let mut work = Vec::new();
            if let Some(requests) = &mut node.handler_requests {
                for action in &mut requests.actions {
                    work.push(Part::Effect(&mut action.action));
                    for binding in action.type_bindings.iter_mut().flatten() {
                        work.push(Part::Type(&mut binding.ty));
                    }
                }
                for parameter in &mut requests.parameters {
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
            while let Some(part) = work.pop() {
                match part {
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
                        ActionTrace::UnknownOrder(actions)
                        | ActionTrace::Widened { actions, .. } => {
                            work.extend(actions.iter_mut().map(Part::Effect))
                        }
                        ActionTrace::Empty | ActionTrace::ParameterCall { .. } => {}
                    },
                }
            }
        }
    }
}

enum Part<'a> {
    Type(&'a mut Type),
    Row(&'a mut EffectRow),
    Effect(&'a mut crate::EffectRef),
    Trace(&'a mut ActionTrace),
}
