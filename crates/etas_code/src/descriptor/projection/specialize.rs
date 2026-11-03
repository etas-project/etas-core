use std::collections::{BTreeMap, BTreeSet, HashSet};

use super::{DescriptorGraph, Mapping, remap::Remapper};
use crate::descriptor::{
    DescriptorError, DescriptorLimits, DescriptorNode, EffectInfo, EffectRowInfoRef,
    EffectTargetInfo, GenericParameterKind, GenericParameterRef, StaticEffectArgInfo, TypeInfo,
    TypeInfoRef,
    traversal::{node_index, references},
};

/// References are scoped to the input graph. This is a descriptive graph
/// transformation, not evidence of type equality or spec satisfaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptorArgument {
    Type(TypeInfoRef),
    EffectRow(EffectRowInfoRef),
}

impl DescriptorGraph {
    /// Specialize every reference to the root signature's binders. Callers
    /// must separately prove the arguments against the checked declaration.
    pub fn specialize_generics(
        &self,
        signature: TypeInfoRef,
        arguments: &[(GenericParameterRef, DescriptorArgument)],
        limits: DescriptorLimits,
    ) -> Result<Self, DescriptorError> {
        self.validate(limits)?;
        let root = DescriptorNode::Type(signature);
        let Some(TypeInfo::Function(function)) = self.type_info(signature) else {
            return Err(DescriptorError::Invalid {
                node: root,
                reason: "generic root is not a function",
            });
        };
        if function.generics.len() != arguments.len() {
            return Err(DescriptorError::Invalid {
                node: root,
                reason: "generic argument count does not match signature",
            });
        }
        let expected = function.generics.iter().copied().collect::<BTreeSet<_>>();
        let mut bindings = BTreeMap::new();
        let mut work = TraversalWork::default();
        for &(binder, argument) in arguments {
            if !expected.contains(&binder) || bindings.insert(binder, argument).is_some() {
                return Err(DescriptorError::Invalid {
                    node: root,
                    reason: "generic argument has a foreign or duplicate binder",
                });
            }
            let parameter = self
                .binder(binder)
                .ok_or(DescriptorError::MissingReference {
                    from: root,
                    target: DescriptorNode::Binder(binder),
                })?;
            let argument_node = match (parameter.kind, argument) {
                (GenericParameterKind::Type, DescriptorArgument::Type(ty)) => {
                    DescriptorNode::Type(ty)
                }
                (GenericParameterKind::EffectRow, DescriptorArgument::EffectRow(row)) => {
                    DescriptorNode::Row(row)
                }
                _ => {
                    return Err(DescriptorError::Invalid {
                        node: root,
                        reason: "generic argument kind does not match binder",
                    });
                }
            };
            if node_index(self.data(), argument_node).is_none() {
                return Err(DescriptorError::MissingReference {
                    from: root,
                    target: argument_node,
                });
            }
            check_closed(self, argument_node, root, limits, &mut work)?;
        }
        let mut mapping = Mapping::default();
        for (index, ty) in self.data().types.iter().enumerate() {
            let id = TypeInfoRef(index as u32);
            let target = match ty {
                TypeInfo::Parameter(binder) => match bindings.get(binder) {
                    Some(DescriptorArgument::Type(ty)) => *ty,
                    _ => id,
                },
                _ => id,
            };
            mapping.types.insert(id, target);
        }
        for index in 0..self.data().rows.len() {
            let id = EffectRowInfoRef(index as u32);
            mapping.rows.insert(id, id);
        }
        for index in 0..self.data().binders.len() {
            let id = GenericParameterRef(index as u32);
            mapping.binders.insert(id, id);
        }
        for index in 0..self.data().traces.len() {
            let id = crate::descriptor::TraceInfoRef(index as u32);
            mapping.traces.insert(id, id);
        }
        let mut added_edges = 0usize;
        let mut added_text = 0usize;
        let mut additions = BTreeMap::new();
        for (index, row) in self.data().rows.iter().enumerate() {
            let Some(DescriptorArgument::EffectRow(argument)) =
                row.tail.and_then(|tail| bindings.get(&tail))
            else {
                continue;
            };
            let argument = self
                .effect_row(*argument)
                .ok_or(DescriptorError::MissingReference {
                    from: root,
                    target: DescriptorNode::Row(*argument),
                })?;
            let mut seen = row.effects.iter().collect::<HashSet<_>>();
            let mut selected = Vec::new();
            for (position, effect) in argument.effects.iter().enumerate() {
                added_edges = added_edges.checked_add(1 + effect.arguments.len()).ok_or(
                    DescriptorError::Limit {
                        resource: "edges",
                        limit: limits.max_edges,
                    },
                )?;
                added_text = added_text.checked_add(effect_text_bytes(effect)?).ok_or(
                    DescriptorError::Limit {
                        resource: "text bytes",
                        limit: limits.max_text_bytes,
                    },
                )?;
                if added_edges > limits.max_edges {
                    return Err(DescriptorError::Limit {
                        resource: "edges",
                        limit: limits.max_edges,
                    });
                }
                if added_text > limits.max_text_bytes {
                    return Err(DescriptorError::Limit {
                        resource: "text bytes",
                        limit: limits.max_text_bytes,
                    });
                }
                if seen.insert(effect) {
                    selected.push(position);
                }
            }
            additions.insert(index, selected);
        }
        let mut output = self.data().clone();
        for (index, ty) in output.types.iter_mut().enumerate() {
            if let TypeInfo::Function(function) = ty {
                function
                    .generics
                    .retain(|binder| !bindings.contains_key(binder));
            }
            Remapper {
                mapping: &mapping,
                from: DescriptorNode::Type(TypeInfoRef(index as u32)),
            }
            .ty(ty)?;
        }
        for (index, row) in output.rows.iter_mut().enumerate() {
            if let Some(DescriptorArgument::EffectRow(argument)) =
                row.tail.and_then(|tail| bindings.get(&tail))
            {
                let source = &self.data().rows[argument.0 as usize];
                for &position in additions.get(&index).into_iter().flatten() {
                    row.effects.push(source.effects[position].clone());
                }
                row.tail = None;
            }
            Remapper {
                mapping: &mapping,
                from: DescriptorNode::Row(EffectRowInfoRef(index as u32)),
            }
            .row(row)?;
            let mut unique = HashSet::new();
            row.effects.retain(|effect| unique.insert(effect.clone()));
        }
        for (index, binder) in output.binders.iter_mut().enumerate() {
            Remapper {
                mapping: &mapping,
                from: DescriptorNode::Binder(GenericParameterRef(index as u32)),
            }
            .binder(binder)?;
        }
        for (index, trace) in output.traces.iter_mut().enumerate() {
            Remapper {
                mapping: &mapping,
                from: DescriptorNode::Trace(crate::descriptor::TraceInfoRef(index as u32)),
            }
            .trace(trace)?;
        }
        let mut outcome_work = 0usize;
        for trace in &mut output.traces {
            let crate::descriptor::TraceInfo::Outcome(
                crate::descriptor::CallableOutcomeInfo::ParameterNonReturnTemplate(call),
            ) = trace
            else {
                continue;
            };
            if call.signature != signature {
                continue;
            }
            if !callback_has_closed_action_row(
                &output.types,
                &output.rows,
                call,
                limits,
                &mut outcome_work,
            )? {
                return Err(DescriptorError::Invalid {
                    node: DescriptorNode::Type(signature),
                    reason: "instantiated callback outcome requires a closed action-only row",
                });
            }
            *trace = crate::descriptor::TraceInfo::Outcome(
                crate::descriptor::CallableOutcomeInfo::ParameterNonReturn(call.clone()),
            );
        }
        DescriptorGraph::new(output, limits)
    }
}

fn callback_has_closed_action_row(
    types: &[TypeInfo],
    rows: &[crate::descriptor::EffectRowInfo],
    call: &crate::descriptor::ParameterCallInfo,
    limits: DescriptorLimits,
    work: &mut usize,
) -> Result<bool, DescriptorError> {
    let Some(TypeInfo::Function(owner)) = types.get(call.signature.0 as usize) else {
        return Ok(false);
    };
    let Some(parameter) = owner.parameters.get(call.position as usize) else {
        return Ok(false);
    };
    let mut ty = parameter.ty;
    let mut seen = BTreeSet::new();
    loop {
        *work = work.checked_add(1).ok_or(DescriptorError::Limit {
            resource: "edges",
            limit: limits.max_edges,
        })?;
        if *work > limits.max_edges {
            return Err(DescriptorError::Limit {
                resource: "edges",
                limit: limits.max_edges,
            });
        }
        if !seen.insert(ty) {
            return Ok(false);
        }
        match types.get(ty.0 as usize) {
            Some(TypeInfo::Function(callback)) => {
                let Some(row) = callback.effects.and_then(|id| rows.get(id.0 as usize)) else {
                    return Ok(false);
                };
                *work = work
                    .checked_add(row.effects.len())
                    .ok_or(DescriptorError::Limit {
                        resource: "edges",
                        limit: limits.max_edges,
                    })?;
                if *work > limits.max_edges {
                    return Err(DescriptorError::Limit {
                        resource: "edges",
                        limit: limits.max_edges,
                    });
                }
                return Ok(callback.generics.is_empty()
                    && row.tail.is_none()
                    && row
                        .effects
                        .iter()
                        .all(|effect| matches!(effect.target, EffectTargetInfo::Action { .. })));
            }
            Some(TypeInfo::Alias { target, .. }) => ty = *target,
            Some(TypeInfo::Builtin {
                constructor: crate::descriptor::BuiltinTypeInfo::CallableDefinition,
                arguments,
            }) if arguments.len() == 1 => ty = arguments[0],
            _ => return Ok(false),
        }
    }
}

#[derive(Default)]
struct TraversalWork {
    nodes: usize,
    edges: usize,
}

fn check_closed(
    graph: &DescriptorGraph,
    argument: DescriptorNode,
    root: DescriptorNode,
    limits: DescriptorLimits,
    work: &mut TraversalWork,
) -> Result<(), DescriptorError> {
    let mut pending = vec![argument];
    let mut visited = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if !visited.insert(node) {
            continue;
        }
        work.nodes = work.nodes.checked_add(1).ok_or(DescriptorError::Limit {
            resource: "nodes",
            limit: limits.max_nodes,
        })?;
        if work.nodes > limits.max_nodes {
            return Err(DescriptorError::Limit {
                resource: "nodes",
                limit: limits.max_nodes,
            });
        }
        match node {
            DescriptorNode::Type(ty)
                if matches!(graph.type_info(ty), Some(TypeInfo::Parameter(_))) =>
            {
                return Err(DescriptorError::Invalid {
                    node: root,
                    reason: "generic argument contains an unresolved type parameter",
                });
            }
            DescriptorNode::Type(ty) if matches!(graph.type_info(ty), Some(TypeInfo::Function(f)) if !f.generics.is_empty()) =>
            {
                return Err(DescriptorError::Invalid {
                    node: root,
                    reason: "generic argument contains a polymorphic function",
                });
            }
            DescriptorNode::Row(row)
                if graph.effect_row(row).is_some_and(|row| row.tail.is_some()) =>
            {
                return Err(DescriptorError::Invalid {
                    node: root,
                    reason: "generic argument contains an open effect row",
                });
            }
            DescriptorNode::Binder(_) => continue,
            _ => {}
        }
        references(graph.data(), node, false, |child| {
            work.edges = work.edges.saturating_add(1);
            if work.edges <= limits.max_edges {
                pending.push(child);
            }
        });
        if work.edges > limits.max_edges {
            return Err(DescriptorError::Limit {
                resource: "edges",
                limit: limits.max_edges,
            });
        }
    }
    Ok(())
}

fn effect_text_bytes(effect: &EffectInfo) -> Result<usize, DescriptorError> {
    fn identity(key: &crate::DefinitionKey) -> Option<usize> {
        key.module()
            .iter()
            .chain(key.declaration())
            .try_fold(0usize, |sum, part| sum.checked_add(part.len()))
    }
    fn callable(id: &crate::CallableId) -> Option<usize> {
        if let Some(key) = id.as_declaration() {
            return identity(key);
        }
        let method = id.as_method()?;
        let implementation = method.implementation();
        let mut size = identity(implementation.subject())?.checked_add(method.name().len())?;
        if let Some(spec) = implementation.selected_spec() {
            size = size.checked_add(identity(spec)?)?;
        }
        implementation
            .module()
            .iter()
            .try_fold(size, |sum, part| sum.checked_add(part.len()))
    }
    let mut size = match &effect.target {
        EffectTargetInfo::Effect(key) => identity(key),
        EffectTargetInfo::Action { effect, name } => {
            identity(effect).and_then(|size| size.checked_add(name.len()))
        }
    }
    .ok_or(DescriptorError::InvalidDescription {
        reason: "effect text size overflow",
    })?;
    for arg in &effect.arguments {
        let added = match arg {
            StaticEffectArgInfo::Resource(key) => identity(key),
            StaticEffectArgInfo::Callable(id) => callable(id),
            StaticEffectArgInfo::String(value) | StaticEffectArgInfo::Integer(value) => {
                Some(value.len())
            }
            StaticEffectArgInfo::Type(_) | StaticEffectArgInfo::Wildcard => Some(0),
        }
        .ok_or(DescriptorError::InvalidDescription {
            reason: "effect text size overflow",
        })?;
        size = size
            .checked_add(added)
            .ok_or(DescriptorError::InvalidDescription {
                reason: "effect text size overflow",
            })?;
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{
        DescriptorGraphInput, EffectRowInfo, FieldInfo, GenericOwner, GenericParameterInfo,
        ParameterInfo, PrimitiveInfo, SignatureInfo, TraceInfo, TraceInfoRef,
    };
    use crate::{CodeFingerprint, DefinitionKey, DefinitionKind, PackageKey};

    fn key(name: &str, kind: DefinitionKind) -> DefinitionKey {
        DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(CodeFingerprint::of_bytes(
                "test.specialize",
                b"pkg",
            )),
            vec!["app".into()],
            vec![name.into()],
            kind,
        )
        .unwrap()
    }

    fn fixture() -> DescriptorGraph {
        let owner = GenericOwner::Declaration(key("identity", DefinitionKind::Flow));
        let effect = EffectInfo {
            target: EffectTargetInfo::Action {
                effect: key("Console", DefinitionKind::Effect),
                name: "write".into(),
            },
            arguments: vec![],
        };
        DescriptorGraph::new(
            DescriptorGraphInput {
                binders: vec![
                    GenericParameterInfo {
                        owner: owner.clone(),
                        position: 0,
                        name: "T".into(),
                        kind: GenericParameterKind::Type,
                        bounds: vec![],
                    },
                    GenericParameterInfo {
                        owner,
                        position: 1,
                        name: "E".into(),
                        kind: GenericParameterKind::EffectRow,
                        bounds: vec![],
                    },
                ],
                types: vec![
                    TypeInfo::Parameter(GenericParameterRef(0)),
                    TypeInfo::Primitive(PrimitiveInfo::Bool),
                    TypeInfo::Function(SignatureInfo {
                        generics: vec![GenericParameterRef(0), GenericParameterRef(1)],
                        parameters: vec![ParameterInfo {
                            name: Some("value".into()),
                            ty: TypeInfoRef(0),
                        }],
                        output: TypeInfoRef(0),
                        declared_output: Some(TypeInfoRef(0)),
                        effects: Some(EffectRowInfoRef(0)),
                    }),
                    TypeInfo::Nominal {
                        definition: key("Node", DefinitionKind::Type),
                        parameters: vec![],
                        representation: Some(TypeInfoRef(4)),
                    },
                    TypeInfo::Record(vec![FieldInfo {
                        name: "next".into(),
                        ty: TypeInfoRef(3),
                    }]),
                ],
                rows: vec![
                    EffectRowInfo {
                        effects: vec![],
                        tail: Some(GenericParameterRef(1)),
                    },
                    EffectRowInfo {
                        effects: vec![effect.clone()],
                        tail: None,
                    },
                ],
                traces: vec![TraceInfo::Event(EffectInfo {
                    arguments: vec![StaticEffectArgInfo::Type(TypeInfoRef(0))],
                    ..effect
                })],
            },
            DescriptorLimits::default(),
        )
        .unwrap()
    }

    fn generic_outcome_fixture() -> DescriptorGraph {
        let identity = key("invoke", DefinitionKind::Flow);
        let callback = SignatureInfo {
            generics: vec![],
            parameters: vec![],
            output: TypeInfoRef(0),
            declared_output: None,
            effects: Some(EffectRowInfoRef(0)),
        };
        let owner = SignatureInfo {
            generics: vec![GenericParameterRef(0)],
            parameters: vec![ParameterInfo {
                name: Some("callback".into()),
                ty: TypeInfoRef(1),
            }],
            output: TypeInfoRef(0),
            declared_output: None,
            effects: Some(EffectRowInfoRef(0)),
        };
        let call = crate::descriptor::ParameterCallInfo {
            callable: crate::CallableId::declaration(identity.clone()).unwrap(),
            signature: TypeInfoRef(2),
            position: 0,
            arguments: Some(vec![]),
            handlers: Some(vec![]),
        };
        DescriptorGraph::new(
            DescriptorGraphInput {
                binders: vec![GenericParameterInfo {
                    owner: GenericOwner::Declaration(identity),
                    position: 0,
                    name: "E".into(),
                    kind: GenericParameterKind::EffectRow,
                    bounds: vec![],
                }],
                types: vec![
                    TypeInfo::Primitive(PrimitiveInfo::Unit),
                    TypeInfo::Function(callback),
                    TypeInfo::Function(owner),
                ],
                rows: vec![
                    EffectRowInfo {
                        effects: vec![],
                        tail: Some(GenericParameterRef(0)),
                    },
                    EffectRowInfo {
                        effects: vec![EffectInfo {
                            target: EffectTargetInfo::Action {
                                effect: key("Console", DefinitionKind::Effect),
                                name: "write".into(),
                            },
                            arguments: vec![],
                        }],
                        tail: None,
                    },
                    EffectRowInfo {
                        effects: vec![EffectInfo {
                            target: EffectTargetInfo::Effect(key("Error", DefinitionKind::Effect)),
                            arguments: vec![],
                        }],
                        tail: None,
                    },
                ],
                traces: vec![TraceInfo::Outcome(
                    crate::descriptor::CallableOutcomeInfo::ParameterNonReturnTemplate(call),
                )],
            },
            DescriptorLimits::default(),
        )
        .unwrap()
    }

    #[test]
    fn generic_outcome_requires_instantiation_before_trace_proof() {
        let graph = generic_outcome_fixture();
        assert!(matches!(
            crate::contract::check_trace_inclusion(
                &graph,
                TraceInfoRef(0),
                &graph,
                TraceInfoRef(0),
                crate::contract::TraceProofLimits::default(),
            ),
            Err(crate::contract::TraceProofError::UninstantiatedGenericOutcome)
        ));
        let applied = graph
            .specialize_generics(
                TypeInfoRef(2),
                &[(
                    GenericParameterRef(0),
                    DescriptorArgument::EffectRow(EffectRowInfoRef(1)),
                )],
                DescriptorLimits::default(),
            )
            .unwrap();
        assert!(matches!(
            applied.trace(TraceInfoRef(0)),
            Some(TraceInfo::Outcome(
                crate::descriptor::CallableOutcomeInfo::ParameterNonReturn(_)
            ))
        ));
        assert!(matches!(
            crate::contract::check_trace_inclusion(
                &applied,
                TraceInfoRef(0),
                &applied,
                TraceInfoRef(0),
                crate::contract::TraceProofLimits::default(),
            ),
            Ok(crate::contract::TraceInclusion::Included)
        ));
        assert!(
            graph
                .specialize_generics(
                    TypeInfoRef(2),
                    &[(
                        GenericParameterRef(0),
                        DescriptorArgument::EffectRow(EffectRowInfoRef(2))
                    ),],
                    DescriptorLimits::default()
                )
                .is_err()
        );
    }

    #[test]
    fn generic_outcome_template_requires_an_owner_bound_open_row() {
        let mut input = generic_outcome_fixture().data().clone();
        input.rows[0].tail = None;
        assert!(DescriptorGraph::new(input, DescriptorLimits::default()).is_err());
        let mut input = generic_outcome_fixture().data().clone();
        input.binders.push(GenericParameterInfo {
            owner: GenericOwner::Declaration(key("foreign", DefinitionKind::Flow)),
            position: 0,
            name: "E".into(),
            kind: GenericParameterKind::EffectRow,
            bounds: vec![],
        });
        input.rows[0].tail = Some(GenericParameterRef(1));
        assert!(DescriptorGraph::new(input, DescriptorLimits::default()).is_err());
    }

    #[test]
    fn specializes_type_effect_and_trace_without_changing_input() {
        let original = fixture();
        let result = original
            .specialize_generics(
                TypeInfoRef(2),
                &[
                    (
                        GenericParameterRef(0),
                        DescriptorArgument::Type(TypeInfoRef(1)),
                    ),
                    (
                        GenericParameterRef(1),
                        DescriptorArgument::EffectRow(EffectRowInfoRef(1)),
                    ),
                ],
                DescriptorLimits::default(),
            )
            .unwrap();
        let Some(TypeInfo::Function(signature)) = result.type_info(TypeInfoRef(2)) else {
            panic!("function");
        };
        assert!(signature.generics.is_empty());
        assert_eq!(signature.parameters[0].ty, TypeInfoRef(1));
        assert_eq!(signature.output, TypeInfoRef(1));
        assert_eq!(signature.declared_output, Some(TypeInfoRef(1)));
        assert_eq!(
            result.effect_row(EffectRowInfoRef(0)),
            result.effect_row(EffectRowInfoRef(1))
        );
        assert!(
            matches!(result.trace(TraceInfoRef(0)), Some(TraceInfo::Event(event)) if event.arguments == [StaticEffectArgInfo::Type(TypeInfoRef(1))])
        );
        assert!(
            matches!(original.type_info(TypeInfoRef(2)), Some(TypeInfo::Function(signature)) if signature.generics.len() == 2)
        );
    }

    #[test]
    fn accepts_closed_recursive_nominal_arguments_and_preserves_their_identity() {
        let result = fixture()
            .specialize_generics(
                TypeInfoRef(2),
                &[
                    (
                        GenericParameterRef(0),
                        DescriptorArgument::Type(TypeInfoRef(3)),
                    ),
                    (
                        GenericParameterRef(1),
                        DescriptorArgument::EffectRow(EffectRowInfoRef(1)),
                    ),
                ],
                DescriptorLimits::default(),
            )
            .unwrap();
        let Some(TypeInfo::Function(signature)) = result.type_info(TypeInfoRef(2)) else {
            panic!("function");
        };
        assert_eq!(signature.output, TypeInfoRef(3));
        assert!(
            matches!(result.type_info(TypeInfoRef(3)), Some(TypeInfo::Nominal { definition, .. }) if definition == &key("Node", DefinitionKind::Type))
        );
    }

    #[test]
    fn row_union_deduplicates_after_type_substitution() {
        let mut input = fixture().data().clone();
        let mut generic = input.rows[1].effects[0].clone();
        generic
            .arguments
            .push(StaticEffectArgInfo::Type(TypeInfoRef(0)));
        let mut concrete = generic.clone();
        concrete.arguments = vec![StaticEffectArgInfo::Type(TypeInfoRef(1))];
        input.rows[0].effects.push(generic);
        input.rows[1].effects = vec![concrete];
        let graph = DescriptorGraph::new(input, DescriptorLimits::default()).unwrap();
        let result = graph
            .specialize_generics(
                TypeInfoRef(2),
                &[
                    (
                        GenericParameterRef(0),
                        DescriptorArgument::Type(TypeInfoRef(1)),
                    ),
                    (
                        GenericParameterRef(1),
                        DescriptorArgument::EffectRow(EffectRowInfoRef(1)),
                    ),
                ],
                DescriptorLimits::default(),
            )
            .unwrap();
        let row = result.effect_row(EffectRowInfoRef(0)).unwrap();
        assert_eq!(row.effects.len(), 1);
        assert_eq!(
            row.effects,
            result.effect_row(EffectRowInfoRef(1)).unwrap().effects
        );
    }

    #[test]
    fn preserves_generic_binders_owned_by_other_declarations() {
        let mut input = fixture().data().clone();
        input.binders.push(GenericParameterInfo {
            owner: GenericOwner::Declaration(key("other", DefinitionKind::Flow)),
            position: 0,
            name: "T".into(),
            kind: GenericParameterKind::Type,
            bounds: vec![],
        });
        input
            .types
            .push(TypeInfo::Parameter(GenericParameterRef(2)));
        input.types.push(TypeInfo::Function(SignatureInfo {
            generics: vec![GenericParameterRef(2)],
            parameters: vec![ParameterInfo {
                name: None,
                ty: TypeInfoRef(5),
            }],
            output: TypeInfoRef(5),
            declared_output: None,
            effects: None,
        }));
        let graph = DescriptorGraph::new(input, DescriptorLimits::default()).unwrap();
        let result = graph
            .specialize_generics(
                TypeInfoRef(2),
                &[
                    (
                        GenericParameterRef(0),
                        DescriptorArgument::Type(TypeInfoRef(1)),
                    ),
                    (
                        GenericParameterRef(1),
                        DescriptorArgument::EffectRow(EffectRowInfoRef(1)),
                    ),
                ],
                DescriptorLimits::default(),
            )
            .unwrap();
        assert_eq!(
            graph.type_info(TypeInfoRef(6)),
            result.type_info(TypeInfoRef(6))
        );
    }

    #[test]
    fn rejects_row_expansion_before_allocating_beyond_the_work_budget() {
        let mut input = fixture().data().clone();
        for _ in 0..16 {
            input.rows.push(EffectRowInfo {
                effects: vec![],
                tail: Some(GenericParameterRef(1)),
            });
        }
        let base = input.rows[1].effects[0].clone();
        input.rows[1].effects = (0..8)
            .map(|index| {
                let mut effect = base.clone();
                let EffectTargetInfo::Action { name, .. } = &mut effect.target else {
                    panic!("action");
                };
                *name = format!("write{index}");
                effect
            })
            .collect();
        let graph = DescriptorGraph::new(input, DescriptorLimits::default()).unwrap();
        let limits = DescriptorLimits {
            max_edges: 64,
            ..Default::default()
        };
        assert!(matches!(
            graph.specialize_generics(
                TypeInfoRef(2),
                &[
                    (
                        GenericParameterRef(0),
                        DescriptorArgument::Type(TypeInfoRef(1))
                    ),
                    (
                        GenericParameterRef(1),
                        DescriptorArgument::EffectRow(EffectRowInfoRef(1))
                    ),
                ],
                limits
            ),
            Err(DescriptorError::Limit {
                resource: "edges",
                limit: 64
            })
        ));
    }

    #[test]
    fn rejects_missing_duplicate_wrong_kind_and_unclosed_arguments() {
        let graph = fixture();
        let t = GenericParameterRef(0);
        let e = GenericParameterRef(1);
        for bindings in [
            vec![(t, DescriptorArgument::Type(TypeInfoRef(1)))],
            vec![
                (t, DescriptorArgument::Type(TypeInfoRef(1))),
                (t, DescriptorArgument::Type(TypeInfoRef(1))),
            ],
            vec![
                (t, DescriptorArgument::EffectRow(EffectRowInfoRef(1))),
                (e, DescriptorArgument::EffectRow(EffectRowInfoRef(1))),
            ],
            vec![
                (t, DescriptorArgument::Type(TypeInfoRef(0))),
                (e, DescriptorArgument::EffectRow(EffectRowInfoRef(1))),
            ],
            vec![
                (t, DescriptorArgument::Type(TypeInfoRef(1))),
                (e, DescriptorArgument::EffectRow(EffectRowInfoRef(0))),
            ],
        ] {
            assert!(
                graph
                    .specialize_generics(TypeInfoRef(2), &bindings, DescriptorLimits::default())
                    .is_err()
            );
        }
    }
}
