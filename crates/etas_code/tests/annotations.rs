use etas_code::{CodeFingerprint, DefinitionKey, DefinitionKind, PackageKey, descriptor::*};
use std::sync::Arc;

fn info(value: StaticValueInfo) -> AnnotationInfo {
    AnnotationInfo {
        path: vec!["custom".into()],
        arguments: vec![AnnotationArgumentInfo {
            name: None,
            value: Arc::new(value),
        }],
    }
}

#[test]
fn annotation_fingerprints_distinguish_literal_kinds_and_argument_names() {
    let values = [
        AnnotationLiteralInfo::Bool(true),
        AnnotationLiteralInfo::String("true".into()),
        AnnotationLiteralInfo::Integer("31".into()),
        AnnotationLiteralInfo::Float("31".into()),
    ]
    .into_iter()
    .map(|value| info(StaticValueInfo::Literal(value)))
    .collect::<Vec<_>>();
    for (index, left) in values.iter().enumerate() {
        for right in &values[index + 1..] {
            assert_ne!(left.fingerprint(), right.fingerprint());
        }
    }
    let mut named = values[0].clone();
    named.arguments[0].name = Some("enabled".into());
    assert_ne!(named.fingerprint(), values[0].fingerprint());
}

#[test]
fn annotation_references_include_canonical_package_and_declaration_kind() {
    let package =
        |tag| PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([tag; 32]));
    let identity = |tag, kind| {
        DefinitionKey::new(
            package(tag),
            vec!["app".into()],
            vec!["Marker".into()],
            kind,
        )
        .unwrap()
    };
    let make = |key| {
        info(StaticValueInfo::Reference(
            AnnotationReferenceInfo::Declaration(key),
        ))
    };
    let baseline = make(identity(1, DefinitionKind::Value)).fingerprint();
    assert_ne!(
        baseline,
        make(identity(2, DefinitionKind::Value)).fingerprint()
    );
    assert_ne!(
        baseline,
        make(identity(1, DefinitionKind::Requirement)).fingerprint()
    );
    let module = info(StaticValueInfo::Reference(
        AnnotationReferenceInfo::Module {
            package: package(1),
            path: vec!["app".into(), "Marker".into()],
        },
    ));
    assert_ne!(baseline, module.fingerprint());
}

#[test]
fn annotation_clones_share_static_values_and_preserve_constructor_order() {
    let target = AnnotationReferenceInfo::Declaration(
        DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([1; 32])),
            vec!["std".into(), "runtime".into(), "limits".into()],
            vec!["Tokens".into()],
            DefinitionKind::Requirement,
        )
        .unwrap(),
    );
    let first = AnnotationArgumentInfo {
        name: None,
        value: Arc::new(StaticValueInfo::Literal(AnnotationLiteralInfo::Integer(
            "1".into(),
        ))),
    };
    let second = AnnotationArgumentInfo {
        name: None,
        value: Arc::new(StaticValueInfo::Literal(AnnotationLiteralInfo::Integer(
            "2".into(),
        ))),
    };
    let a = info(StaticValueInfo::Constructor {
        kind: AnnotationConstructorKind::Limit,
        target: target.clone(),
        arguments: vec![first.clone(), second.clone()],
    });
    let b = info(StaticValueInfo::Constructor {
        kind: AnnotationConstructorKind::Limit,
        target,
        arguments: vec![second, first],
    });
    assert_ne!(a.fingerprint(), b.fingerprint());
    for _ in 0..100 {
        let clone = a.clone();
        assert!(Arc::ptr_eq(
            &a.arguments[0].value,
            &clone.arguments[0].value
        ));
        assert_eq!(a.fingerprint(), clone.fingerprint());
    }
}
