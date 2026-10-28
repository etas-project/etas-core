use etas_code::{
    CallableId, CodeFingerprint, DefinitionKey, DefinitionKind, ImplementationHeader,
    ImplementationKey, ImplementationSpecIdentity, MethodId, PackageKey, RevisionInfo,
    RevisionInputs, RevisionKey,
};
use etas_core::memory::MemoryFootprint;

fn fingerprint(text: &str) -> CodeFingerprint {
    CodeFingerprint::of_bytes("etas.code.test.input.v1", text.as_bytes())
}

fn package(origin: &str) -> PackageKey {
    PackageKey::from_canonical_fingerprint(fingerprint(origin))
}

fn definition(origin: &str, module: &[&str], name: &str, kind: DefinitionKind) -> DefinitionKey {
    DefinitionKey::new(
        package(origin),
        module.iter().map(|value| (*value).to_owned()).collect(),
        vec![name.to_owned()],
        kind,
    )
    .unwrap()
}

#[test]
fn package_module_declaration_and_kind_are_all_part_of_identity() {
    let baseline = definition(
        "registry:a@1",
        &["app", "agent"],
        "Worker",
        DefinitionKind::Agent,
    );
    for other in [
        definition(
            "registry:b@1",
            &["app", "agent"],
            "Worker",
            DefinitionKind::Agent,
        ),
        definition(
            "registry:a@1",
            &["other", "agent"],
            "Worker",
            DefinitionKind::Agent,
        ),
        definition(
            "registry:a@1",
            &["app", "agent"],
            "Other",
            DefinitionKind::Agent,
        ),
        definition(
            "registry:a@1",
            &["app", "agent"],
            "Worker",
            DefinitionKind::Flow,
        ),
    ] {
        assert_ne!(baseline, other);
        assert_ne!(baseline.fingerprint(), other.fingerprint());
    }
}

#[test]
fn implementation_header_is_structured_order_independent_and_body_independent() {
    let left = ImplementationSpecIdentity {
        definition: definition("contracts", &["app"], "Left", DefinitionKind::Spec),
        arguments: vec![fingerprint("type:i32")],
    };
    let right = ImplementationSpecIdentity {
        definition: definition("contracts", &["app"], "Right", DefinitionKind::Spec),
        arguments: Vec::new(),
    };
    let a = ImplementationHeader::new(vec![left.clone(), right.clone()]).unwrap();
    let b = ImplementationHeader::new(vec![right, left.clone(), left.clone()]).unwrap();
    assert_eq!(a.fingerprint(), b.fingerprint());
    let changed = ImplementationHeader::new(vec![ImplementationSpecIdentity {
        arguments: vec![fingerprint("type:string")],
        ..left
    }])
    .unwrap();
    assert_ne!(a.fingerprint(), changed.fingerprint());
    let invalid = ImplementationSpecIdentity {
        definition: definition("app", &[], "NotASpec", DefinitionKind::Type),
        arguments: Vec::new(),
    };
    assert!(ImplementationHeader::new(vec![invalid]).is_err());
}

#[test]
fn canonical_encoding_preserves_segment_boundaries_and_unicode() {
    let a = definition("origin", &["a.b", "c"], "工作", DefinitionKind::Flow);
    let b = definition("origin", &["a", "b.c"], "工作", DefinitionKind::Flow);
    let c = definition("origin", &["a", "b", "c"], "工作", DefinitionKind::Flow);
    assert_ne!(a.fingerprint(), b.fingerprint());
    assert_ne!(a.fingerprint(), c.fingerprint());
    let decoded: DefinitionKey = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
    assert_eq!(a, decoded);
    assert_eq!(a.fingerprint(), decoded.fingerprint());
}

#[test]
fn spec_and_implementation_ownership_distinguish_same_named_methods() {
    let subject = definition("base", &["app"], "Worker", DefinitionKind::Agent);
    let left = definition("spec-a", &["contract"], "Stage", DefinitionKind::Spec);
    let right = definition("spec-b", &["contract"], "Stage", DefinitionKind::Spec);
    let make = |origin: &str, spec| {
        MethodId::new(
            ImplementationKey::new(
                package(origin),
                vec!["extensions".into()],
                subject.clone(),
                spec,
                fingerprint("impl header"),
            )
            .unwrap(),
            "run".into(),
        )
        .unwrap()
    };
    let a = make("extension-a", Some(left.clone()));
    for other in [
        make("extension-a", Some(right)),
        make("extension-b", Some(left)),
        make("extension-a", None),
    ] {
        assert_ne!(a, other);
        assert_ne!(a.fingerprint(), other.fingerprint());
    }
    let decoded: MethodId = serde_json::from_value(serde_json::to_value(&a).unwrap()).unwrap();
    assert_eq!(a, decoded);
}

#[test]
fn wire_identity_validation_does_not_bypass_constructors() {
    let key = definition("origin", &["app"], "Worker", DefinitionKind::Agent);
    let valid = serde_json::to_value(key).unwrap();
    for field in ["module", "declaration"] {
        let mut invalid = valid.clone();
        invalid[field] = serde_json::json!([""]);
        assert!(serde_json::from_value::<DefinitionKey>(invalid).is_err());
    }
    let mut empty = valid.clone();
    empty["declaration"] = serde_json::json!([]);
    assert!(serde_json::from_value::<DefinitionKey>(empty).is_err());
    let mut future = valid.clone();
    future["schema"] = serde_json::json!(999);
    assert!(serde_json::from_value::<DefinitionKey>(future).is_err());
    let mut unknown = valid;
    unknown["authority"] = serde_json::json!("grant");
    assert!(serde_json::from_value::<DefinitionKey>(unknown).is_err());
    let not_spec = definition("origin", &["app"], "Flow", DefinitionKind::Flow);
    let subject = definition("origin", &["app"], "Worker", DefinitionKind::Agent);
    assert!(
        ImplementationKey::new(
            package("origin"),
            vec![],
            subject,
            Some(not_spec),
            fingerprint("header")
        )
        .is_err()
    );
}

#[test]
fn noncallable_declarations_are_rejected_also_when_decoded() {
    let key = definition("origin", &["app"], "Data", DefinitionKind::Type);
    assert!(CallableId::declaration(key.clone()).is_err());
    assert!(serde_json::from_value::<CallableId>(serde_json::json!({"Declaration": key})).is_err());
}

fn inputs() -> RevisionInputs {
    RevisionInputs {
        definition: CallableId::declaration(definition(
            "origin",
            &["app"],
            "Worker",
            DefinitionKind::Agent,
        ))
        .unwrap(),
        content: fingerprint("body"),
        environment: fingerprint("environment"),
        contract: fingerprint("contract"),
        compilation: fingerprint("compiler/backend/schema/options"),
        parent: None,
    }
}

#[test]
fn every_compile_input_and_parent_participates_in_revision_identity() {
    let base = inputs();
    let key = RevisionKey::from_inputs(&base);
    let mut variants = vec![];
    let mut changed = base.clone();
    changed.content = fingerprint("new body");
    variants.push(changed);
    let mut changed = base.clone();
    changed.environment = fingerprint("new dependencies");
    variants.push(changed);
    let mut changed = base.clone();
    changed.contract = fingerprint("new contract");
    variants.push(changed);
    let mut changed = base.clone();
    changed.compilation = fingerprint("new compiler");
    variants.push(changed);
    let mut changed = base.clone();
    changed.parent = Some(key);
    variants.push(changed);
    for variant in variants {
        assert_ne!(key, RevisionKey::from_inputs(&variant));
        assert_eq!(variant.definition, base.definition);
    }
    assert_eq!(key, RevisionKey::from_inputs(&base.clone()));
}

#[test]
fn copying_a_definition_or_revision_shares_its_identity_storage() {
    let base = RevisionInfo::new(inputs());
    let key = base.inputs().definition.as_declaration().unwrap();
    let one = MemoryFootprint::of(key);
    let many = vec![key.clone(); 100];
    let total = MemoryFootprint::of(&many);
    assert_eq!(total.allocations.len(), one.allocations.len() + 1);
    assert_eq!(
        total.estimated_bytes() - one.estimated_bytes(),
        (many.capacity() * std::mem::size_of::<DefinitionKey>()) as u64
    );
    let before = base.key();
    let mut edit = base.inputs().clone();
    edit.content = fingerprint("replacement");
    edit.parent = Some(before);
    let edited = RevisionInfo::new(edit);
    assert_eq!(base.key(), before);
    assert_ne!(base.key(), edited.key());
    assert_eq!(base.inputs().definition, edited.inputs().definition);
    assert_eq!(edited.inputs().parent, Some(base.key()));
}

#[test]
fn canonical_identity_digest_has_a_fixed_versioned_vector() {
    let definition = DefinitionKey::new(
        PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([7; 32])),
        vec!["app".into()],
        vec!["Worker".into()],
        DefinitionKind::Agent,
    )
    .unwrap();
    let revision = RevisionInfo::new(RevisionInputs {
        definition: CallableId::declaration(definition.clone()).unwrap(),
        content: CodeFingerprint::from_bytes([1; 32]),
        environment: CodeFingerprint::from_bytes([2; 32]),
        contract: CodeFingerprint::from_bytes([3; 32]),
        compilation: CodeFingerprint::from_bytes([4; 32]),
        parent: None,
    });
    assert_eq!(
        (
            definition.fingerprint().to_string(),
            revision.key().fingerprint().to_string()
        ),
        (
            "75066858decb16a953f972a4a94eaedfb5a45355aa886ed2db2667408234f5ba".into(),
            "0d96b21abfbf5f1d18b31de16f7402869c9a2add027e3a56da85e9ae08a6a279".into(),
        )
    );
}
