use std::{collections::BTreeSet, fmt};

use super::traversal::{node_at, node_index, references, validate_expansion};
use crate::{DefinitionKey, DefinitionKind};

use super::{
    CallableOutcomeInfo, DescriptorGraphInput, DescriptorLimits, EffectInfo, EffectRowInfoRef,
    EffectTargetInfo, GenericOwner, GenericParameterKind, GenericParameterRef, PrimitiveInfo,
    ReturnObservationInfo, SignatureInfo, StaticEffectArgInfo, TraceInfo, TraceInfoRef, TypeInfo,
    TypeInfoRef, VariantPayloadInfo,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DescriptorNode {
    Type(TypeInfoRef),
    Row(EffectRowInfoRef),
    Binder(GenericParameterRef),
    Trace(TraceInfoRef),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DescriptorError {
    InvalidDescription {
        reason: &'static str,
    },
    Limit {
        resource: &'static str,
        limit: usize,
    },
    MissingReference {
        from: DescriptorNode,
        target: DescriptorNode,
    },
    Invalid {
        node: DescriptorNode,
        reason: &'static str,
    },
    DuplicateName {
        node: DescriptorNode,
        name: String,
    },
    UnguardedCycle {
        node: DescriptorNode,
    },
}

impl fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescription { reason } => write!(f, "invalid descriptor: {reason}"),
            Self::Limit { resource, limit } => {
                write!(f, "descriptor {resource} limit exceeded ({limit})")
            }
            Self::MissingReference { from, target } => {
                write!(f, "descriptor {from:?} references missing {target:?}")
            }
            Self::Invalid { node, reason } => write!(f, "invalid descriptor {node:?}: {reason}"),
            Self::DuplicateName { node, name } => {
                write!(f, "duplicate name {name:?} in descriptor {node:?}")
            }
            Self::UnguardedCycle { node } => {
                write!(f, "unguarded structural/alias cycle at descriptor {node:?}")
            }
        }
    }
}

impl std::error::Error for DescriptorError {}

pub(super) fn validate(
    input: &DescriptorGraphInput,
    limits: DescriptorLimits,
) -> Result<(), DescriptorError> {
    let count = input
        .types
        .len()
        .checked_add(input.rows.len())
        .and_then(|n| n.checked_add(input.binders.len()))
        .and_then(|n| n.checked_add(input.traces.len()))
        .ok_or(DescriptorError::Limit {
            resource: "nodes",
            limit: limits.max_nodes,
        })?;
    if count > limits.max_nodes || count > u32::MAX as usize {
        return Err(DescriptorError::Limit {
            resource: "nodes",
            limit: limits.max_nodes.min(u32::MAX as usize),
        });
    }
    let mut validator = Validator {
        input,
        limits,
        text_bytes: 0,
        edge_count: 0,
    };
    let mut binder_identities = BTreeSet::new();
    let mut binder_names = BTreeSet::new();
    for (index, binder) in input.binders.iter().enumerate() {
        let node = DescriptorNode::Binder(GenericParameterRef(index as u32));
        validator.name(node, &binder.name)?;
        match &binder.owner {
            GenericOwner::Declaration(key) => validator.identity(key)?,
            GenericOwner::Method(method) => {
                validator.text(method.name())?;
                let implementation = method.implementation();
                for segment in implementation.module() {
                    validator.text(segment)?;
                }
                validator.identity(implementation.subject())?;
                if let Some(spec) = implementation.selected_spec() {
                    validator.identity(spec)?;
                }
            }
        }
        if !binder_identities.insert((&binder.owner, binder.position))
            || !binder_names.insert((&binder.owner, &binder.name))
        {
            return Err(DescriptorError::Invalid {
                node,
                reason: "duplicate generic binder identity or name",
            });
        }
        if binder.kind == GenericParameterKind::EffectRow && !binder.bounds.is_empty() {
            return Err(DescriptorError::Invalid {
                node,
                reason: "effect-row binder has type-spec bounds",
            });
        }
        for bound in &binder.bounds {
            validator.kind(node, &bound.definition, DefinitionKind::Spec)?;
        }
    }
    for (index, ty) in input.types.iter().enumerate() {
        let node = DescriptorNode::Type(TypeInfoRef(index as u32));
        match ty {
            TypeInfo::Primitive(_) | TypeInfo::Tuple(_) => {}
            TypeInfo::Builtin {
                constructor,
                arguments,
            } => {
                if constructor.arity() != arguments.len() {
                    return Err(DescriptorError::Invalid {
                        node,
                        reason: "builtin type generic arity differs from its constructor",
                    });
                }
            }
            TypeInfo::Parameter(parameter) => {
                validator.binder_kind(node, *parameter, GenericParameterKind::Type)?
            }
            TypeInfo::Record(fields) => {
                validator.names(node, fields.iter().map(|field| field.name.as_str()))?
            }
            TypeInfo::Nominal {
                definition,
                parameters,
                ..
            }
            | TypeInfo::Alias {
                definition,
                parameters,
                ..
            }
            | TypeInfo::Enum {
                definition,
                parameters,
                ..
            } => {
                if matches!(ty, TypeInfo::Nominal { .. })
                    && definition.kind() == DefinitionKind::Agent
                {
                    validator.identity(definition)?;
                    if !parameters.is_empty() {
                        return Err(DescriptorError::Invalid {
                            node,
                            reason: "nominal agent component cannot declare type parameters",
                        });
                    }
                } else {
                    validator.kind(node, definition, DefinitionKind::Type)?;
                }
                let mut seen = BTreeSet::new();
                for parameter in parameters {
                    validator.binder_kind(node, *parameter, GenericParameterKind::Type)?;
                    let binder = &input.binders[parameter.0 as usize];
                    if binder.owner != GenericOwner::Declaration(definition.clone())
                        || !seen.insert(*parameter)
                    {
                        return Err(DescriptorError::Invalid {
                            node,
                            reason: "nominal/alias generic binder has a different owner or is duplicated",
                        });
                    }
                }
                if let TypeInfo::Enum { variants, .. } = ty {
                    validator.names(node, variants.iter().map(|variant| variant.name.as_str()))?;
                    for variant in variants {
                        if let VariantPayloadInfo::Named(fields) = &variant.payload {
                            validator
                                .names(node, fields.iter().map(|field| field.name.as_str()))?;
                        }
                    }
                }
            }
            TypeInfo::Applied {
                constructor,
                arguments,
            } => {
                let target = input.types.get(constructor.0 as usize).ok_or(
                    DescriptorError::MissingReference {
                        from: node,
                        target: DescriptorNode::Type(*constructor),
                    },
                )?;
                let parameters = match target {
                    TypeInfo::Nominal { parameters, .. }
                    | TypeInfo::Alias { parameters, .. }
                    | TypeInfo::Enum { parameters, .. } => parameters,
                    _ => {
                        return Err(DescriptorError::Invalid {
                            node,
                            reason: "applied type has no nominal/alias constructor",
                        });
                    }
                };
                if parameters.len() != arguments.len() {
                    return Err(DescriptorError::Invalid {
                        node,
                        reason: "applied type generic arity differs from its constructor",
                    });
                }
            }
            TypeInfo::Function(signature) => validator.signature(node, signature)?,
            TypeInfo::Handler { .. } => {}
        }
    }
    for (index, row) in input.rows.iter().enumerate() {
        let node = DescriptorNode::Row(EffectRowInfoRef(index as u32));
        if let Some(tail) = row.tail {
            validator.binder_kind(node, tail, GenericParameterKind::EffectRow)?;
        }
        for effect in &row.effects {
            validator.effect(node, effect)?;
        }
    }
    for (index, trace) in input.traces.iter().enumerate() {
        let node = DescriptorNode::Trace(TraceInfoRef(index as u32));
        let calls = match trace {
            TraceInfo::Empty | TraceInfo::Seq(_) | TraceInfo::Choice(_) | TraceInfo::Repeat(_) => {
                &[][..]
            }
            TraceInfo::Outcome(outcome) => {
                if let CallableOutcomeInfo::Returned { ty, observation } = outcome {
                    validator.return_observation(node, *ty, observation)?;
                }
                &[][..]
            }
            TraceInfo::Event(effect) => {
                validator.effect(node, effect)?;
                if !matches!(effect.target, EffectTargetInfo::Action { .. }) {
                    return Err(DescriptorError::Invalid {
                        node,
                        reason: "trace event must name an action",
                    });
                }
                &[][..]
            }
            TraceInfo::ParameterCall(call) => std::slice::from_ref(call),
            TraceInfo::UnknownOrder(actions) | TraceInfo::Widened { actions, .. } => {
                let row = input.rows.get(actions.0 as usize).ok_or(
                    DescriptorError::MissingReference {
                        from: node,
                        target: DescriptorNode::Row(*actions),
                    },
                )?;
                if row.tail.is_some()
                    || row
                        .effects
                        .iter()
                        .any(|effect| !matches!(effect.target, EffectTargetInfo::Action { .. }))
                {
                    return Err(DescriptorError::Invalid {
                        node,
                        reason: "trace footprint must contain only concrete actions",
                    });
                }
                match trace {
                    TraceInfo::Widened {
                        parameter_calls, ..
                    } => parameter_calls,
                    _ => &[][..],
                }
            }
        };
        validator.edge_count = validator.edge_count.saturating_add(calls.len());
        if validator.edge_count > limits.max_edges {
            return Err(DescriptorError::Limit {
                resource: "edges",
                limit: limits.max_edges,
            });
        }
        for call in calls {
            let signature = input.types.get(call.signature.0 as usize).ok_or(
                DescriptorError::MissingReference {
                    from: node,
                    target: DescriptorNode::Type(call.signature),
                },
            )?;
            let TypeInfo::Function(owner) = signature else {
                return Err(DescriptorError::Invalid {
                    node,
                    reason: "trace parameter call has no callable signature parameter",
                });
            };
            let Some(parameter) = owner.parameters.get(call.position as usize) else {
                return Err(DescriptorError::Invalid {
                    node,
                    reason: "trace parameter call has no callable signature parameter",
                });
            };
            let mut parameter_type = parameter.ty;
            let mut visited = BTreeSet::new();
            loop {
                validator.edge_count = validator.edge_count.saturating_add(1);
                if validator.edge_count > limits.max_edges {
                    return Err(DescriptorError::Limit {
                        resource: "edges",
                        limit: limits.max_edges,
                    });
                }
                if !visited.insert(parameter_type) {
                    return Err(DescriptorError::Invalid {
                        node,
                        reason: "trace parameter call has a cyclic callable alias",
                    });
                }
                match input.types.get(parameter_type.0 as usize) {
                    Some(TypeInfo::Function(_)) => break,
                    Some(TypeInfo::Alias { target, .. }) => parameter_type = *target,
                    Some(_) => {
                        return Err(DescriptorError::Invalid {
                            node,
                            reason: "trace parameter call requires a checked callback function",
                        });
                    }
                    None => {
                        return Err(DescriptorError::MissingReference {
                            from: node,
                            target: DescriptorNode::Type(parameter_type),
                        });
                    }
                }
            }
            validator.callable(node, &call.callable)?;
        }
    }
    let mut adjacency = vec![Vec::new(); count];
    for (index, children) in adjacency.iter_mut().enumerate() {
        let node = node_at(input, index);
        let mut failure = None;
        references(input, node, true, |target| {
            if failure.is_some() {
                return;
            }
            validator.edge_count += 1;
            if validator.edge_count > limits.max_edges {
                failure = Some(DescriptorError::Limit {
                    resource: "edges",
                    limit: limits.max_edges,
                });
            } else if node_index(input, target).is_none() {
                failure = Some(DescriptorError::MissingReference { from: node, target });
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
        references(input, node, false, |target| {
            // All references were checked above, before expansion traversal.
            if let Some(target_index) = node_index(input, target) {
                children.push(target_index);
            }
        });
    }
    validate_expansion(input, &adjacency, limits.max_expansion_depth)
}

struct Validator<'a> {
    input: &'a DescriptorGraphInput,
    limits: DescriptorLimits,
    text_bytes: usize,
    edge_count: usize,
}

impl Validator<'_> {
    fn return_observation(
        &mut self,
        node: DescriptorNode,
        ty: TypeInfoRef,
        observation: &ReturnObservationInfo,
    ) -> Result<(), DescriptorError> {
        let mut current = ty;
        let mut visited = BTreeSet::new();
        while visited.insert(current) {
            match self.input.types.get(current.0 as usize) {
                Some(TypeInfo::Alias { target, .. }) => current = *target,
                Some(TypeInfo::Primitive(PrimitiveInfo::Never)) => break,
                Some(_) if matches!(observation, ReturnObservationInfo::Any) => return Ok(()),
                Some(TypeInfo::Applied { constructor, .. })
                    if matches!(observation, ReturnObservationInfo::Variant { .. }) =>
                {
                    current = *constructor
                }
                Some(TypeInfo::Primitive(PrimitiveInfo::Unit))
                    if matches!(observation, ReturnObservationInfo::Unit) =>
                {
                    return Ok(());
                }
                Some(TypeInfo::Primitive(PrimitiveInfo::Bool))
                    if matches!(observation, ReturnObservationInfo::Bool(_)) =>
                {
                    return Ok(());
                }
                Some(TypeInfo::Enum {
                    definition,
                    variants,
                    ..
                }) => {
                    if let ReturnObservationInfo::Variant { owner, name } = observation
                        && definition == owner
                        && variants.iter().any(|variant| variant.name == *name)
                    {
                        self.identity(owner)?;
                        return self.name(node, name);
                    }
                    break;
                }
                None => {
                    return Err(DescriptorError::MissingReference {
                        from: node,
                        target: DescriptorNode::Type(current),
                    });
                }
                _ => break,
            }
        }
        Err(DescriptorError::Invalid {
            node,
            reason: "return observation does not match its checked result type",
        })
    }

    fn effect(&mut self, node: DescriptorNode, effect: &EffectInfo) -> Result<(), DescriptorError> {
        let identity = match &effect.target {
            EffectTargetInfo::Effect(key) => key,
            EffectTargetInfo::Action { effect, name } => {
                self.name(node, name)?;
                effect
            }
        };
        self.kind(node, identity, DefinitionKind::Effect)?;
        for argument in &effect.arguments {
            match argument {
                StaticEffectArgInfo::Resource(key) => self.identity(key)?,
                StaticEffectArgInfo::Callable(id) => self.callable(node, id)?,
                StaticEffectArgInfo::String(value) | StaticEffectArgInfo::Integer(value) => {
                    self.text(value)?
                }
                StaticEffectArgInfo::Type(_) | StaticEffectArgInfo::Wildcard => {}
            }
        }
        Ok(())
    }

    fn text(&mut self, value: &str) -> Result<(), DescriptorError> {
        self.text_bytes =
            self.text_bytes
                .checked_add(value.len())
                .ok_or(DescriptorError::Limit {
                    resource: "text bytes",
                    limit: self.limits.max_text_bytes,
                })?;
        if self.text_bytes > self.limits.max_text_bytes {
            return Err(DescriptorError::Limit {
                resource: "text bytes",
                limit: self.limits.max_text_bytes,
            });
        }
        Ok(())
    }
    fn name(&mut self, node: DescriptorNode, value: &str) -> Result<(), DescriptorError> {
        if value.is_empty() || value.contains('\0') {
            return Err(DescriptorError::Invalid {
                node,
                reason: "empty or invalid name",
            });
        }
        self.text(value)
    }
    fn names<'a>(
        &mut self,
        node: DescriptorNode,
        values: impl Iterator<Item = &'a str>,
    ) -> Result<(), DescriptorError> {
        let mut seen = BTreeSet::new();
        for value in values {
            self.name(node, value)?;
            if !seen.insert(value) {
                return Err(DescriptorError::DuplicateName {
                    node,
                    name: value.to_owned(),
                });
            }
        }
        Ok(())
    }
    fn callable(
        &mut self,
        node: DescriptorNode,
        callable: &crate::CallableId,
    ) -> Result<(), DescriptorError> {
        if let Some(declaration) = callable.as_declaration() {
            self.identity(declaration)?;
        } else if let Some(method) = callable.as_method() {
            let implementation = method.implementation();
            self.identity(implementation.subject())?;
            if let Some(spec) = implementation.selected_spec() {
                self.kind(node, spec, DefinitionKind::Spec)?;
            }
            for segment in implementation.module() {
                self.text(segment)?;
            }
            self.name(node, method.name())?;
        }
        Ok(())
    }

    fn identity(&mut self, key: &DefinitionKey) -> Result<(), DescriptorError> {
        for segment in key.module().iter().chain(key.declaration()) {
            self.text(segment)?;
        }
        Ok(())
    }
    fn kind(
        &mut self,
        node: DescriptorNode,
        key: &DefinitionKey,
        kind: DefinitionKind,
    ) -> Result<(), DescriptorError> {
        if key.kind() != kind {
            return Err(DescriptorError::Invalid {
                node,
                reason: "declaration identity has the wrong kind",
            });
        }
        self.identity(key)
    }
    fn binder_kind(
        &self,
        node: DescriptorNode,
        parameter: GenericParameterRef,
        kind: GenericParameterKind,
    ) -> Result<(), DescriptorError> {
        let binder = self.input.binders.get(parameter.0 as usize).ok_or(
            DescriptorError::MissingReference {
                from: node,
                target: DescriptorNode::Binder(parameter),
            },
        )?;
        if binder.kind != kind {
            return Err(DescriptorError::Invalid {
                node,
                reason: "generic binder has the wrong kind",
            });
        }
        Ok(())
    }
    fn signature(
        &mut self,
        node: DescriptorNode,
        signature: &SignatureInfo,
    ) -> Result<(), DescriptorError> {
        self.names(
            node,
            signature
                .parameters
                .iter()
                .filter_map(|parameter| parameter.name.as_deref()),
        )?;
        let mut generics = BTreeSet::new();
        for generic in &signature.generics {
            if !generics.insert(*generic) {
                return Err(DescriptorError::Invalid {
                    node,
                    reason: "signature repeats a generic binder",
                });
            }
        }
        Ok(())
    }
}
