use super::*;
use etas_code::{ImplementationHeader, ImplementationKey, MethodId};

#[test]
fn implementation_and_method_parameters_have_independent_identities() {
    let subject = key("Container", DefinitionKind::Type);
    let implementation = ImplementationKey::from_header(
        subject.package(),
        vec!["app".into()],
        subject,
        None,
        &ImplementationHeader::new(vec![]).unwrap(),
    )
    .unwrap();
    let method = MethodId::new(implementation.clone(), "replace".into()).unwrap();
    let data = DescriptorGraphInput {
        types: vec![
            TypeInfo::Parameter(GenericParameterRef(0)),
            TypeInfo::Parameter(GenericParameterRef(1)),
        ],
        binders: [
            GenericOwner::Implementation(implementation),
            GenericOwner::Method(method),
        ]
        .into_iter()
        .map(|owner| GenericParameterInfo {
            owner,
            position: 0,
            name: "T".into(),
            kind: GenericParameterKind::Type,
            bounds: vec![],
        })
        .collect(),
        ..Default::default()
    };
    let graph = graph(data.clone());
    assert_ne!(
        graph.type_fingerprint(TypeInfoRef(0)).unwrap(),
        graph.type_fingerprint(TypeInfoRef(1)).unwrap()
    );
    let mut renamed = data;
    let mut alias = renamed.binders[0].clone();
    alias.name = "Value".into();
    renamed.binders.push(alias);
    renamed
        .types
        .push(TypeInfo::Parameter(GenericParameterRef(2)));
    let merged =
        DescriptorGraph::new_with_merged_binders(renamed, DescriptorLimits::default()).unwrap();
    assert_eq!(merged.data().binders.len(), 2);
    assert_eq!(
        merged.type_fingerprint(TypeInfoRef(0)).unwrap(),
        merged.type_fingerprint(TypeInfoRef(2)).unwrap()
    );
}

#[test]
fn implementation_pattern_fingerprint_is_alpha_equivalent() {
    let pattern = |name: &str, position| {
        graph(DescriptorGraphInput {
            types: vec![TypeInfo::Parameter(GenericParameterRef(0))],
            binders: vec![GenericParameterInfo {
                owner: GenericOwner::ImplementationPattern(key("Container", DefinitionKind::Type)),
                position,
                name: name.into(),
                kind: GenericParameterKind::Type,
                bounds: vec![],
            }],
            ..Default::default()
        })
        .type_fingerprint(TypeInfoRef(0))
        .unwrap()
    };
    assert_eq!(pattern("T", 0), pattern("Value", 0));
    assert_ne!(pattern("T", 0), pattern("T", 1));
}
