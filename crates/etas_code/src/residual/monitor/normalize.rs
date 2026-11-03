use std::collections::BTreeSet;

/// Input to normalization for one conjunctive group, never multiple OR branches.
pub enum TraceClause<'a, Pattern, Origin> {
    Allow {
        pattern: &'a Pattern,
        origin: Origin,
    },
    Deny {
        pattern: &'a Pattern,
        origin: Origin,
    },
    Before {
        guard: &'a Pattern,
        target: &'a Pattern,
        origin: Origin,
    },
}

pub struct PriorRequirement<'a, Pattern, Origin> {
    pub guard: &'a Pattern,
    pub target: &'a Pattern,
    pub origin: Origin,
}

/// One complete alternative. Normalization never proves a predicate or grants
/// runtime authority, and separate scopes/alternatives remain separate values.
pub struct NormalizedTraceAlternative<'a, Pattern, Origin> {
    pub permissions: Vec<(&'a Pattern, Origin)>,
    pub denials: Vec<(&'a Pattern, Origin)>,
    pub prerequisites: Vec<PriorRequirement<'a, Pattern, Origin>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalizeError {
    ClauseLimit,
}

pub fn normalize_trace_alternative<'a, Pattern: Ord, Origin: Copy>(
    clauses: impl IntoIterator<Item = TraceClause<'a, Pattern, Origin>>,
    max_clauses: usize,
) -> Result<NormalizedTraceAlternative<'a, Pattern, Origin>, NormalizeError> {
    let mut seen = BTreeSet::new();
    let mut result = NormalizedTraceAlternative {
        permissions: Vec::new(),
        denials: Vec::new(),
        prerequisites: Vec::new(),
    };
    for (index, clause) in clauses.into_iter().enumerate() {
        if index >= max_clauses {
            return Err(NormalizeError::ClauseLimit);
        }
        let (first, second, origin) = match clause {
            TraceClause::Allow { pattern, origin } => (Some(pattern), None, origin),
            TraceClause::Deny { pattern, origin } => {
                result.denials.push((pattern, origin));
                (None, None, origin)
            }
            TraceClause::Before {
                guard,
                target,
                origin,
            } => {
                result.prerequisites.push(PriorRequirement {
                    guard,
                    target,
                    origin,
                });
                (Some(guard), Some(target), origin)
            }
        };
        for pattern in [first, second].into_iter().flatten() {
            if seen.insert(pattern) {
                result.permissions.push((pattern, origin));
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_denials_prerequisites_and_local_permissions() {
        let normalized = normalize_trace_alternative(
            [
                TraceClause::Allow {
                    pattern: &"other",
                    origin: 1,
                },
                TraceClause::Before {
                    guard: &"guard",
                    target: &"target",
                    origin: 2,
                },
                TraceClause::Allow {
                    pattern: &"target",
                    origin: 3,
                },
                TraceClause::Deny {
                    pattern: &"blocked",
                    origin: 4,
                },
            ],
            4,
        )
        .unwrap();
        assert_eq!(
            normalized.permissions,
            vec![(&"other", 1), (&"guard", 2), (&"target", 2)]
        );
        assert_eq!(normalized.denials, vec![(&"blocked", 4)]);
        assert_eq!(normalized.prerequisites.len(), 1);
        assert_eq!(
            (
                normalized.prerequisites[0].guard,
                normalized.prerequisites[0].target,
                normalized.prerequisites[0].origin
            ),
            (&"guard", &"target", 2)
        );
    }

    #[test]
    fn alternatives_do_not_pool_permissions_and_limits_fail_closed() {
        let first = normalize_trace_alternative(
            [TraceClause::Before {
                guard: &"a",
                target: &"b",
                origin: 0,
            }],
            1,
        )
        .unwrap();
        let second = normalize_trace_alternative(
            [TraceClause::Allow {
                pattern: &"c",
                origin: 1,
            }],
            1,
        )
        .unwrap();
        assert_eq!(first.permissions, vec![(&"a", 0), (&"b", 0)]);
        assert_eq!(second.permissions, vec![(&"c", 1)]);
        assert!(second.prerequisites.is_empty());
        assert!(matches!(
            normalize_trace_alternative(
                [TraceClause::Allow {
                    pattern: &"c",
                    origin: 1,
                }],
                0
            ),
            Err(NormalizeError::ClauseLimit)
        ));
    }
}
