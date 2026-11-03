//! Structural fingerprint of input data, not metadata validation or evidence.

use super::*;
use crate::{
    ActionTrace, EffectArg, EffectArgKind, EffectRef, EffectRow, Type, TypeField, TypeKind,
};

enum Task<'a> {
    Type(&'a Type),
    Field(&'a TypeField),
    Row(&'a EffectRow),
    Effect(&'a EffectRef),
    Arg(&'a EffectArg),
    Trace(&'a ActionTrace),
}

struct Encoder(blake3::Hasher);
impl Encoder {
    fn new() -> Self {
        Self(blake3::Hasher::new_derive_key(
            "etas.metadata.callable-value-input.v3",
        ))
    }
    fn byte(&mut self, value: u8) {
        self.0.update(&[value]);
    }
    fn number(&mut self, value: usize) {
        self.0.update(&(value as u64).to_le_bytes());
    }
    fn text(&mut self, value: &str) {
        self.number(value.len());
        self.0.update(value.as_bytes());
    }
    fn target(&mut self, target: &crate::CallableRequestTarget) {
        self.0
            .update(crate::proto::callable_request_target_fingerprint(target).as_bytes());
    }
    fn path(&mut self, path: &[String]) {
        self.number(path.len());
        for segment in path {
            self.text(segment);
        }
    }
    fn option(&mut self, present: bool) {
        self.byte(u8::from(present));
    }
    fn texts(&mut self, values: &[String]) {
        self.number(values.len());
        for value in values {
            self.text(value);
        }
    }
    fn scope(&mut self, scope: CallableValueScope) {
        match scope {
            CallableValueScope::Root => self.byte(0),
            CallableValueScope::Value(id) => {
                self.byte(1);
                self.number(id as usize);
            }
        }
    }
    fn parameter(&mut self, parameter: CallableValueParameter) {
        self.scope(parameter.scope);
        self.number(parameter.position as usize);
    }
    fn source(&mut self, source: CallableValueSource) {
        match source {
            CallableValueSource::Parameter(value) => {
                self.byte(0);
                self.parameter(value);
            }
            CallableValueSource::Value(value) => {
                self.byte(1);
                self.number(value as usize);
            }
            CallableValueSource::FrozenContract(value) => {
                self.byte(2);
                self.number(value as usize);
            }
        }
    }
    fn encode(&mut self, root: Task<'_>) {
        let mut pending = vec![root];
        while let Some(task) = pending.pop() {
            match task {
                Task::Type(ty) => {
                    self.byte(0);
                    self.byte(match ty.kind {
                        TypeKind::Primitive => 0,
                        TypeKind::Var => 1,
                        TypeKind::Named => 2,
                        TypeKind::Applied => 3,
                        TypeKind::Alias => 4,
                        TypeKind::Nominal => 5,
                        TypeKind::Enum => 6,
                        TypeKind::Array => 7,
                        TypeKind::List => 8,
                        TypeKind::Map => 9,
                        TypeKind::Set => 10,
                        TypeKind::Range => 11,
                        TypeKind::Slice => 12,
                        TypeKind::Option => 13,
                        TypeKind::Result => 14,
                        TypeKind::Record => 15,
                        TypeKind::Tuple => 16,
                        TypeKind::Function => 17,
                        TypeKind::Handler => 18,
                        TypeKind::Trusted => 19,
                        TypeKind::Untrusted => 20,
                        TypeKind::Secret => 21,
                        TypeKind::Public => 22,
                        TypeKind::Sanitized => 23,
                        TypeKind::Prompt => 24,
                        TypeKind::PromptPart => 25,
                        TypeKind::Message => 26,
                        TypeKind::MemorySelection => 27,
                        TypeKind::Store => 28,
                        TypeKind::MemoryRegion => 29,
                        TypeKind::ResourceHandle => 30,
                    });
                    self.text(&ty.name);
                    self.path(&ty.path);
                    self.number(ty.children.len());
                    self.number(ty.fields.len());
                    self.option(ty.effects.is_some());
                    self.option(ty.produced_effects.is_some());
                    if let Some(row) = &ty.produced_effects {
                        pending.push(Task::Row(row));
                    }
                    if let Some(row) = &ty.effects {
                        pending.push(Task::Row(row));
                    }
                    pending.extend(ty.fields.iter().rev().map(Task::Field));
                    pending.extend(ty.children.iter().rev().map(Task::Type));
                }
                Task::Field(field) => {
                    self.byte(1);
                    self.text(&field.name);
                    pending.push(Task::Type(&field.ty));
                }
                Task::Row(row) => {
                    self.byte(2);
                    self.option(row.tail.is_some());
                    if let Some(tail) = &row.tail {
                        self.text(tail);
                    }
                    self.number(row.effects.len());
                    pending.extend(row.effects.iter().rev().map(Task::Effect));
                }
                Task::Effect(effect) => {
                    self.byte(3);
                    self.path(&effect.path);
                    self.number(effect.args.len());
                    pending.extend(effect.args.iter().rev().map(Task::Arg));
                }
                Task::Arg(arg) => {
                    self.byte(4);
                    self.byte(match arg.kind {
                        EffectArgKind::Type => 0,
                        EffectArgKind::Path => 1,
                        EffectArgKind::String => 2,
                        EffectArgKind::Int => 3,
                        EffectArgKind::Wildcard => 4,
                    });
                    self.path(&arg.path);
                    self.text(&arg.value);
                    self.option(arg.ty.is_some());
                    if let Some(ty) = &arg.ty {
                        pending.push(Task::Type(ty));
                    }
                }
                Task::Trace(trace) => {
                    self.byte(5);
                    match trace {
                        ActionTrace::Empty => self.byte(0),
                        ActionTrace::Event(event) => {
                            self.byte(1);
                            self.byte(match event.source {
                                crate::ActionTraceEventSource::Perform => 0,
                                crate::ActionTraceEventSource::StdIntrinsic => 1,
                                crate::ActionTraceEventSource::AgentCall => 2,
                                crate::ActionTraceEventSource::ExternalMetadata => 3,
                                crate::ActionTraceEventSource::Transfer => 4,
                                crate::ActionTraceEventSource::Unknown => 5,
                            });
                            pending.push(Task::Effect(&event.action));
                        }
                        ActionTrace::ParameterCall { parameter } => {
                            self.byte(2);
                            self.target(parameter);
                        }
                        ActionTrace::Seq(parts) => {
                            self.byte(3);
                            self.number(parts.len());
                            pending.extend(parts.iter().rev().map(Task::Trace));
                        }
                        ActionTrace::Choice(parts) => {
                            self.byte(4);
                            self.number(parts.len());
                            pending.extend(parts.iter().rev().map(Task::Trace));
                        }
                        ActionTrace::Repeat(part) => {
                            self.byte(5);
                            pending.push(Task::Trace(part));
                        }
                        ActionTrace::UnknownOrder(actions) => {
                            self.byte(6);
                            self.number(actions.len());
                            pending.extend(actions.iter().rev().map(Task::Effect));
                        }
                        ActionTrace::Widened {
                            actions,
                            parameter_calls,
                        } => {
                            self.byte(7);
                            self.number(actions.len());
                            self.number(parameter_calls.len());
                            for parameter in parameter_calls {
                                self.target(parameter);
                            }
                            pending.extend(actions.iter().rev().map(Task::Effect));
                        }
                    }
                }
            }
        }
    }
}

/// Hashes all input fields without interpreting references or admitting a graph.
pub fn callable_value_graph_input_fingerprint(
    graph: &CallableValueGraph,
) -> etas_code::CodeFingerprint {
    let mut e = Encoder::new();
    e.number(graph.sources.len());
    for source in &graph.sources {
        e.source(*source);
    }
    e.number(graph.values.len());
    for node in &graph.values {
        e.number(node.residual_checks.len());
        for reference in &node.residual_checks {
            e.0.update(reference.fingerprint().as_bytes());
        }
        e.option(node.handler_requests.is_some());
        if let Some(requests) = &node.handler_requests {
            e.number(requests.actions.len());
            for action in &requests.actions {
                e.encode(Task::Effect(&action.action));
                e.option(action.type_bindings.is_some());
                if let Some(bindings) = &action.type_bindings {
                    e.number(bindings.len());
                    for binding in bindings {
                        e.text(&binding.parameter);
                        e.encode(Task::Type(&binding.ty));
                    }
                }
                e.option(action.signature.is_some());
                if let Some(signature) = &action.signature {
                    e.number(signature.params.len());
                    for ty in &signature.params {
                        e.encode(Task::Type(ty));
                    }
                    e.encode(Task::Type(&signature.output));
                }
            }
            e.number(requests.parameters.len());
            for parameter in &requests.parameters {
                e.target(&parameter.parameter);
                e.number(parameter.excluded.len());
                for effect in &parameter.excluded {
                    e.encode(Task::Effect(effect));
                }
            }
        }
        e.number(node.behavior.determinism as usize);
        e.option(node.behavior.independent_determinism.is_some());
        if let Some(value) = node.behavior.independent_determinism {
            e.number(value as usize);
        }
        e.number(node.behavior.host_requirements.len());
        for value in &node.behavior.host_requirements {
            e.number(*value as usize);
        }
        e.number(node.behavior.interpreter_features.len());
        for value in &node.behavior.interpreter_features {
            e.number(*value as usize);
        }
        e.scope(node.parent_scope);
        e.number(node.generic_params.len());
        for param in &node.generic_params {
            e.text(&param.name);
            e.byte(match param.kind {
                crate::GenericParamKind::Type => 0,
                crate::GenericParamKind::Effect => 1,
            });
            e.number(param.bounds.len());
            for bound in &param.bounds {
                e.path(&bound.spec);
                e.number(bound.args.len());
                for ty in &bound.args {
                    e.encode(Task::Type(ty));
                }
            }
        }
        e.number(node.generic_bindings.len());
        for binding in &node.generic_bindings {
            e.text(&binding.label);
            e.scope(binding.scope);
            e.number(binding.position as usize);
        }
        e.texts(&node.param_names);
        e.number(node.params.len());
        for ty in &node.params {
            e.encode(Task::Type(ty));
        }
        e.encode(Task::Type(&node.output));
        e.encode(Task::Row(&node.public_effects));
        e.encode(Task::Row(&node.requested_actions));
        e.encode(Task::Row(&node.handled_requested_actions));
        e.encode(Task::Row(&node.default_handler_effects.direct_effects));
        e.encode(Task::Row(&node.default_handler_effects.default_actions));
        e.number(node.default_handler_effects.handlers.len());
        for handler in &node.default_handler_effects.handlers {
            e.encode(Task::Effect(&handler.action));
            e.encode(Task::Row(&handler.produced));
        }
        e.number(node.parameter_bindings.len());
        for binding in &node.parameter_bindings {
            e.text(&binding.label);
            e.parameter(binding.parameter);
        }
        e.encode(Task::Trace(&node.action_trace));
        e.option(node.returned_sources.is_some());
        if let Some(sources) = &node.returned_sources {
            e.number(sources.len());
            for source in sources {
                e.source(*source);
            }
        }
    }
    etas_code::CodeFingerprint::from_bytes(*e.0.finalize().as_bytes())
}

pub fn published_method_input_fingerprint(
    method: &crate::PublishedMethod,
) -> etas_code::CodeFingerprint {
    let mut e = Encoder::new();
    e.text("published-method.v1");
    e.0.update(method.identity.fingerprint().as_bytes());
    e.encode(Task::Type(&method.self_type));
    match method.receiver {
        crate::MethodReceiver::Value { slot } => {
            e.byte(0);
            e.number(slot as usize);
        }
        crate::MethodReceiver::Associated => e.byte(1),
        crate::MethodReceiver::Component => e.byte(2),
    }
    e.number(method.owner_params.len());
    for param in &method.owner_params {
        e.text(&param.name);
        e.byte(match param.kind {
            crate::GenericParamKind::Type => 0,
            crate::GenericParamKind::Effect => 1,
        });
        e.number(param.bounds.len());
        for bound in &param.bounds {
            e.path(&bound.spec);
            e.number(bound.args.len());
            for ty in &bound.args {
                e.encode(Task::Type(ty));
            }
        }
    }
    e.0.update(callable_value_graph_input_fingerprint(&method.callable).as_bytes());
    etas_code::CodeFingerprint::from_bytes(*e.0.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph() -> CallableValueGraph {
        CallableValueGraph {
            sources: vec![CallableValueSource::Value(0)],
            values: vec![CallableValueNode {
                residual_checks: Vec::new(),
                handler_requests: None,
                behavior: CallableValueBehavior {
                    determinism: CallableValueDeterminism::Deterministic,
                    independent_determinism: None,
                    host_requirements: vec![],
                    interpreter_features: vec![],
                },
                parent_scope: CallableValueScope::Root,
                generic_params: vec![],
                generic_bindings: vec![],
                param_names: vec![],
                params: vec![],
                output: Type {
                    name: "unit".into(),
                    ..Default::default()
                },
                public_effects: EffectRow::default(),
                requested_actions: EffectRow::default(),
                handled_requested_actions: EffectRow::default(),
                default_handler_effects: Default::default(),
                parameter_bindings: vec![],
                action_trace: ActionTrace::Empty,
                returned_sources: None,
            }],
        }
    }

    #[test]
    fn structural_fingerprint_tracks_behavior_scopes_rows_and_trace() {
        let original = graph();
        let before = callable_value_graph_input_fingerprint(&original);
        let mut changed = original.clone();
        changed.sources[0] = CallableValueSource::FrozenContract(0);
        assert_ne!(before, callable_value_graph_input_fingerprint(&changed));
        changed = original.clone();
        changed.values[0].behavior.independent_determinism =
            Some(CallableValueDeterminism::Deterministic);
        assert_ne!(before, callable_value_graph_input_fingerprint(&changed));
        changed = original.clone();
        changed.values[0].requested_actions.tail = Some("E".into());
        assert_ne!(before, callable_value_graph_input_fingerprint(&changed));
        changed = original.clone();
        changed.values[0].action_trace = ActionTrace::ParameterCall {
            parameter: "f".into(),
        };
        assert_ne!(before, callable_value_graph_input_fingerprint(&changed));
        changed = original.clone();
        changed.values[0].handler_requests = Some(Default::default());
        assert_ne!(before, callable_value_graph_input_fingerprint(&changed));
        changed = original.clone();
        changed.values[0].returned_sources = Some(vec![]);
        assert_ne!(before, callable_value_graph_input_fingerprint(&changed));
        changed = original;
        changed.sources[0] = CallableValueSource::Value(u32::MAX);
        assert_ne!(before, callable_value_graph_input_fingerprint(&changed));
        assert!(changed.validate_references(None).is_err());
    }

    #[test]
    fn deeply_nested_types_are_traversed_iteratively() {
        let mut graph = graph();
        let mut ty = Type::default();
        for _ in 0..10_000 {
            ty = Type {
                kind: TypeKind::Array,
                children: vec![ty],
                ..Default::default()
            };
        }
        graph.values[0].output = ty;
        let fingerprint = callable_value_graph_input_fingerprint(&graph);
        assert_eq!(fingerprint, callable_value_graph_input_fingerprint(&graph));
        std::mem::forget(graph);
    }
}
