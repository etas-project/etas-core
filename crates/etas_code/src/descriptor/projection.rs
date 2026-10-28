use std::collections::BTreeMap;

use super::{
    DescriptorError, DescriptorGraph, DescriptorGraphInput, DescriptorNode, EffectRowInfoRef,
    GenericParameterRef, TraceInfoRef, TypeInfoRef,
    traversal::{node_index, references},
};

mod remap;

impl DescriptorGraph {
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
        for root in roots {
            if node_index(self.data(), *root).is_none() {
                return Err(DescriptorError::MissingReference {
                    from: *root,
                    target: *root,
                });
            }
        }
        let mut mapping = Mapping::default();
        let mut nodes = Vec::new();
        let mut pending = roots.iter().rev().copied().collect::<Vec<_>>();
        while let Some(node) = pending.pop() {
            if !mapping.insert(node) {
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
        Ok((Self::from_validated_projection(output), roots))
    }
}

#[derive(Default)]
struct Mapping {
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

    fn insert(&mut self, node: DescriptorNode) -> bool {
        fn insert<K: Ord + Copy>(map: &mut BTreeMap<K, K>, key: K, value: K) -> bool {
            if let std::collections::btree_map::Entry::Vacant(entry) = map.entry(key) {
                entry.insert(value);
                true
            } else {
                false
            }
        }
        match node {
            DescriptorNode::Type(key) => {
                let index = TypeInfoRef(self.types.len() as u32);
                insert(&mut self.types, key, index)
            }
            DescriptorNode::Row(key) => {
                let index = EffectRowInfoRef(self.rows.len() as u32);
                insert(&mut self.rows, key, index)
            }
            DescriptorNode::Binder(key) => {
                let index = GenericParameterRef(self.binders.len() as u32);
                insert(&mut self.binders, key, index)
            }
            DescriptorNode::Trace(key) => {
                let index = TraceInfoRef(self.traces.len() as u32);
                insert(&mut self.traces, key, index)
            }
        }
    }
}
