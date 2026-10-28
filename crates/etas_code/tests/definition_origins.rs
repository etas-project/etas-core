use etas_code::{CodeFingerprint, DefinitionKey, DefinitionKind, PackageKey, identity::*};

fn origins() -> PackageDefinitionOrigins {
    let package = PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([3; 32]));
    PackageDefinitionOrigins {
        package,
        modules: vec![DefinitionModuleOrigin {
            path: vec!["alias".into(), "api".into()],
            canonical_path: vec!["owner".into(), "api".into()],
        }],
        declarations: vec![DefinitionBindingOrigin {
            module: vec!["alias".into(), "api".into()],
            declaration: vec!["lookup".into()],
            definition: DefinitionKey::new(
                package,
                vec!["owner".into(), "api".into()],
                vec!["lookup".into()],
                DefinitionKind::Flow,
            )
            .unwrap(),
        }],
    }
}

#[test]
fn origin_lookup_paths_do_not_rewrite_canonical_declaration_identity() {
    let first = origins();
    first.validate().unwrap();
    let mut second = first.clone();
    second.modules[0].path[0] = "second".into();
    second.declarations[0].module[0] = "second".into();
    second.validate().unwrap();
    assert_ne!(first.fingerprint(), second.fingerprint());
    assert_eq!(
        first.declarations[0].definition,
        second.declarations[0].definition
    );
    let mut renamed = first.clone();
    renamed.modules[0].canonical_path[0] = "other".into();
    assert_ne!(first.fingerprint(), renamed.fingerprint());
}

#[test]
fn origins_reject_duplicate_and_unbound_lookup_records() {
    for case in 0..4 {
        let mut value = origins();
        match case {
            0 => value.modules.push(value.modules[0].clone()),
            1 => value.declarations.push(value.declarations[0].clone()),
            2 => value.declarations[0].module = vec!["unbound".into()],
            3 => value.declarations[0].declaration.clear(),
            _ => unreachable!(),
        }
        assert!(value.validate().is_err(), "case {case}");
    }
}
