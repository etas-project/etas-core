use std::collections::BTreeMap;

use super::{
    DescriptorError, DescriptorGraph, DescriptorGraphInput, DescriptorNode, EffectRowInfoRef,
    GenericParameterRef, TraceInfoRef, TypeInfoRef,
    traversal::{node_index, references},
};

mod remap;

/// Arena offsets for a projected segment that will be appended to another graph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DescriptorGraphOffsets {
    pub types: u32,
    pub rows: u32,
    pub binders: u32,
    pub traces: u32,
}

impl DescriptorGraph {
    /// Validates and merges identical binder identities when independently
    /// projected descriptor closures are combined into one graph.
    pub fn new_with_merged_binders(
        mut input: DescriptorGraphInput,
        limits: super::DescriptorLimits,
    ) -> Result<Self, DescriptorError> {
        let max_nodes = limits.max_nodes.min(u32::MAX as usize);
        let count = input
            .types
            .len()
            .checked_add(input.rows.len())
            .and_then(|n| n.checked_add(input.binders.len()))
            .and_then(|n| n.checked_add(input.traces.len()))
            .ok_or(DescriptorError::Limit {
                resource: "nodes",
                limit: max_nodes,
            })?;
        if count > max_nodes {
            return Err(DescriptorError::Limit {
                resource: "nodes",
                limit: max_nodes,
            });
        }
        let mut identities = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeSet::new();
        let duplicates = input.binders.iter().any(|binder| {
            !identities.insert((&binder.owner, binder.position))
                || !names.insert((&binder.owner, &binder.name))
        });
        if !duplicates {
            return Self::new(input, limits);
        }
        super::validation::validate_for_binder_merge(&input, limits)?;
        let graph = Self::from_validated_projection(input.clone());
        let mut by_identity = BTreeMap::new();
        let mut by_name = BTreeMap::new();
        let mut binders = Vec::new();
        let mut fingerprints = BTreeMap::new();
        let mut mapping = Mapping::default();
        for (index, binder) in input.binders.iter().enumerate() {
            let source = GenericParameterRef(index as u32);
            let key = (binder.owner.clone(), binder.position);
            if by_name
                .insert((binder.owner.clone(), binder.name.clone()), binder.position)
                .is_some_and(|position| position != binder.position)
            {
                return Err(DescriptorError::Invalid {
                    node: DescriptorNode::Binder(source),
                    reason: "generic binder name identifies different positions",
                });
            }
            let target = if let Some((target, original)) = by_identity.get(&key) {
                let original: &super::GenericParameterInfo = original;
                if original.name != binder.name
                    || original.kind != binder.kind
                    || original.bounds.len() != binder.bounds.len()
                {
                    return Err(DescriptorError::Invalid {
                        node: DescriptorNode::Binder(source),
                        reason: "conflicting generic binder identity",
                    });
                }
                for (left, right) in original.bounds.iter().zip(&binder.bounds) {
                    if left.definition != right.definition
                        || left.arguments.len() != right.arguments.len()
                    {
                        return Err(DescriptorError::Invalid {
                            node: DescriptorNode::Binder(source),
                            reason: "conflicting generic binder bounds",
                        });
                    }
                    for (left, right) in left.arguments.iter().zip(&right.arguments) {
                        let mut fingerprint = |id| {
                            if let Some(value) = fingerprints.get(&id) {
                                return Ok(*value);
                            }
                            let value = graph.type_fingerprint(id)?;
                            fingerprints.insert(id, value);
                            Ok::<_, DescriptorError>(value)
                        };
                        if fingerprint(*left)? != fingerprint(*right)? {
                            return Err(DescriptorError::Invalid {
                                node: DescriptorNode::Binder(source),
                                reason: "conflicting generic binder bounds",
                            });
                        }
                    }
                }
                *target
            } else {
                let target = GenericParameterRef(binders.len() as u32);
                binders.push(binder.clone());
                by_identity.insert(key, (target, binder.clone()));
                target
            };
            mapping.binders.insert(source, target);
        }
        let mut parameter_types = BTreeMap::new();
        for (index, ty) in input.types.iter().enumerate() {
            let id = TypeInfoRef(index as u32);
            let target = match ty {
                super::TypeInfo::Parameter(binder) => {
                    let canonical = mapping.binders.get(binder).copied().ok_or(
                        DescriptorError::MissingReference {
                            from: DescriptorNode::Type(id),
                            target: DescriptorNode::Binder(*binder),
                        },
                    )?;
                    *parameter_types.entry(canonical).or_insert(id)
                }
                _ => id,
            };
            mapping.types.insert(id, target);
        }
        for index in 0..input.rows.len() {
            let id = EffectRowInfoRef(index as u32);
            mapping.rows.insert(id, id);
        }
        for index in 0..input.traces.len() {
            let id = TraceInfoRef(index as u32);
            mapping.traces.insert(id, id);
        }
        for (index, ty) in input.types.iter_mut().enumerate() {
            remap::Remapper {
                mapping: &mapping,
                from: DescriptorNode::Type(TypeInfoRef(index as u32)),
            }
            .ty(ty)?;
        }
        for (index, row) in input.rows.iter_mut().enumerate() {
            remap::Remapper {
                mapping: &mapping,
                from: DescriptorNode::Row(EffectRowInfoRef(index as u32)),
            }
            .row(row)?;
        }
        for (index, binder) in binders.iter_mut().enumerate() {
            remap::Remapper {
                mapping: &mapping,
                from: DescriptorNode::Binder(GenericParameterRef(index as u32)),
            }
            .binder(binder)?;
        }
        for (index, trace) in input.traces.iter_mut().enumerate() {
            remap::Remapper {
                mapping: &mapping,
                from: DescriptorNode::Trace(TraceInfoRef(index as u32)),
            }
            .trace(trace)?;
        }
        input.binders = binders;
        Self::new(input, limits)
    }

    /// Copies only the root's reference closure. The root becomes index zero in
    /// its own arena; all reachable references are remapped, including cycles.
    /// Runtime handles can keep sharing the original graph until export.
    pub fn project(&self, root: DescriptorNode) -> Result<Self, DescriptorError> {
        self.project_roots(&[root]).map(|(graph, _)| graph)
    }

    /// Projects a shared closure and returns rebased roots in the input order.
    pub fn project_roots(
        &self,
        roots: &[DescriptorNode],
    ) -> Result<(Self, Vec<DescriptorNode>), DescriptorError> {
        let (output, roots) = self.relocate_roots(roots, DescriptorGraphOffsets::default())?;
        Ok((Self::from_validated_projection(output), roots))
    }

    /// Relocate a reference-closed segment for appending at `offsets`. The
    /// segment's references address the combined graph, not the segment alone;
    /// the combined graph must be validated after insertion.
    pub fn relocate_roots(
        &self,
        roots: &[DescriptorNode],
        offsets: DescriptorGraphOffsets,
    ) -> Result<(DescriptorGraphInput, Vec<DescriptorNode>), DescriptorError> {
        for root in roots {
            if node_index(self.data(), *root).is_none() {
                return Err(DescriptorError::MissingReference {
                    from: *root,
                    target: *root,
                });
            }
        }
        let mut mapping = Mapping {
            offsets,
            ..Mapping::default()
        };
        let mut nodes = Vec::new();
        let mut pending = roots.iter().rev().copied().collect::<Vec<_>>();
        while let Some(node) = pending.pop() {
            if !mapping.insert(node)? {
                continue;
            }
            nodes.push(node);
            let start = pending.len();
            references(self.data(), node, true, |child| pending.push(child));
            pending[start..].reverse();
        }
        let mut output = DescriptorGraphInput::default();
        for node in nodes {
            let remap = remap::Remapper {
                mapping: &mapping,
                from: node,
            };
            match node {
                DescriptorNode::Type(reference) => {
                    let mut ty = self.data().types[reference.0 as usize].clone();
                    remap.ty(&mut ty)?;
                    output.types.push(ty);
                }
                DescriptorNode::Row(reference) => {
                    let mut row = self.data().rows[reference.0 as usize].clone();
                    remap.row(&mut row)?;
                    output.rows.push(row);
                }
                DescriptorNode::Binder(reference) => {
                    let mut binder = self.data().binders[reference.0 as usize].clone();
                    remap.binder(&mut binder)?;
                    output.binders.push(binder);
                }
                DescriptorNode::Trace(reference) => {
                    let mut trace = self.data().traces[reference.0 as usize].clone();
                    remap.trace(&mut trace)?;
                    output.traces.push(trace);
                }
            }
        }
        let roots = roots
            .iter()
            .map(|root| {
                mapping
                    .resolve(*root)
                    .ok_or(DescriptorError::MissingReference {
                        from: *root,
                        target: *root,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        // A reference-closed subgraph preserves the original validation limits.
        Ok((output, roots))
    }
}

#[derive(Default)]
struct Mapping {
    offsets: DescriptorGraphOffsets,
    types: BTreeMap<TypeInfoRef, TypeInfoRef>,
    rows: BTreeMap<EffectRowInfoRef, EffectRowInfoRef>,
    binders: BTreeMap<GenericParameterRef, GenericParameterRef>,
    traces: BTreeMap<TraceInfoRef, TraceInfoRef>,
}

impl Mapping {
    fn resolve(&self, node: DescriptorNode) -> Option<DescriptorNode> {
        match node {
            DescriptorNode::Type(key) => self.types.get(&key).copied().map(DescriptorNode::Type),
            DescriptorNode::Row(key) => self.rows.get(&key).copied().map(DescriptorNode::Row),
            DescriptorNode::Binder(key) => {
                self.binders.get(&key).copied().map(DescriptorNode::Binder)
            }
            DescriptorNode::Trace(key) => self.traces.get(&key).copied().map(DescriptorNode::Trace),
        }
    }

    fn insert(&mut self, node: DescriptorNode) -> Result<bool, DescriptorError> {
        fn insert<K: Ord + Copy>(
            map: &mut BTreeMap<K, K>,
            key: K,
            offset: u32,
            make: impl FnOnce(u32) -> K,
        ) -> Result<bool, DescriptorError> {
            let count = map.len();
            if let std::collections::btree_map::Entry::Vacant(entry) = map.entry(key) {
                let index = u32::try_from(count)
                    .ok()
                    .and_then(|count| offset.checked_add(count))
                    .ok_or(DescriptorError::Limit {
                        resource: "descriptor arena indices",
                        limit: u32::MAX as usize,
                    })?;
                entry.insert(make(index));
                Ok(true)
            } else {
                Ok(false)
            }
        }
        match node {
            DescriptorNode::Type(key) => {
                insert(&mut self.types, key, self.offsets.types, TypeInfoRef)
            }
            DescriptorNode::Row(key) => {
                insert(&mut self.rows, key, self.offsets.rows, EffectRowInfoRef)
            }
            DescriptorNode::Binder(key) => insert(
                &mut self.binders,
                key,
                self.offsets.binders,
                GenericParameterRef,
            ),
            DescriptorNode::Trace(key) => {
                insert(&mut self.traces, key, self.offsets.traces, TraceInfoRef)
            }
        }
    }
}
