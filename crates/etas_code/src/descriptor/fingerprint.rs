use std::collections::BTreeMap;

use crate::{CodeFingerprint, identity::canonical::CanonicalEncoder};

use super::{
    BuiltinTypeInfo, CallableOutcomeInfo, DescriptorError, DescriptorGraph, DescriptorNode,
    EffectInfo, EffectRowInfoRef, EffectTargetInfo, GenericOwner, GenericParameterKind,
    ParameterCallInfo, PrimitiveInfo, ReturnObservationInfo, SignatureInfo, StaticEffectArgInfo,
    TraceInfo, TraceInfoRef, TypeInfo, TypeInfoRef, VariantPayloadInfo,
    traversal::{node_index, references},
};

impl DescriptorGraph {
    /// Identifies the reachable descriptor graph, not semantic type equality.
    /// Encounter numbers are canonical traversal labels, never compiler/local IDs.
    pub fn type_fingerprint(&self, root: TypeInfoRef) -> Result<CodeFingerprint, DescriptorError> {
        self.node_fingerprint(DescriptorNode::Type(root), "etas.code.type-descriptor.v1")
    }

    pub fn row_fingerprint(
        &self,
        root: EffectRowInfoRef,
    ) -> Result<CodeFingerprint, DescriptorError> {
        self.node_fingerprint(DescriptorNode::Row(root), "etas.code.row-descriptor.v1")
    }

    pub fn trace_fingerprint(
        &self,
        root: TraceInfoRef,
    ) -> Result<CodeFingerprint, DescriptorError> {
        self.node_fingerprint(DescriptorNode::Trace(root), "etas.code.trace-descriptor.v1")
    }

    fn node_fingerprint(
        &self,
        root: DescriptorNode,
        domain: &'static str,
    ) -> Result<CodeFingerprint, DescriptorError> {
        if node_index(self.data(), root).is_none() {
            return Err(DescriptorError::MissingReference {
                from: root,
                target: root,
            });
        }
        let mut encoder = CanonicalEncoder::new(domain);
        let mut visited = BTreeMap::new();
        let mut work = vec![root];
        while let Some(node) = work.pop() {
            if let Some(label) = visited.get(&node) {
                encoder.tag(0);
                encoder.number(*label);
                continue;
            }
            let label = visited.len() as u64;
            visited.insert(node, label);
            encoder.tag(1);
            encoder.number(label);
            self.encode_node(node, &mut encoder);
            let mut children = Vec::new();
            references(self.data(), node, true, |child| children.push(child));
            encoder.number(children.len() as u64);
            work.extend(children.into_iter().rev());
        }
        Ok(encoder.finish())
    }

    fn encode_node(&self, node: DescriptorNode, encoder: &mut CanonicalEncoder) {
        match node {
            DescriptorNode::Type(reference) => {
                encoder.tag(1);
                match &self.data().types[reference.0 as usize] {
                    TypeInfo::Primitive(primitive) => {
                        encoder.tag(1);
                        encoder.tag(primitive_tag(*primitive));
                    }
                    TypeInfo::Builtin {
                        constructor,
                        arguments,
                    } => {
                        encoder.tag(11);
                        encoder.tag(builtin_tag(*constructor));
                        encoder.number(arguments.len() as u64);
                    }
                    TypeInfo::Parameter(_) => encoder.tag(2),
                    TypeInfo::Tuple(elements) => {
                        encoder.tag(3);
                        encoder.number(elements.len() as u64);
                    }
                    TypeInfo::Record(fields) => {
                        encoder.tag(4);
                        encoder.number(fields.len() as u64);
                        for field in fields {
                            encoder.bytes(field.name.as_bytes());
                        }
                    }
                    TypeInfo::Nominal {
                        definition,
                        parameters,
                        representation,
                    } => {
                        encoder.tag(5);
                        encoder.fingerprint(definition.fingerprint());
                        encoder.number(parameters.len() as u64);
                        encoder.tag(u8::from(representation.is_some()));
                    }
                    TypeInfo::Alias {
                        definition,
                        parameters,
                        ..
                    } => {
                        encoder.tag(6);
                        encoder.fingerprint(definition.fingerprint());
                        encoder.number(parameters.len() as u64);
                    }
                    TypeInfo::Enum {
                        definition,
                        parameters,
                        variants,
                    } => {
                        encoder.tag(7);
                        encoder.fingerprint(definition.fingerprint());
                        encoder.number(parameters.len() as u64);
                        encoder.number(variants.len() as u64);
                        for variant in variants {
                            encoder.bytes(variant.name.as_bytes());
                            match &variant.payload {
                                VariantPayloadInfo::None => encoder.tag(0),
                                VariantPayloadInfo::Positional(types) => {
                                    encoder.tag(1);
                                    encoder.number(types.len() as u64);
                                }
                                VariantPayloadInfo::Named(fields) => {
                                    encoder.tag(2);
                                    encoder.number(fields.len() as u64);
                                    for field in fields {
                                        encoder.bytes(field.name.as_bytes());
                                    }
                                }
                            }
                        }
                    }
                    TypeInfo::Applied { arguments, .. } => {
                        encoder.tag(8);
                        encoder.number(arguments.len() as u64);
                    }
                    TypeInfo::Function(signature) => {
                        encoder.tag(9);
                        encode_signature(encoder, signature);
                    }
                    TypeInfo::Handler {
                        produced, result, ..
                    } => {
                        encoder.tag(10);
                        encoder.tag(u8::from(produced.is_some()));
                        encoder.tag(u8::from(result.is_some()));
                    }
                }
            }
            DescriptorNode::Row(reference) => {
                encoder.tag(2);
                let row = &self.data().rows[reference.0 as usize];
                encoder.tag(u8::from(row.tail.is_some()));
                encoder.number(row.effects.len() as u64);
                for effect in &row.effects {
                    encode_effect(encoder, effect);
                }
            }
            DescriptorNode::Binder(reference) => {
                encoder.tag(3);
                let binder = &self.data().binders[reference.0 as usize];
                match &binder.owner {
                    GenericOwner::Declaration(key) => {
                        encoder.tag(0);
                        encoder.fingerprint(key.fingerprint());
                    }
                    GenericOwner::Method(method) => {
                        encoder.tag(1);
                        encoder.fingerprint(method.fingerprint());
                    }
                }
                encoder.number(u64::from(binder.position));
                encoder.bytes(binder.name.as_bytes());
                encoder.tag(match binder.kind {
                    GenericParameterKind::Type => 0,
                    GenericParameterKind::EffectRow => 1,
                });
                encoder.number(binder.bounds.len() as u64);
                for bound in &binder.bounds {
                    encoder.fingerprint(bound.definition.fingerprint());
                    encoder.number(bound.arguments.len() as u64);
                }
            }
            DescriptorNode::Trace(reference) => {
                encoder.tag(4);
                match &self.data().traces[reference.0 as usize] {
                    TraceInfo::Empty => encoder.tag(0),
                    TraceInfo::Outcome(outcome) => {
                        encoder.tag(8);
                        match outcome {
                            CallableOutcomeInfo::NonReturn => encoder.tag(0),
                            CallableOutcomeInfo::ParameterNonReturn(call) => {
                                encoder.tag(3);
                                encode_parameter_call(encoder, call);
                            }
                            CallableOutcomeInfo::Raised { .. } => encoder.tag(1),
                            CallableOutcomeInfo::Returned { observation, .. } => {
                                encoder.tag(2);
                                match observation {
                                    ReturnObservationInfo::Any => encoder.tag(0),
                                    ReturnObservationInfo::Unit => encoder.tag(1),
                                    ReturnObservationInfo::Bool(value) => {
                                        encoder.tag(2);
                                        encoder.tag(u8::from(*value));
                                    }
                                    ReturnObservationInfo::Variant { owner, name } => {
                                        encoder.tag(3);
                                        encoder.fingerprint(owner.fingerprint());
                                        encoder.bytes(name.as_bytes());
                                    }
                                }
                            }
                        }
                    }
                    TraceInfo::Event(effect) => {
                        encoder.tag(1);
                        encode_effect(encoder, effect);
                    }
                    TraceInfo::ParameterCall(call) => {
                        encoder.tag(2);
                        encode_parameter_call(encoder, call);
                    }
                    TraceInfo::Seq(parts) => {
                        encoder.tag(3);
                        encoder.number(parts.len() as u64);
                    }
                    TraceInfo::Choice(parts) => {
                        encoder.tag(4);
                        encoder.number(parts.len() as u64);
                    }
                    TraceInfo::Repeat(_) => encoder.tag(5),
                    TraceInfo::UnknownOrder(_) => encoder.tag(6),
                    TraceInfo::Widened {
                        parameter_calls, ..
                    } => {
                        encoder.tag(7);
                        encoder.number(parameter_calls.len() as u64);
                        for call in parameter_calls {
                            encode_parameter_call(encoder, call);
                        }
                    }
                }
            }
        }
    }
}

fn encode_parameter_call(encoder: &mut CanonicalEncoder, call: &ParameterCallInfo) {
    encoder.fingerprint(call.callable.fingerprint());
    encoder.number(u64::from(call.position));
    match &call.arguments {
        None => encoder.tag(0),
        Some(arguments) => {
            encoder.tag(1);
            encoder.number(arguments.len() as u64);
            for argument in arguments {
                match argument {
                    super::ParameterArgumentInfo::Bool(value) => {
                        encoder.tag(0);
                        encoder.tag(u8::from(*value));
                    }
                    super::ParameterArgumentInfo::Input { position } => {
                        encoder.tag(1);
                        encoder.number(u64::from(*position));
                    }
                    super::ParameterArgumentInfo::Integer {
                        negative,
                        magnitude,
                    } => {
                        encoder.tag(2);
                        encoder.tag(u8::from(*negative));
                        encoder.bytes(&magnitude.to_le_bytes());
                    }
                }
            }
        }
    }
    match &call.handlers {
        None => encoder.tag(0),
        Some(handlers) => {
            encoder.tag(1);
            encoder.number(handlers.len() as u64);
            for handler in handlers {
                match handler {
                    super::ParameterHandlerScopeInfo::Handle { arms } => {
                        encoder.tag(0);
                        encoder.number(arms.len() as u64);
                        for arm in arms {
                            encode_effect(encoder, &arm.action);
                            encoder.tag(match arm.completion {
                                super::ParameterHandlerCompletionInfo::Resume { .. } => 0,
                                super::ParameterHandlerCompletionInfo::Finish { .. } => 1,
                            });
                            match arm.completion.value() {
                                super::ReturnObservationInfo::Unit => encoder.tag(0),
                                super::ReturnObservationInfo::Bool(value) => {
                                    encoder.tag(1);
                                    encoder.tag(u8::from(*value));
                                }
                                super::ReturnObservationInfo::Any => encoder.tag(2),
                                super::ReturnObservationInfo::Variant { owner, name } => {
                                    encoder.tag(3);
                                    encoder.fingerprint(owner.fingerprint());
                                    encoder.bytes(name.as_bytes());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn encode_effect(encoder: &mut CanonicalEncoder, effect: &EffectInfo) {
    match &effect.target {
        EffectTargetInfo::Effect(key) => {
            encoder.tag(0);
            encoder.fingerprint(key.fingerprint());
        }
        EffectTargetInfo::Action { effect, name } => {
            encoder.tag(1);
            encoder.fingerprint(effect.fingerprint());
            encoder.bytes(name.as_bytes());
        }
    }
    encoder.number(effect.arguments.len() as u64);
    for argument in &effect.arguments {
        match argument {
            StaticEffectArgInfo::Type(_) => encoder.tag(1),
            StaticEffectArgInfo::Resource(key) => {
                encoder.tag(2);
                encoder.fingerprint(key.fingerprint());
            }
            StaticEffectArgInfo::String(value) => {
                encoder.tag(3);
                encoder.bytes(value.as_bytes());
            }
            StaticEffectArgInfo::Integer(value) => {
                encoder.tag(4);
                encoder.bytes(value.as_bytes());
            }
            StaticEffectArgInfo::Wildcard => encoder.tag(5),
            StaticEffectArgInfo::Callable(id) => {
                encoder.tag(6);
                encoder.fingerprint(id.fingerprint());
            }
        }
    }
}

fn encode_signature(encoder: &mut CanonicalEncoder, signature: &SignatureInfo) {
    encoder.number(signature.generics.len() as u64);
    encoder.number(signature.parameters.len() as u64);
    for parameter in &signature.parameters {
        match &parameter.name {
            Some(name) => {
                encoder.tag(1);
                encoder.bytes(name.as_bytes());
            }
            None => encoder.tag(0),
        }
    }
    encoder.tag(u8::from(signature.declared_output.is_some()));
    encoder.tag(u8::from(signature.effects.is_some()));
}

fn primitive_tag(primitive: PrimitiveInfo) -> u8 {
    match primitive {
        PrimitiveInfo::Unit => 0,
        PrimitiveInfo::Never => 1,
        PrimitiveInfo::Bool => 2,
        PrimitiveInfo::I8 => 3,
        PrimitiveInfo::I16 => 4,
        PrimitiveInfo::I32 => 5,
        PrimitiveInfo::I64 => 6,
        PrimitiveInfo::I128 => 18,
        PrimitiveInfo::Isize => 7,
        PrimitiveInfo::U8 => 8,
        PrimitiveInfo::U16 => 9,
        PrimitiveInfo::U32 => 10,
        PrimitiveInfo::U64 => 11,
        PrimitiveInfo::U128 => 19,
        PrimitiveInfo::Usize => 12,
        PrimitiveInfo::F32 => 13,
        PrimitiveInfo::F64 => 14,
        PrimitiveInfo::Char => 15,
        PrimitiveInfo::String => 16,
        PrimitiveInfo::Bytes => 17,
    }
}

fn builtin_tag(constructor: BuiltinTypeInfo) -> u8 {
    match constructor {
        BuiltinTypeInfo::Array => 0,
        BuiltinTypeInfo::List => 1,
        BuiltinTypeInfo::Map => 2,
        BuiltinTypeInfo::Set => 3,
        BuiltinTypeInfo::Range => 4,
        BuiltinTypeInfo::Slice => 5,
        BuiltinTypeInfo::Option => 6,
        BuiltinTypeInfo::Result => 7,
        BuiltinTypeInfo::Schema => 8,
        BuiltinTypeInfo::Prompt => 9,
        BuiltinTypeInfo::PromptPart => 10,
        BuiltinTypeInfo::Message => 11,
        BuiltinTypeInfo::MemorySelection => 12,
        BuiltinTypeInfo::Store => 13,
        BuiltinTypeInfo::MemoryRegion => 14,
        BuiltinTypeInfo::MemoryRegionHandle => 15,
        BuiltinTypeInfo::ExternalToolHandle => 16,
        BuiltinTypeInfo::Trusted => 17,
        BuiltinTypeInfo::Untrusted => 18,
        BuiltinTypeInfo::Secret => 19,
        BuiltinTypeInfo::Public => 20,
        BuiltinTypeInfo::Sanitized => 21,
        BuiltinTypeInfo::CallableDefinition => 22,
    }
}
