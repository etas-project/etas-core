use std::collections::BTreeMap;

use crate::CodeFingerprint;

use super::{BoundPredicateGraph, PredicateId, PredicateNode, ValueBindingId, ValueType};

/// Borrowed checked values are never copied into diagnostics or monitor state.
/// Identity bytes must be the exact checked identity, not a redacted digest of
/// data whose equality is being tested. This type intentionally lacks Debug.
#[derive(Clone, Copy)]
pub enum BorrowedValue<'a> {
    Boolean(bool),
    Signed {
        ty: CodeFingerprint,
        value: i128,
    },
    Unsigned {
        ty: CodeFingerprint,
        value: u128,
    },
    Text(&'a str),
    Bytes(&'a [u8]),
    Identity {
        ty: CodeFingerprint,
        value: &'a [u8],
    },
}

impl BorrowedValue<'_> {
    pub(crate) fn value_type(self) -> ValueType {
        match self {
            Self::Boolean(_) => ValueType::Boolean,
            Self::Signed { ty, .. } | Self::Unsigned { ty, .. } => ValueType::Integer(ty),
            Self::Text(_) => ValueType::Text,
            Self::Bytes(_) => ValueType::Bytes,
            Self::Identity { ty, .. } => ValueType::Identity(ty),
        }
    }

    pub(crate) fn byte_len(self) -> usize {
        match self {
            Self::Text(value) => value.len(),
            Self::Bytes(value) | Self::Identity { value, .. } => value.len(),
            Self::Boolean(_) | Self::Signed { .. } | Self::Unsigned { .. } => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredicateDecision {
    Satisfied,
    Violated,
    NeedsEvidence,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PredicateEvaluationLimits {
    pub max_nodes: usize,
    pub max_bindings: usize,
    pub max_work: usize,
    pub max_value_bytes: usize,
    pub max_comparison_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredicateEvaluationError {
    NodeLimit,
    BindingLimit,
    WorkLimit,
    ValueSizeLimit,
    ComparisonLimit,
    MissingCheckedNode(PredicateId),
    MissingCheckedBinding(ValueBindingId),
    TypeMismatch(ValueBindingId),
    IntegerRepresentationMismatch,
    InconsistentCheckedTypes,
}

struct Budget {
    remaining_work: usize,
    remaining_comparison_bytes: usize,
    max_value_bytes: usize,
}

impl Budget {
    fn work(&mut self, amount: usize) -> Result<(), PredicateEvaluationError> {
        self.remaining_work = self
            .remaining_work
            .checked_sub(amount)
            .ok_or(PredicateEvaluationError::WorkLimit)?;
        Ok(())
    }

    fn value(&self, value: BorrowedValue<'_>) -> Result<(), PredicateEvaluationError> {
        if value.byte_len() > self.max_value_bytes {
            return Err(PredicateEvaluationError::ValueSizeLimit);
        }
        Ok(())
    }

    fn compare(
        &mut self,
        left: BorrowedValue<'_>,
        right: BorrowedValue<'_>,
    ) -> Result<bool, PredicateEvaluationError> {
        let bytes = left
            .byte_len()
            .checked_add(right.byte_len())
            .ok_or(PredicateEvaluationError::ComparisonLimit)?;
        self.remaining_comparison_bytes = self
            .remaining_comparison_bytes
            .checked_sub(bytes)
            .ok_or(PredicateEvaluationError::ComparisonLimit)?;
        match (left, right) {
            (BorrowedValue::Boolean(a), BorrowedValue::Boolean(b)) => Ok(a == b),
            (BorrowedValue::Signed { value: a, .. }, BorrowedValue::Signed { value: b, .. }) => {
                Ok(a == b)
            }
            (
                BorrowedValue::Unsigned { value: a, .. },
                BorrowedValue::Unsigned { value: b, .. },
            ) => Ok(a == b),
            (BorrowedValue::Text(a), BorrowedValue::Text(b)) => Ok(a == b),
            (BorrowedValue::Bytes(a), BorrowedValue::Bytes(b)) => Ok(a == b),
            (
                BorrowedValue::Identity { value: a, .. },
                BorrowedValue::Identity { value: b, .. },
            ) => Ok(a == b),
            (BorrowedValue::Signed { .. }, BorrowedValue::Unsigned { .. })
            | (BorrowedValue::Unsigned { .. }, BorrowedValue::Signed { .. }) => {
                Err(PredicateEvaluationError::IntegerRepresentationMismatch)
            }
            _ => Err(PredicateEvaluationError::InconsistentCheckedTypes),
        }
    }
}

impl BoundPredicateGraph {
    /// Evaluates the supported data-only operators. Bindings are supplied by a
    /// trusted checked-site adapter; neither this result nor the input map is
    /// an installation proof or a permit to dispatch an action. Pattern and
    /// provider tests remain explicitly unsupported until their checked native
    /// semantics and evidence adapters are installed.
    pub fn evaluate_borrowed(
        &self,
        bindings: &BTreeMap<ValueBindingId, BorrowedValue<'_>>,
        limits: PredicateEvaluationLimits,
    ) -> Result<PredicateDecision, PredicateEvaluationError> {
        self.evaluate_with(bindings.len(), |id| bindings.get(&id).copied(), limits)
    }

    pub fn evaluate_validated(
        &self,
        bindings: &crate::residual::binding::ValidatedBorrowedBindings<'_, '_>,
        limits: PredicateEvaluationLimits,
    ) -> Result<PredicateDecision, PredicateEvaluationError> {
        self.evaluate_with(bindings.supplied_count(), |id| bindings.get(id), limits)
    }

    fn evaluate_with<'value>(
        &self,
        supplied_count: usize,
        get: impl Fn(ValueBindingId) -> Option<BorrowedValue<'value>>,
        limits: PredicateEvaluationLimits,
    ) -> Result<PredicateDecision, PredicateEvaluationError> {
        if self.graph.nodes.len() > limits.max_nodes {
            return Err(PredicateEvaluationError::NodeLimit);
        }
        if self.binding_count() > limits.max_bindings || supplied_count > limits.max_bindings {
            return Err(PredicateEvaluationError::BindingLimit);
        }
        let mut budget = Budget {
            remaining_work: limits.max_work,
            remaining_comparison_bytes: limits.max_comparison_bytes,
            max_value_bytes: limits.max_value_bytes,
        };
        let mut values = BTreeMap::new();
        let mut pending = vec![(self.graph.root, false)];
        while let Some((id, leaving)) = pending.pop() {
            budget.work(1)?;
            if values.contains_key(&id) {
                continue;
            }
            let node = self
                .graph
                .nodes
                .get(&id)
                .ok_or(PredicateEvaluationError::MissingCheckedNode(id))?;
            if !leaving {
                let children: &[PredicateId] = match node {
                    PredicateNode::All(children) | PredicateNode::Any(children) => children,
                    PredicateNode::Not(child) => std::slice::from_ref(child),
                    _ => &[],
                };
                if !children.is_empty() {
                    // Bound the work before expanding a broad node's stack.
                    budget.work(
                        children
                            .len()
                            .checked_add(1)
                            .ok_or(PredicateEvaluationError::WorkLimit)?,
                    )?;
                    pending.push((id, true));
                    pending.extend(children.iter().rev().map(|child| (*child, false)));
                    continue;
                }
            }
            let lookup = |binding: ValueBindingId| -> Result<Option<BorrowedValue<'_>>, PredicateEvaluationError> {
                let ty = self.binding_type(binding).ok_or(PredicateEvaluationError::MissingCheckedBinding(binding))?;
                let value = get(binding);
                if let Some(value) = value {
                    if value.value_type() != ty {
                        return Err(PredicateEvaluationError::TypeMismatch(binding));
                    }
                    budget.value(value)?;
                }
                Ok(value)
            };
            let child = |id: &PredicateId| {
                values
                    .get(id)
                    .copied()
                    .ok_or(PredicateEvaluationError::MissingCheckedNode(*id))
            };
            let decision = match node {
                PredicateNode::Equal { left, right } => match (lookup(*left)?, lookup(*right)?) {
                    (Some(left), Some(right)) => {
                        if budget.compare(left, right)? {
                            PredicateDecision::Satisfied
                        } else {
                            PredicateDecision::Violated
                        }
                    }
                    _ => PredicateDecision::NeedsEvidence,
                },
                PredicateNode::All(children) => {
                    let mut result = PredicateDecision::Satisfied;
                    for id in children {
                        result = conjunction(result, child(id)?);
                    }
                    result
                }
                PredicateNode::Any(children) => {
                    let mut result = PredicateDecision::Violated;
                    for id in children {
                        result = disjunction(result, child(id)?);
                    }
                    result
                }
                PredicateNode::Not(id) => match child(id)? {
                    PredicateDecision::Satisfied => PredicateDecision::Violated,
                    PredicateDecision::Violated => PredicateDecision::Satisfied,
                    other => other,
                },
                PredicateNode::PatternMember { .. } | PredicateNode::Evidence { .. } => {
                    PredicateDecision::Unsupported
                }
            };
            values.insert(id, decision);
        }
        values
            .get(&self.graph.root)
            .copied()
            .ok_or(PredicateEvaluationError::MissingCheckedNode(
                self.graph.root,
            ))
    }
}

fn conjunction(left: PredicateDecision, right: PredicateDecision) -> PredicateDecision {
    use PredicateDecision::*;
    match (left, right) {
        (Violated, _) | (_, Violated) => Violated,
        (Unsupported, _) | (_, Unsupported) => Unsupported,
        (NeedsEvidence, _) | (_, NeedsEvidence) => NeedsEvidence,
        (Satisfied, Satisfied) => Satisfied,
    }
}

fn disjunction(left: PredicateDecision, right: PredicateDecision) -> PredicateDecision {
    use PredicateDecision::*;
    match (left, right) {
        (Unsupported, _) | (_, Unsupported) => Unsupported,
        (Satisfied, _) | (_, Satisfied) => Satisfied,
        (NeedsEvidence, _) | (_, NeedsEvidence) => NeedsEvidence,
        (Violated, Violated) => Violated,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        BorrowedPredicateSupportError, PredicateGraph, PredicateLimits, PredicateSchema,
        ValuePatternId,
    };
    use super::*;

    fn fp(byte: u8) -> CodeFingerprint {
        CodeFingerprint::from_bytes([byte; 32])
    }
    fn binding(byte: u8) -> ValueBindingId {
        ValueBindingId::from_canonical_fingerprint(fp(byte))
    }
    fn singleton(node: PredicateNode) -> (PredicateId, BTreeMap<PredicateId, PredicateNode>) {
        let root = PredicateId::from_node(&node);
        (root, BTreeMap::from([(root, node)]))
    }
    fn graph_limits() -> PredicateLimits {
        PredicateLimits {
            max_nodes: 32,
            max_edges: 32,
            max_depth: 8,
            max_evidence_arguments: 4,
        }
    }
    fn eval_limits() -> PredicateEvaluationLimits {
        PredicateEvaluationLimits {
            max_nodes: 32,
            max_bindings: 8,
            max_work: 64,
            max_value_bytes: 16,
            max_comparison_bytes: 32,
        }
    }

    #[test]
    fn checked_equality_requires_exact_borrowed_values_and_types() {
        let left = binding(1);
        let right = binding(2);
        let (root, nodes) = singleton(PredicateNode::Equal { left, right });
        let mut schema = PredicateSchema::default();
        schema.bindings.insert(left, ValueType::Text);
        schema.bindings.insert(right, ValueType::Text);
        let graph = PredicateGraph { root, nodes }
            .bind(&schema, graph_limits())
            .unwrap();
        let mut values = BTreeMap::from([(left, BorrowedValue::Text("draft"))]);
        assert_eq!(
            graph.evaluate_borrowed(&values, eval_limits()),
            Ok(PredicateDecision::NeedsEvidence)
        );
        values.insert(right, BorrowedValue::Text("draft"));
        assert_eq!(
            graph.evaluate_borrowed(&values, eval_limits()),
            Ok(PredicateDecision::Satisfied)
        );
        values.insert(right, BorrowedValue::Text("other"));
        assert_eq!(
            graph.evaluate_borrowed(&values, eval_limits()),
            Ok(PredicateDecision::Violated)
        );
        values.insert(right, BorrowedValue::Bytes(b"draft"));
        assert_eq!(
            graph.evaluate_borrowed(&values, eval_limits()),
            Err(PredicateEvaluationError::TypeMismatch(right))
        );
    }

    #[test]
    fn logical_results_do_not_turn_missing_or_unsupported_into_success() {
        let left = binding(1);
        let right = binding(2);
        let pattern = ValuePatternId::from_canonical_fingerprint(fp(3));
        let equal = PredicateNode::Equal { left, right };
        let member = PredicateNode::PatternMember {
            value: left,
            pattern,
        };
        let equal_id = PredicateId::from_node(&equal);
        let member_id = PredicateId::from_node(&member);
        let mut schema = PredicateSchema::default();
        schema.bindings.insert(left, ValueType::Text);
        schema.bindings.insert(right, ValueType::Text);
        schema.patterns.insert(pattern, ValueType::Text);
        let make = |all| {
            let root_node = if all {
                PredicateNode::All(vec![equal_id, member_id])
            } else {
                PredicateNode::Any(vec![equal_id, member_id])
            };
            let root = PredicateId::from_node(&root_node);
            PredicateGraph {
                root,
                nodes: BTreeMap::from([
                    (equal_id, equal.clone()),
                    (member_id, member.clone()),
                    (root, root_node),
                ]),
            }
            .bind(&schema, graph_limits())
            .unwrap()
        };
        let mut values = BTreeMap::from([(left, BorrowedValue::Text("a"))]);
        assert_eq!(
            make(false).validate_borrowed_support(eval_limits()),
            Err(BorrowedPredicateSupportError::UnsupportedPattern(member_id))
        );
        assert_eq!(
            make(true).evaluate_borrowed(&values, eval_limits()),
            Ok(PredicateDecision::Unsupported)
        );
        values.insert(right, BorrowedValue::Text("b"));
        assert_eq!(
            make(true).evaluate_borrowed(&values, eval_limits()),
            Ok(PredicateDecision::Violated)
        );
        assert_eq!(
            make(false).evaluate_borrowed(&values, eval_limits()),
            Ok(PredicateDecision::Unsupported)
        );
        values.insert(right, BorrowedValue::Text("a"));
        assert_eq!(
            make(false).evaluate_borrowed(&values, eval_limits()),
            Ok(PredicateDecision::Unsupported)
        );
    }

    #[test]
    fn evaluation_enforces_work_byte_and_integer_identity_bounds() {
        let left = binding(1);
        let right = binding(2);
        let (root, nodes) = singleton(PredicateNode::Equal { left, right });
        let mut schema = PredicateSchema::default();
        schema.bindings.insert(left, ValueType::Text);
        schema.bindings.insert(right, ValueType::Text);
        let graph = PredicateGraph { root, nodes }
            .bind(&schema, graph_limits())
            .unwrap();
        let values = BTreeMap::from([
            (left, BorrowedValue::Text("1234")),
            (right, BorrowedValue::Text("1234")),
        ]);
        let mut limits = eval_limits();
        limits.max_value_bytes = 3;
        assert_eq!(
            graph.evaluate_borrowed(&values, limits),
            Err(PredicateEvaluationError::ValueSizeLimit)
        );
        limits.max_value_bytes = 4;
        limits.max_comparison_bytes = 7;
        assert_eq!(
            graph.evaluate_borrowed(&values, limits),
            Err(PredicateEvaluationError::ComparisonLimit)
        );
        limits.max_comparison_bytes = 8;
        limits.max_work = 0;
        assert_eq!(
            graph.evaluate_borrowed(&values, limits),
            Err(PredicateEvaluationError::WorkLimit)
        );
        limits.max_work = 1;
        limits.max_bindings = 1;
        assert_eq!(
            graph.evaluate_borrowed(&values, limits),
            Err(PredicateEvaluationError::BindingLimit)
        );

        schema.bindings.insert(left, ValueType::Integer(fp(4)));
        schema.bindings.insert(right, ValueType::Integer(fp(4)));
        let (root, nodes) = singleton(PredicateNode::Equal { left, right });
        let graph = PredicateGraph { root, nodes }
            .bind(&schema, graph_limits())
            .unwrap();
        let values = BTreeMap::from([
            (
                left,
                BorrowedValue::Signed {
                    ty: fp(4),
                    value: 1,
                },
            ),
            (
                right,
                BorrowedValue::Unsigned {
                    ty: fp(4),
                    value: 1,
                },
            ),
        ]);
        assert_eq!(
            graph.evaluate_borrowed(&values, eval_limits()),
            Err(PredicateEvaluationError::IntegerRepresentationMismatch)
        );
    }

    #[test]
    fn nominal_identity_and_nested_work_are_checked_without_recursion() {
        let left = binding(1);
        let right = binding(2);
        let equal = PredicateNode::Equal { left, right };
        let equal_id = PredicateId::from_node(&equal);
        let not = PredicateNode::Not(equal_id);
        let root = PredicateId::from_node(&not);
        let mut schema = PredicateSchema::default();
        schema.bindings.insert(left, ValueType::Identity(fp(4)));
        schema.bindings.insert(right, ValueType::Identity(fp(4)));
        let graph = PredicateGraph {
            root,
            nodes: BTreeMap::from([(equal_id, equal), (root, not)]),
        }
        .bind(&schema, graph_limits())
        .unwrap();
        let mut support_limits = eval_limits();
        support_limits.max_work = 4;
        assert_eq!(
            graph.validate_borrowed_support(support_limits),
            Err(BorrowedPredicateSupportError::WorkLimit)
        );
        support_limits.max_work = 5;
        assert_eq!(graph.validate_borrowed_support(support_limits), Ok(()));
        let mut values = BTreeMap::from([
            (
                left,
                BorrowedValue::Identity {
                    ty: fp(4),
                    value: b"one",
                },
            ),
            (
                right,
                BorrowedValue::Identity {
                    ty: fp(4),
                    value: b"two",
                },
            ),
        ]);
        assert_eq!(
            graph.evaluate_borrowed(&values, eval_limits()),
            Ok(PredicateDecision::Satisfied)
        );
        values.insert(
            right,
            BorrowedValue::Identity {
                ty: fp(5),
                value: b"two",
            },
        );
        assert_eq!(
            graph.evaluate_borrowed(&values, eval_limits()),
            Err(PredicateEvaluationError::TypeMismatch(right))
        );
        let mut limits = eval_limits();
        limits.max_nodes = 1;
        assert_eq!(
            graph.evaluate_borrowed(&values, limits),
            Err(PredicateEvaluationError::NodeLimit)
        );
        limits.max_nodes = 2;
        limits.max_work = 2;
        assert_eq!(
            graph.evaluate_borrowed(&values, limits),
            Err(PredicateEvaluationError::WorkLimit)
        );
    }
}
