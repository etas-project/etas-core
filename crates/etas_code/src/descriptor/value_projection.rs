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
        let mut result = Self {
            current: ScopedType { ty: root, frame: 0 },
            frames: vec![Frame::default()],
        };
        for step in path {
            charge(limits, work, 1)?;
            result.current = result.head(input, node, result.current, true, limits, work)?;
            let current = result.current;
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
                (ValueProjectionStep::CallResult, TypeInfo::Function(signature))
                    if signature.parameters.is_empty() =>
                {
                    Some(signature.output)
                }
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
            result.current = ScopedType {
                ty: selected,
                frame: current.frame,
            };
        }
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
