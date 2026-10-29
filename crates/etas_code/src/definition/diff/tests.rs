use super::*;
use crate::{CallableId, DefinitionKind, PackageKey, RevisionInputs};

fn fp(value: &str) -> CodeFingerprint {
    CodeFingerprint::of_bytes("etas.test.definition-diff", value.as_bytes())
}

fn identity(name: &str) -> DefinitionKey {
    DefinitionKey::new(
        PackageKey::from_canonical_fingerprint(fp("package")),
        vec!["app".into()],
        vec![name.into()],
        DefinitionKind::Agent,
    )
    .unwrap()
}

fn revision(identity: &DefinitionKey) -> RevisionInfo {
    RevisionInfo::new(RevisionInputs {
        definition: CallableId::declaration(identity.clone()).unwrap(),
        content: fp("body"),
        environment: fp("environment"),
        contract: fp("contract"),
        compilation: fp("compilation"),
        parent: None,
    })
}

fn snapshot<'a>(
    identity: &'a DefinitionKey,
    revision: &'a RevisionInfo,
) -> DefinitionDiffSnapshot<'a> {
    DefinitionDiffSnapshot {
        identity,
        revision,
        signature: Some(fp("signature")),
        body: Some(fp("body")),
        declared_bound: None,
        escaping: Some(fp("empty")),
        requested: Some(fp("requested")),
        default_handled: Some(fp("handled")),
        determinism: Some(DeterminismInfo::Deterministic),
        trace: Some(fp("trace")),
    }
}

#[test]
fn identical_known_observations_are_known_unchanged() {
    let identity = identity("Worker");
    let revision = revision(&identity);
    let report = ChangeReport::compare(
        snapshot(&identity, &revision),
        snapshot(&identity, &revision),
    )
    .unwrap();
    assert_eq!(report.body_changed, Some(false));
    assert_eq!(report.escaping_changed, Some(false));
    assert_eq!(report.requested_changed, Some(false));
    assert_eq!(report.default_handled_changed, Some(false));
    assert_eq!(report.determinism_changed, Some(false));
    assert_eq!(report.trace_changed, Some(false));
    assert!(!report.signature_changed);
    assert!(!report.dependencies_changed);
    assert!(!report.declared_bound_changed);
}

#[test]
fn unavailable_observations_do_not_become_known_unchanged() {
    let identity = identity("Worker");
    let revision = revision(&identity);
    let mut edited = snapshot(&identity, &revision);
    edited.body = None;
    edited.escaping = None;
    edited.requested = None;
    edited.default_handled = None;
    edited.determinism = None;
    edited.trace = None;
    let report = ChangeReport::compare(snapshot(&identity, &revision), edited).unwrap();
    assert!(report.old_body_available);
    assert!(!report.new_body_available);
    assert_eq!(report.body_changed, None);
    assert_eq!(report.escaping_changed, None);
    assert_eq!(report.requested_changed, None);
    assert_eq!(report.default_handled_changed, None);
    assert_eq!(report.determinism_changed, None);
    assert_eq!(report.trace_changed, None);
}

#[test]
fn requested_actions_remain_independent_of_escaping_effects() {
    let identity = identity("Worker");
    let revision = revision(&identity);
    let mut edited = snapshot(&identity, &revision);
    edited.requested = Some(fp("different requests"));
    edited.trace = Some(fp("different order"));
    let report = ChangeReport::compare(snapshot(&identity, &revision), edited).unwrap();
    assert_eq!(report.escaping_changed, Some(false));
    assert_eq!(report.requested_changed, Some(true));
    assert_eq!(report.default_handled_changed, Some(false));
    assert_eq!(report.trace_changed, Some(true));
}

#[test]
fn different_agent_identities_cannot_share_a_change_report() {
    let first = identity("Worker");
    let second = identity("Other");
    let first_revision = revision(&first);
    let second_revision = revision(&second);
    assert!(matches!(
        ChangeReport::compare(
            snapshot(&first, &first_revision),
            snapshot(&second, &second_revision)
        ),
        Err(super::super::DefinitionOperationError::IncompatibleDefinitions)
    ));
}

#[test]
fn flow_reports_preserve_their_definition_kind_and_reject_foreign_revisions() {
    let flow = DefinitionKey::new(
        PackageKey::from_canonical_fingerprint(fp("package")),
        vec!["app".into()],
        vec!["normalize".into()],
        DefinitionKind::Flow,
    )
    .unwrap();
    let revision = revision(&flow);
    let report =
        ChangeReport::compare(snapshot(&flow, &revision), snapshot(&flow, &revision)).unwrap();
    assert_eq!(report.identity.kind(), DefinitionKind::Flow);
    let other = identity("Worker");
    assert!(
        ChangeReport::compare(snapshot(&other, &revision), snapshot(&other, &revision)).is_err()
    );
}

#[test]
fn agent_default_method_revisions_are_bound_to_the_same_selected_entry() {
    let owner = identity("Worker");
    let implementation = crate::ImplementationKey::new(
        owner.package(),
        owner.module().to_vec(),
        owner.clone(),
        None,
        fp("impl"),
    )
    .unwrap();
    let mut inputs = revision(&owner).inputs().clone();
    inputs.definition =
        CallableId::method(crate::MethodId::new(implementation.clone(), "entry".into()).unwrap());
    let first = RevisionInfo::new(inputs.clone());
    assert!(ChangeReport::compare(snapshot(&owner, &first), snapshot(&owner, &first)).is_ok());
    inputs.definition =
        CallableId::method(crate::MethodId::new(implementation, "auxiliary".into()).unwrap());
    let other = RevisionInfo::new(inputs);
    assert!(ChangeReport::compare(snapshot(&owner, &first), snapshot(&owner, &other)).is_err());
}
