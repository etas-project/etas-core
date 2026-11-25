use std::collections::{BTreeMap, BTreeSet};

use super::{
    BuiltinTypeInfo, DescriptorError, DescriptorLimits, DescriptorNode, GenericParameterRef,
    PrimitiveInfo, SignatureInfo, TypeInfo, TypeInfoRef, VariantPayloadInfo,
};

/// A position in a checked value, independent of compiler arena identifiers.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub enum ValueProjectionStep {
    Field(String),
    TupleElement(u32),
    VariantPayload {
        variant: String,
        position: u32,
    },
    SequenceElement,
    ListHead,
    ListTail,
    MapKey,
    MapValue,
    /// Origins produced by invoking this callable, not the callable itself.
    CallResult,
    /// The result of an application with checked inputs in its value graph.
    CallWithArguments {
        inputs: u32,
    },
    /// This callable with the checked inputs of a particular invocation.
    Invocation {
        inputs: u32,
    },
    /// Zero or more traversals through checked, type-preserving alternatives.
    /// This is the finite closure of a loop's value dependencies.
    Repeat {
        alternatives: Vec<Vec<ValueProjectionStep>>,
    },
}

impl ValueProjectionStep {
    pub fn call_inputs(path: &[Self]) -> BTreeSet<u32> {
        let mut result = BTreeSet::new();
        let mut pending = path.iter().collect::<Vec<_>>();
        while let Some(step) = pending.pop() {
            match step {
                Self::CallWithArguments { inputs } | Self::Invocation { inputs } => {
                    result.insert(*inputs);
                }
                Self::Repeat { alternatives } => pending.extend(alternatives.iter().flatten()),
                _ => {}
            }
        }
        result
    }

    pub fn map_call_inputs<E>(
        path: &mut [Self],
        map: &mut impl FnMut(u32) -> Result<u32, E>,
    ) -> Result<(), E> {
        for step in path {
            match step {
                Self::CallWithArguments { inputs } | Self::Invocation { inputs } => {
                    *inputs = map(*inputs)?
                }
                Self::Repeat { alternatives } => {
                    for path in alternatives.iter_mut() {
                        Self::map_call_inputs(path, map)?;
                    }
                    alternatives.sort_unstable();
                    alternatives.dedup();
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn nodes(path: &[Self]) -> usize {
        let mut count = 0usize;
        let mut pending = path.iter().collect::<Vec<_>>();
        while let Some(step) = pending.pop() {
            count = count.saturating_add(1);
            if let Self::Repeat { alternatives } = step {
                count = count.saturating_add(alternatives.len());
                pending.extend(alternatives.iter().flatten());
            }
        }
        count
    }

    pub fn work(path: &[Self]) -> usize {
        let mut count = Self::nodes(path);
        let mut pending = path.iter().collect::<Vec<_>>();
        while let Some(step) = pending.pop() {
            match step {
                Self::Field(name) | Self::VariantPayload { variant: name, .. } => {
                    count = count.saturating_add(name.len())
                }
                Self::Repeat { alternatives } => pending.extend(alternatives.iter().flatten()),
                _ => {}
            }
        }
        count
    }

    pub fn well_formed(path: &[Self], max_depth: usize, max_nodes: usize) -> bool {
        let mut pending = vec![(path, 0usize)];
        let mut count = 0usize;
        while let Some((path, depth)) = pending.pop() {
            if depth.saturating_add(path.len()) > max_depth {
                return false;
            }
            count = count.saturating_add(path.len());
            if count > max_nodes {
                return false;
            }
            for step in path {
                match step {
                    Self::Field(name) | Self::VariantPayload { variant: name, .. }
                        if name.is_empty() =>
                    {
                        return false;
                    }
                    Self::Repeat { alternatives } => {
                        if alternatives.is_empty()
                            || alternatives.iter().any(Vec::is_empty)
                            || alternatives.windows(2).any(|pair| pair[0] >= pair[1])
                        {
                            return false;
                        }
                        pending
                            .extend(alternatives.iter().map(|path| (path.as_slice(), depth + 1)));
                    }
                    _ => {}
                }
            }
        }
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ScopedType {
    ty: TypeInfoRef,
    frame: usize,
}

#[derive(Clone, Debug, Default)]
struct Frame {
    parent: Option<usize>,
    bindings: BTreeMap<GenericParameterRef, ScopedType>,
}

/// Applied declarations introduce a new binder scope. Arguments retain their
/// original scope, including when a recursive declaration reuses its binders.
#[derive(Clone)]
pub(super) struct ProjectedType {
    current: ScopedType,
    frames: Vec<Frame>,
}

impl ProjectedType {
    pub fn resolve(
        input: &[TypeInfo],
        node: DescriptorNode,
        root: TypeInfoRef,
        path: &[ValueProjectionStep],
        limits: DescriptorLimits,
        work: &mut usize,
    ) -> Result<Self, DescriptorError> {
        if path.len() > limits.max_expansion_depth {
            return Err(DescriptorError::Limit {
                resource: "value projection depth",
                limit: limits.max_expansion_depth,
            });
        }
        if !ValueProjectionStep::well_formed(path, limits.max_expansion_depth, limits.max_edges) {
            return Err(DescriptorError::Invalid {
                node,
                reason: "malformed value projection path",
            });
        }
        let mut result = Self {
            current: ScopedType { ty: root, frame: 0 },
            frames: vec![Frame::default()],
        };
        result.follow(input, node, path, limits, work)?;
        result.current = result.head(input, node, result.current, false, limits, work)?;
        while let Some(TypeInfo::Builtin {
            constructor: BuiltinTypeInfo::CallableDefinition,
            arguments,
        }) = input.get(result.current.ty.0 as usize)
        {
            let [contract] = arguments.as_slice() else {
                return Err(DescriptorError::Invalid {
                    node,
                    reason: "invalid projected callable definition",
                });
            };
            result.current = result.head(
                input,
                node,
                ScopedType {
                    ty: *contract,
                    frame: result.current.frame,
                },
                false,
                limits,
                work,
            )?;
        }
        Ok(result)
    }

    fn follow(
        &mut self,
        input: &[TypeInfo],
        node: DescriptorNode,
        path: &[ValueProjectionStep],
        limits: DescriptorLimits,
        work: &mut usize,
    ) -> Result<(), DescriptorError> {
        for step in path {
            if let ValueProjectionStep::Repeat { alternatives } = step {
                if alternatives.is_empty() || alternatives.iter().any(Vec::is_empty) {
                    return Err(DescriptorError::Invalid {
                        node,
                        reason: "empty repeated value projection",
                    });
                }
                for alternative in alternatives {
                    let mut trial = self.clone();
                    trial.follow(input, node, alternative, limits, work)?;
                    if !trial.same_type(input, node, self.current, trial.current, limits, work)? {
                        return Err(DescriptorError::Invalid {
                            node,
                            reason: "repeated value projection must preserve its checked type",
                        });
                    }
                }
                continue;
            }
            charge(limits, work, 1)?;
            self.current = self.head(input, node, self.current, true, limits, work)?;
            let current = self.current;
            let ty = input
                .get(current.ty.0 as usize)
                .ok_or(DescriptorError::MissingReference {
                    from: node,
                    target: DescriptorNode::Type(current.ty),
                })?;
            charge(
                limits,
                work,
                match ty {
                    TypeInfo::Record(fields) => fields.len(),
                    TypeInfo::Enum { variants, .. } => variants.len(),
                    _ => 0,
                },
            )?;
            let selected = match (step, ty) {
                (ValueProjectionStep::Invocation { .. }, TypeInfo::Function(_)) => Some(current.ty),
                (
                    ValueProjectionStep::CallResult | ValueProjectionStep::CallWithArguments { .. },
                    TypeInfo::Function(signature),
                ) => Some(signature.output),
                (ValueProjectionStep::Field(name), TypeInfo::Record(fields)) => fields
                    .iter()
                    .find(|field| field.name == *name)
                    .map(|field| field.ty),
                (ValueProjectionStep::TupleElement(position), TypeInfo::Tuple(elements)) => {
                    elements.get(*position as usize).copied()
                }
                (
                    ValueProjectionStep::VariantPayload { variant, position },
                    TypeInfo::Enum { variants, .. },
                ) => variants
                    .iter()
                    .find(|candidate| candidate.name == *variant)
                    .and_then(|variant| match &variant.payload {
                        VariantPayloadInfo::None => None,
                        VariantPayloadInfo::Positional(elements) => {
                            elements.get(*position as usize).copied()
                        }
                        VariantPayloadInfo::Named(fields) => {
                            fields.get(*position as usize).map(|field| field.ty)
                        }
                    }),
                (
                    ValueProjectionStep::VariantPayload {
                        variant,
                        position: 0,
                    },
                    TypeInfo::Builtin {
                        constructor: BuiltinTypeInfo::Option,
                        arguments,
                    },
                ) if variant == "Some" => arguments.first().copied(),
                (
                    ValueProjectionStep::VariantPayload {
                        variant,
                        position: 0,
                    },
                    TypeInfo::Builtin {
                        constructor: BuiltinTypeInfo::Result,
                        arguments,
                    },
                ) if variant == "Ok" || variant == "Err" => {
                    arguments.get(usize::from(variant == "Err")).copied()
                }
                (
                    ValueProjectionStep::SequenceElement,
                    TypeInfo::Builtin {
                        constructor:
                            BuiltinTypeInfo::Array
                            | BuiltinTypeInfo::List
                            | BuiltinTypeInfo::Slice
                            | BuiltinTypeInfo::Set,
                        arguments,
                    },
                )
                | (
                    ValueProjectionStep::ListHead,
                    TypeInfo::Builtin {
                        constructor: BuiltinTypeInfo::List,
                        arguments,
                    },
                ) => arguments.first().copied(),
                (
                    ValueProjectionStep::ListTail,
                    TypeInfo::Builtin {
                        constructor: BuiltinTypeInfo::List,
                        ..
                    },
                ) => Some(current.ty),
                (
                    ValueProjectionStep::MapKey,
                    TypeInfo::Builtin {
                        constructor: BuiltinTypeInfo::Map,
                        arguments,
                    },
                ) => arguments.first().copied(),
                (
                    ValueProjectionStep::MapValue,
                    TypeInfo::Builtin {
                        constructor: BuiltinTypeInfo::Map,
                        arguments,
                    },
                ) => arguments.get(1).copied(),
                _ => None,
            }
            .ok_or(DescriptorError::Invalid {
                node,
                reason: "value projection does not select a checked field or payload",
            })?;
            self.current = ScopedType {
                ty: selected,
                frame: current.frame,
            };
        }
        Ok(())
    }

    fn same_type(
        &mut self,
        input: &[TypeInfo],
        node: DescriptorNode,
        a: ScopedType,
        b: ScopedType,
        limits: DescriptorLimits,
        work: &mut usize,
    ) -> Result<bool, DescriptorError> {
        let mut pending = vec![(a, b)];
        let mut seen = BTreeSet::new();
        while let Some((a, b)) = pending.pop() {
            let a = self.head(input, node, a, false, limits, work)?;
            let b = self.head(input, node, b, false, limits, work)?;
            if a == b || !seen.insert((a, b)) {
                continue;
            }
            if a.ty != b.ty {
                return Ok(false);
            }
            let mut children = Vec::new();
            match &input[a.ty.0 as usize] {
                TypeInfo::Nominal { parameters, .. } | TypeInfo::Enum { parameters, .. } => {
                    for parameter in parameters {
                        match (
                            self.binding(a.frame, *parameter, limits, work)?,
                            self.binding(b.frame, *parameter, limits, work)?,
                        ) {
                            (Some(left), Some(right)) => pending.push((left, right)),
                            (None, None) => {}
                            (Some(bound), None) | (None, Some(bound)) => {
                                let bound = self.head(input, node, bound, false, limits, work)?;
                                if !matches!(input.get(bound.ty.0 as usize),Some(TypeInfo::Parameter(actual)) if actual == parameter)
                                {
                                    return Ok(false);
                                }
                            }
                        }
                    }
                }
                TypeInfo::Builtin { arguments, .. } | TypeInfo::Tuple(arguments) => {
                    children.extend(arguments)
                }
                TypeInfo::Record(fields) => children.extend(fields.iter().map(|field| &field.ty)),
                TypeInfo::Function(signature) => {
                    children.extend(signature.parameters.iter().map(|parameter| &parameter.ty));
                    children.push(&signature.output);
                }
                TypeInfo::Task { output, .. } => children.push(output),
                _ => {}
            }
            pending.extend(children.into_iter().map(|ty| {
                (
                    ScopedType {
                        ty: *ty,
                        frame: a.frame,
                    },
                    ScopedType {
                        ty: *ty,
                        frame: b.frame,
                    },
                )
            }));
        }
        Ok(true)
    }

    pub fn signature(
        &self,
        input: &[TypeInfo],
        node: DescriptorNode,
    ) -> Result<SignatureInfo, DescriptorError> {
        match input.get(self.current.ty.0 as usize) {
            Some(TypeInfo::Function(signature)) => Ok(signature.clone()),
            _ => Err(DescriptorError::Invalid {
                node,
                reason: "projected callback is not a checked function",
            }),
        }
    }

    pub fn scalar(
        &mut self,
        input: &[TypeInfo],
        node: DescriptorNode,
        ty: TypeInfoRef,
        limits: DescriptorLimits,
        work: &mut usize,
    ) -> Result<PrimitiveInfo, DescriptorError> {
        let resolved = self.head(
            input,
            node,
            ScopedType {
                ty,
                frame: self.current.frame,
            },
            false,
            limits,
            work,
        )?;
        match input.get(resolved.ty.0 as usize) {
            Some(TypeInfo::Primitive(primitive))
                if !matches!(
                    primitive,
                    PrimitiveInfo::Never | PrimitiveInfo::String | PrimitiveInfo::Bytes
                ) =>
            {
                Ok(*primitive)
            }
            _ => Err(DescriptorError::Invalid {
                node,
                reason: "projected callback argument requires a checked scalar type",
            }),
        }
    }

    fn binding(
        &self,
        mut frame: usize,
        parameter: GenericParameterRef,
        limits: DescriptorLimits,
        work: &mut usize,
    ) -> Result<Option<ScopedType>, DescriptorError> {
        loop {
            charge(limits, work, 1)?;
            let scope = &self.frames[frame];
            if let Some(value) = scope.bindings.get(&parameter) {
                return Ok(Some(*value));
            }
            let Some(parent) = scope.parent else {
                return Ok(None);
            };
            frame = parent;
        }
    }

    fn head(
        &mut self,
        input: &[TypeInfo],
        node: DescriptorNode,
        mut current: ScopedType,
        representation: bool,
        limits: DescriptorLimits,
        work: &mut usize,
    ) -> Result<ScopedType, DescriptorError> {
        let mut visited = BTreeSet::new();
        loop {
            charge(limits, work, 1)?;
            if !visited.insert(current) {
                return Err(DescriptorError::Invalid {
                    node,
                    reason: "value projection has a cyclic type binding",
                });
            }
            let ty = input
                .get(current.ty.0 as usize)
                .ok_or(DescriptorError::MissingReference {
                    from: node,
                    target: DescriptorNode::Type(current.ty),
                })?;
            match ty {
                TypeInfo::Parameter(parameter) => {
                    match self.binding(current.frame, *parameter, limits, work)? {
                        Some(value) => current = value,
                        None => return Ok(current),
                    }
                }
                TypeInfo::Alias { target, .. } => current.ty = *target,
                TypeInfo::Nominal {
                    representation: Some(target),
                    ..
                } if representation => current.ty = *target,
                TypeInfo::Applied {
                    constructor,
                    arguments,
                } => {
                    let parameters = match input.get(constructor.0 as usize) {
                        Some(
                            TypeInfo::Nominal { parameters, .. }
                            | TypeInfo::Alias { parameters, .. }
                            | TypeInfo::Enum { parameters, .. },
                        ) if parameters.len() == arguments.len() => parameters,
                        _ => {
                            return Err(DescriptorError::Invalid {
                                node,
                                reason: "value projection has an invalid applied declaration",
                            });
                        }
                    };
                    charge(limits, work, parameters.len())?;
                    let frame = Frame {
                        parent: Some(current.frame),
                        bindings: parameters
                            .iter()
                            .zip(arguments)
                            .map(|(parameter, ty)| {
                                (
                                    *parameter,
                                    ScopedType {
                                        ty: *ty,
                                        frame: current.frame,
                                    },
                                )
                            })
                            .collect(),
                    };
                    current = ScopedType {
                        ty: *constructor,
                        frame: self.frames.len(),
                    };
                    self.frames.push(frame);
                }
                _ => return Ok(current),
            }
        }
    }
}

fn charge(limits: DescriptorLimits, work: &mut usize, count: usize) -> Result<(), DescriptorError> {
    *work = work.saturating_add(count);
    if *work > limits.max_edges {
        return Err(DescriptorError::Limit {
            resource: "edges",
            limit: limits.max_edges,
        });
    }
    Ok(())
}
