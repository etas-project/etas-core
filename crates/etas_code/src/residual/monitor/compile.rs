//! Shared temporal clause construction. A matcher must consult the evidence
//! for the current target and scope; an earlier request is not evidence.

use etas_utils::automaton::{NoTransitionPolicy, SymbolicAutomaton};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorEvidenceState {
    Active,
    Rejected,
}

/// Matches a target event only when it lacks valid matching prior evidence.
/// The adapter owns the concrete evidence check, including correlation,
/// outcome, scope and validity; this label is not an authorization token.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorEvidenceLabel<Pattern> {
    Missing { guard: Pattern, target: Pattern },
}

/// `G >> A` and `A << G` use the same temporal component. Permissions and OR
/// alternatives are normalized separately, before product construction.
pub fn compile_prior_evidence<Pattern: Clone>(
    guard: &Pattern,
    target: &Pattern,
) -> SymbolicAutomaton<PriorEvidenceState, PriorEvidenceLabel<Pattern>> {
    let mut automaton =
        SymbolicAutomaton::new(PriorEvidenceState::Active, NoTransitionPolicy::Stay);
    automaton.add_transition(
        PriorEvidenceState::Active,
        PriorEvidenceState::Rejected,
        PriorEvidenceLabel::Missing {
            guard: guard.clone(),
            target: target.clone(),
        },
    );
    automaton.mark_accepting(PriorEvidenceState::Active);
    automaton.mark_rejecting(PriorEvidenceState::Rejected);
    automaton
}

#[cfg(test)]
mod tests {
    use super::*;
    use etas_utils::automaton::Matcher;

    struct Event {
        action: &'static str,
        key: u32,
        scope: u32,
    }

    struct Evidence {
        key: u32,
        scope: u32,
        accepted: bool,
    }

    struct ExactMatcher<'a>(&'a [Evidence]);

    impl Matcher<PriorEvidenceLabel<&'static str>, Event> for ExactMatcher<'_> {
        fn matches(&self, label: &PriorEvidenceLabel<&'static str>, event: &Event) -> bool {
            let PriorEvidenceLabel::Missing { guard, target } = label;
            event.action == *target
                && !self.0.iter().any(|evidence| {
                    *guard == "approval"
                        && evidence.accepted
                        && evidence.key == event.key
                        && evidence.scope == event.scope
                })
        }
    }

    #[test]
    fn target_requires_matching_accepted_evidence_and_rejection_persists() {
        let machine = compile_prior_evidence(&"approval", &"send");
        let event = Event {
            action: "send",
            key: 7,
            scope: 3,
        };
        for evidence in [
            vec![],
            vec![Evidence {
                key: 7,
                scope: 3,
                accepted: false,
            }],
            vec![Evidence {
                key: 8,
                scope: 3,
                accepted: true,
            }],
            vec![Evidence {
                key: 7,
                scope: 4,
                accepted: true,
            }],
        ] {
            let step = machine.step(
                &PriorEvidenceState::Active,
                &event,
                &ExactMatcher(&evidence),
            );
            assert!(step.rejecting.contains(&PriorEvidenceState::Rejected));
            let later = machine.step(&PriorEvidenceState::Rejected, &event, &ExactMatcher(&[]));
            assert!(later.rejecting.contains(&PriorEvidenceState::Rejected));
        }
        let matching = [Evidence {
            key: 7,
            scope: 3,
            accepted: true,
        }];
        let step = machine.step(
            &PriorEvidenceState::Active,
            &event,
            &ExactMatcher(&matching),
        );
        assert!(step.accepting.contains(&PriorEvidenceState::Active));
        assert!(step.rejecting.is_empty());
    }
}
