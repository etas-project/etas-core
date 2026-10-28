use std::sync::Arc;

use etas_code::{
    CallableId, CodeFingerprint, DefinitionKey, DefinitionKind, PackageKey, RevisionInputs,
    RevisionKey, definition::*,
};
use etas_core::memory::MemoryFootprint;

fn revision(body: u8) -> RevisionKey {
    let hash = |value| CodeFingerprint::from_bytes([value; 32]);
    let definition = DefinitionKey::new(
        PackageKey::from_canonical_fingerprint(hash(1)),
        vec!["app".into()],
        vec!["Worker".into()],
        DefinitionKind::Agent,
    )
    .unwrap();
    RevisionKey::from_inputs(&RevisionInputs {
        definition: CallableId::declaration(definition).unwrap(),
        content: hash(body),
        environment: hash(3),
        contract: hash(4),
        compilation: hash(5),
        parent: None,
    })
}

#[test]
fn draft_invalidates_current_analysis_without_mutating_or_copying_the_base() {
    let input = AnalysisFingerprint::new(revision(2), CodeFingerprint::from_bytes([6; 32]));
    let base = DefinitionAnalysisState::Checked {
        input,
        summaries: Arc::new(vec!["requested: Console.stdout_write".to_string()]),
    };
    let draft = base.after_body_edit();
    assert_eq!(base.current(input).unwrap().len(), 1);
    assert_eq!(
        draft.current(input),
        Err(AnalysisUnavailableReason::EditedBody)
    );
    let DefinitionAnalysisState::BaselineOnly {
        base: baseline,
        summaries,
    } = &draft
    else {
        panic!("baseline")
    };
    assert_eq!(*baseline, input);
    assert!(std::ptr::eq(base.current(input).unwrap(), &**summaries));
    let base_bytes = MemoryFootprint::of(&base).estimated_bytes();
    assert_eq!(
        MemoryFootprint::of(&(base.clone(), draft.clone())).estimated_bytes(),
        base_bytes
    );
    let again = draft.after_body_edit();
    assert!(matches!(again, DefinitionAnalysisState::BaselineOnly { base, .. } if base == input));
}

#[test]
fn no_current_analysis_is_fabricated_for_missing_or_rejected_facts() {
    let input = AnalysisFingerprint::new(revision(2), CodeFingerprint::from_bytes([6; 32]));
    for reason in [
        AnalysisUnavailableReason::NotAnalyzed,
        AnalysisUnavailableReason::Rejected,
        AnalysisUnavailableReason::MissingDefaultEntry,
        AnalysisUnavailableReason::MissingDependency,
    ] {
        let state = DefinitionAnalysisState::<Vec<String>>::Unavailable { reason };
        assert_eq!(state.current(input), Err(reason));
        assert_eq!(state.after_body_edit().current(input), Err(reason));
    }
}

#[test]
fn checked_results_are_bound_to_revision_and_analysis_configuration() {
    let input = AnalysisFingerprint::new(revision(2), CodeFingerprint::from_bytes([6; 32]));
    let state = DefinitionAnalysisState::Checked {
        input,
        summaries: Arc::new(Vec::<String>::new()),
    };
    assert!(state.current(input).unwrap().is_empty());
    for changed in [
        AnalysisFingerprint::new(revision(9), CodeFingerprint::from_bytes([6; 32])),
        AnalysisFingerprint::new(revision(2), CodeFingerprint::from_bytes([7; 32])),
    ] {
        assert_ne!(input.fingerprint(), changed.fingerprint());
        assert_eq!(
            state.current(changed),
            Err(AnalysisUnavailableReason::InputMismatch)
        );
    }
}
