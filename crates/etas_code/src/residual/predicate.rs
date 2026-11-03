//! Closed, typed predicate graphs. Validation is not installation evidence.

use std::collections::{BTreeMap, BTreeSet};

mod evaluate;
pub use evaluate::{
    BorrowedValue, PredicateDecision, PredicateEvaluationError, PredicateEvaluationLimits,
};

use crate::{CodeFingerprint, identity::canonical::CanonicalEncoder};

use super::{
    binding::BindingLayout,
    identity::{EvidenceTestId, GuardProviderId, PredicateId, ValueBindingId, ValuePatternId},
};

/// Exact checked type identities prevent comparisons across nominal domains.
#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum ValueType {
    Boolean,
    Integer(CodeFingerprint),
    Text,
    Bytes,
    Identity(CodeFingerprint),
}

impl ValueType {
    pub(super) fn encode(self, encoder: &mut CanonicalEncoder) {
        match self {
            Self::Boolean => encoder.tag(1),
            Self::Integer(id) => {
                encoder.tag(2);
                encoder.fingerprint(id);
            }
            Self::Text => encoder.tag(3),
            Self::Bytes => encoder.tag(4),
            Self::Identity(id) => {
                encoder.tag(5);
                encoder.fingerprint(id);
            }
        }
    }
}

/// A registered provider's semantic version and exact operand contract.
#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct EvidenceTestSchema {
    pub provider: GuardProviderId,
    pub semantic_version: u32,
    pub arguments: Vec<ValueType>,
}

impl EvidenceTestId {
    pub fn from_schema(schema: &EvidenceTestSchema) -> Self {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.evidence-test.v1");
        encoder.fingerprint(schema.provider.fingerprint());
        encoder.number(u64::from(schema.semantic_version));
        encoder.number(schema.arguments.len() as u64);
        for argument in &schema.arguments {
            argument.encode(&mut encoder);
        }
        Self::from_canonical_fingerprint(encoder.finish())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum PredicateNode {
    Equal {
        left: ValueBindingId,
        right: ValueBindingId,
    },
    PatternMember {
        value: ValueBindingId,
        pattern: ValuePatternId,
    },
    All(Vec<PredicateId>),
    Any(Vec<PredicateId>),
    Not(PredicateId),
    Evidence {
        test: EvidenceTestId,
        arguments: Vec<ValueBindingId>,
    },
}

impl PredicateId {
    /// Content identity includes the node kind and ordered arguments.
    pub fn from_node(node: &PredicateNode) -> Self {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.predicate.v1");
        match node {
            PredicateNode::Equal { left, right } => {
                encoder.tag(1);
                encoder.fingerprint(left.fingerprint());
                encoder.fingerprint(right.fingerprint());
            }
            PredicateNode::PatternMember { value, pattern } => {
                encoder.tag(2);
                encoder.fingerprint(value.fingerprint());
                encoder.fingerprint(pattern.fingerprint());
            }
            PredicateNode::All(children) | PredicateNode::Any(children) => {
                encoder.tag(if matches!(node, PredicateNode::All(_)) {
                    3
                } else {
                    4
                });
                encoder.number(children.len() as u64);
                for child in children {
                    encoder.fingerprint(child.fingerprint());
                }
            }
            PredicateNode::Not(child) => {
                encoder.tag(5);
                encoder.fingerprint(child.fingerprint());
            }
            PredicateNode::Evidence { test, arguments } => {
                encoder.tag(6);
                encoder.fingerprint(test.fingerprint());
                encoder.number(arguments.len() as u64);
                for argument in arguments {
                    encoder.fingerprint(argument.fingerprint());
                }
            }
        }
        Self::from_canonical_fingerprint(encoder.finish())
    }
}

/// Input tables must be supplied from checked bindings and registered tests.
#[derive(Clone, Debug, Default, etas_core::RetainedSize)]
pub struct PredicateSchema {
    pub bindings: BTreeMap<ValueBindingId, ValueType>,
    pub patterns: BTreeMap<ValuePatternId, ValueType>,
    pub evidence_tests: BTreeMap<EvidenceTestId, EvidenceTestSchema>,
    pub registered_providers: BTreeSet<(GuardProviderId, u32)>,
}

#[derive(Clone, Copy, Debug)]
pub struct PredicateLimits {
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_depth: usize,
    pub max_evidence_arguments: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PredicateError {
    NodeLimit,
    EdgeLimit,
    DepthLimit,
    EvidenceArgumentLimit,
    MissingNode(PredicateId),
    MissingBinding(ValueBindingId),
    MissingPattern(ValuePatternId),
    UnknownEvidenceTest(EvidenceTestId),
    UnknownEvidenceProvider(GuardProviderId, u32),
    EvidenceTestIdentityMismatch(EvidenceTestId),
    TypeMismatch,
    EmptyLogicalOperator,
    Cycle(PredicateId),
    NodeIdentityMismatch(PredicateId),
    UnreachableNode(PredicateId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BorrowedPredicateSupportError {
    NodeLimit,
    BindingLimit,
    WorkLimit,
    UnsupportedPattern(PredicateId),
    UnsupportedEvidence(PredicateId),
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct PredicateGraph {
    pub root: PredicateId,
    pub nodes: BTreeMap<PredicateId, PredicateNode>,
}

/// Immutable schema binding for later plan verification. This is descriptive
/// checked data, not evidence that any runtime guard has been installed.
#[derive(Clone, Debug, etas_core::RetainedSize)]
pub struct BoundPredicateGraph {
    graph: PredicateGraph,
    schema: PredicateSchema,
}

impl BoundPredicateGraph {
    pub fn validate_with_limits(&self, limits: PredicateLimits) -> Result<(), PredicateError> {
        self.graph.validate(&self.schema, limits)
    }

    /// Checks whether the native borrowed evaluator implements every node and
    /// whether its worst-case graph traversal fits the target's work budget.
    pub fn validate_borrowed_support(
        &self,
        limits: PredicateEvaluationLimits,
    ) -> Result<(), BorrowedPredicateSupportError> {
        if self.graph.nodes.len() > limits.max_nodes {
            return Err(BorrowedPredicateSupportError::NodeLimit);
        }
        if self.binding_count() > limits.max_bindings {
            return Err(BorrowedPredicateSupportError::BindingLimit);
        }
        let mut work = 1usize;
        for (id, node) in &self.graph.nodes {
            let children = match node {
                PredicateNode::All(children) | PredicateNode::Any(children) => children.len(),
                PredicateNode::Not(_) => 1,
                PredicateNode::Equal { .. } => 0,
                PredicateNode::PatternMember { .. } => {
                    return Err(BorrowedPredicateSupportError::UnsupportedPattern(*id));
                }
                PredicateNode::Evidence { .. } => {
                    return Err(BorrowedPredicateSupportError::UnsupportedEvidence(*id));
                }
            };
            if children != 0 {
                work = work
                    .checked_add(
                        children
                            .checked_add(1)
                            .and_then(|n| n.checked_mul(2))
                            .ok_or(BorrowedPredicateSupportError::WorkLimit)?,
                    )
                    .ok_or(BorrowedPredicateSupportError::WorkLimit)?;
            }
        }
        if work > limits.max_work {
            return Err(BorrowedPredicateSupportError::WorkLimit);
        }
        Ok(())
    }
    pub fn graph(&self) -> &PredicateGraph {
        &self.graph
    }

    pub fn binding_type(&self, id: ValueBindingId) -> Option<ValueType> {
        self.schema.bindings.get(&id).copied()
    }

    pub fn binding_count(&self) -> usize {
        self.schema.bindings.len()
    }

    pub fn pattern_type(&self, id: ValuePatternId) -> Option<ValueType> {
        self.schema.patterns.get(&id).copied()
    }

    pub fn evidence_test(&self, id: EvidenceTestId) -> Option<&EvidenceTestSchema> {
        self.schema.evidence_tests.get(&id)
    }

    /// Checks the exact operand types used by this graph against a layout.
    pub fn validate_layout(&self, layout: &BindingLayout) -> Result<(), PredicateError> {
        for (id, expected) in &self.schema.bindings {
            let actual = layout
                .value_type(*id)
                .ok_or(PredicateError::MissingBinding(*id))?;
            if actual != *expected {
                return Err(PredicateError::TypeMismatch);
            }
        }
        Ok(())
    }
}

impl PredicateGraph {
    /// Freezes only the checked schema entries referenced by this graph.
    pub fn bind(
        self,
        schema: &PredicateSchema,
        limits: PredicateLimits,
    ) -> Result<BoundPredicateGraph, PredicateError> {
        self.validate(schema, limits)?;
        let mut bound = PredicateSchema::default();
        let mut add_binding = |id: ValueBindingId| -> Result<(), PredicateError> {
            let ty = schema
                .bindings
                .get(&id)
                .ok_or(PredicateError::MissingBinding(id))?;
            bound.bindings.insert(id, *ty);
            Ok(())
        };
        for node in self.nodes.values() {
            match node {
                PredicateNode::Equal { left, right } => {
                    add_binding(*left)?;
                    add_binding(*right)?;
                }
                PredicateNode::PatternMember { value, .. } => add_binding(*value)?,
                PredicateNode::Evidence { arguments, .. } => {
                    for argument in arguments {
                        add_binding(*argument)?;
                    }
                }
                PredicateNode::All(_) | PredicateNode::Any(_) | PredicateNode::Not(_) => {}
            }
        }
        for node in self.nodes.values() {
            match node {
                PredicateNode::PatternMember { pattern, .. } => {
                    let ty = schema
                        .patterns
                        .get(pattern)
                        .ok_or(PredicateError::MissingPattern(*pattern))?;
                    bound.patterns.insert(*pattern, *ty);
                }
                PredicateNode::Evidence { test, .. } => {
                    let test_schema = schema
                        .evidence_tests
                        .get(test)
                        .ok_or(PredicateError::UnknownEvidenceTest(*test))?;
                    bound
                        .registered_providers
                        .insert((test_schema.provider, test_schema.semantic_version));
                    bound.evidence_tests.insert(*test, test_schema.clone());
                }
                _ => {}
            }
        }
        Ok(BoundPredicateGraph {
            graph: self,
            schema: bound,
        })
    }

    /// Validates structural and checked-type references only. Graphs must still
    /// be bound into a verified plan and supported runtime before execution.
    pub fn validate(
        &self,
        schema: &PredicateSchema,
        limits: PredicateLimits,
    ) -> Result<(), PredicateError> {
        if self.nodes.len() > limits.max_nodes {
            return Err(PredicateError::NodeLimit);
        }
        let binding = |id: &ValueBindingId| {
            schema
                .bindings
                .get(id)
                .ok_or(PredicateError::MissingBinding(*id))
        };
        let mut edges = 0usize;
        let mut visited = BTreeSet::new();
        let mut active = BTreeSet::new();
        let mut heights = BTreeMap::<PredicateId, usize>::new();
        let mut stack = vec![(self.root, false, 1usize)];
        while let Some((id, leaving, depth)) = stack.pop() {
            if leaving {
                let node = self.nodes.get(&id).ok_or(PredicateError::MissingNode(id))?;
                let children: &[PredicateId] = match node {
                    PredicateNode::All(children) | PredicateNode::Any(children) => children,
                    PredicateNode::Not(child) => std::slice::from_ref(child),
                    _ => &[],
                };
                let mut height = 1usize;
                for child in children {
                    let child_height = heights
                        .get(child)
                        .ok_or(PredicateError::MissingNode(*child))?;
                    height = height.max(child_height.saturating_add(1));
                }
                if height > limits.max_depth {
                    return Err(PredicateError::DepthLimit);
                }
                heights.insert(id, height);
                active.remove(&id);
                visited.insert(id);
                continue;
            }
            if depth > limits.max_depth {
                return Err(PredicateError::DepthLimit);
            }
            if active.contains(&id) {
                return Err(PredicateError::Cycle(id));
            }
            if visited.contains(&id) {
                continue;
            }
            let node = self.nodes.get(&id).ok_or(PredicateError::MissingNode(id))?;
            let references = match node {
                PredicateNode::Equal { .. } | PredicateNode::PatternMember { .. } => 2,
                PredicateNode::All(children) | PredicateNode::Any(children) => children.len(),
                PredicateNode::Not(_) => 1,
                PredicateNode::Evidence { arguments, .. } => {
                    if arguments.len() > limits.max_evidence_arguments {
                        return Err(PredicateError::EvidenceArgumentLimit);
                    }
                    // Count the test and provider as well as its operand bindings.
                    arguments
                        .len()
                        .checked_add(2)
                        .ok_or(PredicateError::EdgeLimit)?
                }
            };
            edges = edges
                .checked_add(references)
                .ok_or(PredicateError::EdgeLimit)?;
            if edges > limits.max_edges {
                return Err(PredicateError::EdgeLimit);
            }
            match node {
                PredicateNode::Equal { left, right } if binding(left)? != binding(right)? => {
                    return Err(PredicateError::TypeMismatch);
                }
                PredicateNode::PatternMember { value, pattern } => {
                    let expected = schema
                        .patterns
                        .get(pattern)
                        .ok_or(PredicateError::MissingPattern(*pattern))?;
                    if binding(value)? != expected {
                        return Err(PredicateError::TypeMismatch);
                    }
                }
                PredicateNode::Evidence { test, arguments } => {
                    let expected = schema
                        .evidence_tests
                        .get(test)
                        .ok_or(PredicateError::UnknownEvidenceTest(*test))?;
                    if expected.arguments.len() != arguments.len() {
                        return Err(PredicateError::TypeMismatch);
                    }
                    if !schema
                        .registered_providers
                        .contains(&(expected.provider, expected.semantic_version))
                    {
                        return Err(PredicateError::UnknownEvidenceProvider(
                            expected.provider,
                            expected.semantic_version,
                        ));
                    }
                    if EvidenceTestId::from_schema(expected) != *test {
                        return Err(PredicateError::EvidenceTestIdentityMismatch(*test));
                    }
                    for (argument, ty) in arguments.iter().zip(&expected.arguments) {
                        if binding(argument)? != ty {
                            return Err(PredicateError::TypeMismatch);
                        }
                    }
                }
                PredicateNode::All(children) | PredicateNode::Any(children)
                    if children.is_empty() =>
                {
                    return Err(PredicateError::EmptyLogicalOperator);
                }
                _ => {}
            }
            let children: &[PredicateId] = match node {
                PredicateNode::All(children) | PredicateNode::Any(children) => children,
                PredicateNode::Not(child) => std::slice::from_ref(child),
                _ => &[],
            };
            active.insert(id);
            stack.push((id, true, depth));
            for child in children.iter().rev() {
                stack.push((*child, false, depth.saturating_add(1)));
            }
        }
        for (id, node) in &self.nodes {
            if !visited.contains(id) {
                return Err(PredicateError::UnreachableNode(*id));
            }
            if PredicateId::from_node(node) != *id {
                return Err(PredicateError::NodeIdentityMismatch(*id));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(byte: u8) -> CodeFingerprint {
        CodeFingerprint::from_bytes([byte; 32])
    }
    fn binding(byte: u8) -> ValueBindingId {
        ValueBindingId::from_canonical_fingerprint(fp(byte))
    }
    fn limits() -> PredicateLimits {
        PredicateLimits {
            max_nodes: 20,
            max_edges: 20,
            max_depth: 8,
            max_evidence_arguments: 4,
        }
    }
    fn insert(
        nodes: &mut BTreeMap<PredicateId, PredicateNode>,
        node: PredicateNode,
    ) -> PredicateId {
        let id = PredicateId::from_node(&node);
        nodes.insert(id, node);
        id
    }
    fn fixture() -> (PredicateGraph, PredicateSchema) {
        let left = binding(1);
        let right = binding(2);
        let mut schema = PredicateSchema::default();
        schema.bindings.insert(left, ValueType::Identity(fp(10)));
        schema.bindings.insert(right, ValueType::Identity(fp(10)));
        let mut nodes = BTreeMap::new();
        let equal = insert(&mut nodes, PredicateNode::Equal { left, right });
        let root = insert(&mut nodes, PredicateNode::Not(equal));
        (PredicateGraph { root, nodes }, schema)
    }

    #[test]
    fn validates_checked_types_and_rejects_tampered_nodes() {
        let (mut graph, mut schema) = fixture();
        assert_eq!(graph.validate(&schema, limits()), Ok(()));
        schema
            .bindings
            .insert(binding(2), ValueType::Identity(fp(11)));
        assert_eq!(
            graph.validate(&schema, limits()),
            Err(PredicateError::TypeMismatch)
        );
        schema
            .bindings
            .insert(binding(2), ValueType::Identity(fp(10)));
        let old_root = graph.root;
        let child = match graph.nodes[&old_root] {
            PredicateNode::Not(child) => child,
            _ => unreachable!(),
        };
        graph
            .nodes
            .insert(old_root, PredicateNode::All(vec![child]));
        assert_eq!(
            graph.validate(&schema, limits()),
            Err(PredicateError::NodeIdentityMismatch(old_root))
        );
    }

    #[test]
    fn evidence_and_pattern_must_be_registered_with_matching_types() {
        let value = binding(1);
        let provider = GuardProviderId::from_canonical_fingerprint(fp(2));
        let descriptor = EvidenceTestSchema {
            provider,
            semantic_version: 1,
            arguments: vec![ValueType::Text],
        };
        let evidence = EvidenceTestId::from_schema(&descriptor);
        let pattern = ValuePatternId::from_canonical_fingerprint(fp(3));
        let mut schema = PredicateSchema::default();
        schema.bindings.insert(value, ValueType::Text);
        let mut nodes = BTreeMap::new();
        let a = insert(
            &mut nodes,
            PredicateNode::Evidence {
                test: evidence,
                arguments: vec![value],
            },
        );
        let b = insert(&mut nodes, PredicateNode::PatternMember { value, pattern });
        let root = insert(&mut nodes, PredicateNode::All(vec![a, b]));
        let graph = PredicateGraph { root, nodes };
        assert_eq!(
            graph.validate(&schema, limits()),
            Err(PredicateError::UnknownEvidenceTest(evidence))
        );
        schema.evidence_tests.insert(evidence, descriptor.clone());
        assert_eq!(
            graph.validate(&schema, limits()),
            Err(PredicateError::UnknownEvidenceProvider(provider, 1))
        );
        schema.registered_providers.insert((provider, 2));
        assert_eq!(
            graph.validate(&schema, limits()),
            Err(PredicateError::UnknownEvidenceProvider(provider, 1))
        );
        schema.registered_providers.insert((provider, 1));
        let mut changed = descriptor.clone();
        changed.arguments = vec![ValueType::Bytes];
        schema.evidence_tests.insert(evidence, changed);
        assert_eq!(
            graph.validate(&schema, limits()),
            Err(PredicateError::EvidenceTestIdentityMismatch(evidence))
        );
        schema.evidence_tests.insert(evidence, descriptor);
        assert_eq!(
            graph.validate(&schema, limits()),
            Err(PredicateError::MissingPattern(pattern))
        );
        schema.patterns.insert(pattern, ValueType::Bytes);
        assert_eq!(
            graph.validate(&schema, limits()),
            Err(PredicateError::TypeMismatch)
        );
        schema.patterns.insert(pattern, ValueType::Text);
        assert_eq!(graph.validate(&schema, limits()), Ok(()));
    }

    #[test]
    fn rejects_cycles_missing_children_and_resource_exhaustion() {
        let id = PredicateId::from_canonical_fingerprint(fp(1));
        let graph = PredicateGraph {
            root: id,
            nodes: BTreeMap::from([(id, PredicateNode::Not(id))]),
        };
        assert_eq!(
            graph.validate(&PredicateSchema::default(), limits()),
            Err(PredicateError::Cycle(id))
        );
        let missing = PredicateId::from_canonical_fingerprint(fp(2));
        let graph = PredicateGraph {
            root: id,
            nodes: BTreeMap::from([(id, PredicateNode::Not(missing))]),
        };
        assert_eq!(
            graph.validate(&PredicateSchema::default(), limits()),
            Err(PredicateError::MissingNode(missing))
        );
        let (graph, schema) = fixture();
        let mut bound = limits();
        bound.max_nodes = 1;
        assert_eq!(
            graph.validate(&schema, bound),
            Err(PredicateError::NodeLimit)
        );
        bound = limits();
        bound.max_depth = 1;
        assert_eq!(
            graph.validate(&schema, bound),
            Err(PredicateError::DepthLimit)
        );
        bound = limits();
        bound.max_edges = 0;
        assert_eq!(
            graph.validate(&schema, bound),
            Err(PredicateError::EdgeLimit)
        );
    }

    #[test]
    fn checks_longest_path_even_when_shared_node_was_already_visited() {
        let mut nodes = BTreeMap::new();
        let leaf = insert(
            &mut nodes,
            PredicateNode::Equal {
                left: binding(1),
                right: binding(1),
            },
        );
        let middle = insert(&mut nodes, PredicateNode::Not(leaf));
        let longer = insert(&mut nodes, PredicateNode::Not(middle));
        let root = insert(&mut nodes, PredicateNode::All(vec![middle, longer]));
        let graph = PredicateGraph { root, nodes };
        let mut schema = PredicateSchema::default();
        schema.bindings.insert(binding(1), ValueType::Boolean);
        let mut bound = limits();
        bound.max_depth = 3;
        assert_eq!(
            graph.validate(&schema, bound),
            Err(PredicateError::DepthLimit)
        );
        bound.max_depth = 4;
        assert_eq!(graph.validate(&schema, bound), Ok(()));
    }

    #[test]
    fn binding_freezes_only_referenced_checked_schema() {
        let value = binding(1);
        let unused = binding(2);
        let provider = GuardProviderId::from_canonical_fingerprint(fp(3));
        let descriptor = EvidenceTestSchema {
            provider,
            semantic_version: 1,
            arguments: vec![ValueType::Text],
        };
        let evidence = EvidenceTestId::from_schema(&descriptor);
        let pattern = ValuePatternId::from_canonical_fingerprint(fp(4));
        let mut schema = PredicateSchema::default();
        schema.bindings.insert(value, ValueType::Text);
        schema.bindings.insert(unused, ValueType::Boolean);
        schema.patterns.insert(pattern, ValueType::Text);
        schema.evidence_tests.insert(evidence, descriptor.clone());
        schema.registered_providers.insert((provider, 1));
        let mut nodes = BTreeMap::new();
        let a = insert(
            &mut nodes,
            PredicateNode::Evidence {
                test: evidence,
                arguments: vec![value],
            },
        );
        let b = insert(&mut nodes, PredicateNode::PatternMember { value, pattern });
        let root = insert(&mut nodes, PredicateNode::All(vec![a, b]));
        let graph = PredicateGraph { root, nodes };
        let bound = graph.bind(&schema, limits()).unwrap();
        schema.bindings.insert(value, ValueType::Bytes);
        schema.patterns.clear();
        schema.evidence_tests.clear();
        assert_eq!(bound.graph().root, root);
        assert_eq!(bound.binding_type(value), Some(ValueType::Text));
        assert_eq!(bound.binding_type(unused), None);
        assert_eq!(bound.pattern_type(pattern), Some(ValueType::Text));
        assert_eq!(bound.evidence_test(evidence), Some(&descriptor));
        let mut next_version = descriptor.clone();
        next_version.semantic_version = 2;
        assert_ne!(EvidenceTestId::from_schema(&next_version), evidence);
    }

    #[test]
    fn invalid_graph_cannot_be_bound() {
        let (graph, mut schema) = fixture();
        schema.bindings.remove(&binding(1));
        assert!(
            matches!(graph.bind(&schema, limits()), Err(PredicateError::MissingBinding(id)) if id == binding(1))
        );
    }

    #[test]
    fn work_limit_counts_bindings_and_provider_references() {
        let value = binding(1);
        let mut schema = PredicateSchema::default();
        schema.bindings.insert(value, ValueType::Text);
        let mut nodes = BTreeMap::new();
        let root = insert(
            &mut nodes,
            PredicateNode::Equal {
                left: value,
                right: value,
            },
        );
        let graph = PredicateGraph { root, nodes };
        let mut bound = limits();
        bound.max_edges = 1;
        assert_eq!(
            graph.validate(&schema, bound),
            Err(PredicateError::EdgeLimit)
        );
        bound.max_edges = 2;
        assert_eq!(graph.validate(&schema, bound), Ok(()));

        let provider = GuardProviderId::from_canonical_fingerprint(fp(5));
        let descriptor = EvidenceTestSchema {
            provider,
            semantic_version: 1,
            arguments: vec![ValueType::Text],
        };
        let evidence = EvidenceTestId::from_schema(&descriptor);
        schema.evidence_tests.insert(evidence, descriptor);
        schema.registered_providers.insert((provider, 1));
        let mut nodes = BTreeMap::new();
        let root = insert(
            &mut nodes,
            PredicateNode::Evidence {
                test: evidence,
                arguments: vec![value],
            },
        );
        let graph = PredicateGraph { root, nodes };
        assert_eq!(
            graph.validate(&schema, bound),
            Err(PredicateError::EdgeLimit)
        );
        bound.max_edges = 3;
        assert_eq!(graph.validate(&schema, bound), Ok(()));
    }
}
