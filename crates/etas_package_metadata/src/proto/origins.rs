use etas_code::identity::{
    DefinitionBindingOrigin, DefinitionModuleOrigin, PackageDefinitionOrigins,
};
use etas_code::{CodeFingerprint, DefinitionKey, DefinitionKind, PackageKey};

use super::schema::*;
use crate::{MetadataArtifactError, PACKAGE_METADATA_FILE};

pub(super) fn to_proto(origins: &PackageDefinitionOrigins) -> ProtoDefinitionOrigins {
    ProtoDefinitionOrigins {
        version: 1,
        package: origins.package.fingerprint().as_bytes().to_vec(),
        modules: origins
            .modules
            .iter()
            .map(|module| ProtoDefinitionModuleOrigin {
                path: module.path.clone(),
                canonical_path: module.canonical_path.clone(),
            })
            .collect(),
        declarations: origins
            .declarations
            .iter()
            .map(|binding| ProtoDefinitionBindingOrigin {
                module: binding.module.clone(),
                declaration: binding.declaration.clone(),
                definition: Some(ProtoDefinitionKey {
                    package: binding
                        .definition
                        .package()
                        .fingerprint()
                        .as_bytes()
                        .to_vec(),
                    module: binding.definition.module().to_vec(),
                    declaration: binding.definition.declaration().to_vec(),
                    kind: kind_to_proto(binding.definition.kind()),
                }),
            })
            .collect(),
    }
}

pub(super) fn from_proto(
    origins: ProtoDefinitionOrigins,
) -> Result<PackageDefinitionOrigins, MetadataArtifactError> {
    if origins.version != 1 {
        return Err(invalid("unsupported definition origin schema"));
    }
    let result = PackageDefinitionOrigins {
        package: package_key(origins.package)?,
        modules: origins
            .modules
            .into_iter()
            .map(|module| DefinitionModuleOrigin {
                path: module.path,
                canonical_path: module.canonical_path,
            })
            .collect(),
        declarations: origins
            .declarations
            .into_iter()
            .map(|binding| {
                let key = binding.definition.ok_or_else(|| {
                    invalid("definition binding is missing its canonical identity")
                })?;
                Ok(DefinitionBindingOrigin {
                    module: binding.module,
                    declaration: binding.declaration,
                    definition: DefinitionKey::new(
                        package_key(key.package)?,
                        key.module,
                        key.declaration,
                        kind_from_proto(key.kind)?,
                    )
                    .map_err(|error| invalid(error.to_string()))?,
                })
            })
            .collect::<Result<_, MetadataArtifactError>>()?,
    };
    result
        .validate()
        .map_err(|error| invalid(error.to_string()))?;
    Ok(result)
}

pub(super) fn package_key(bytes: Vec<u8>) -> Result<PackageKey, MetadataArtifactError> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| invalid("definition package identity must contain 32 bytes"))?;
    Ok(PackageKey::from_canonical_fingerprint(
        CodeFingerprint::from_bytes(bytes),
    ))
}

pub(super) fn kind_to_proto(kind: DefinitionKind) -> u32 {
    match kind {
        DefinitionKind::Agent => 1,
        DefinitionKind::Flow => 2,
        DefinitionKind::Tool => 3,
        DefinitionKind::Type => 4,
        DefinitionKind::Effect => 5,
        DefinitionKind::Spec => 6,
        DefinitionKind::Value => 7,
        DefinitionKind::EnumVariant => 8,
        DefinitionKind::Requirement => 9,
        DefinitionKind::Protocol => 10,
    }
}

pub(super) fn kind_from_proto(kind: u32) -> Result<DefinitionKind, MetadataArtifactError> {
    Ok(match kind {
        1 => DefinitionKind::Agent,
        2 => DefinitionKind::Flow,
        3 => DefinitionKind::Tool,
        4 => DefinitionKind::Type,
        5 => DefinitionKind::Effect,
        6 => DefinitionKind::Spec,
        7 => DefinitionKind::Value,
        8 => DefinitionKind::EnumVariant,
        9 => DefinitionKind::Requirement,
        10 => DefinitionKind::Protocol,
        _ => return Err(invalid(format!("unknown definition kind {kind}"))),
    })
}

fn invalid(reason: impl Into<String>) -> MetadataArtifactError {
    MetadataArtifactError::invalid(PACKAGE_METADATA_FILE, reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    fn fixture(count: usize) -> PackageDefinitionOrigins {
        let package = PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([3; 32]));
        PackageDefinitionOrigins {
            package,
            modules: vec![DefinitionModuleOrigin {
                path: vec!["api".into()],
                canonical_path: vec!["api".into()],
            }],
            declarations: (0..count)
                .map(|i| {
                    let declaration = vec![format!("flow_{i}")];
                    DefinitionBindingOrigin {
                        module: vec!["api".into()],
                        declaration: declaration.clone(),
                        definition: DefinitionKey::new(
                            package,
                            vec!["api".into()],
                            declaration,
                            DefinitionKind::Flow,
                        )
                        .unwrap(),
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn origin_protobuf_roundtrip_is_typed_and_rejects_bad_identity_fields() {
        let original = fixture(1);
        let wire = to_proto(&original);
        assert_eq!(
            from_proto(ProtoDefinitionOrigins::decode(wire.encode_to_vec().as_slice()).unwrap())
                .unwrap(),
            original
        );
        for case in 0..6 {
            let mut wire = to_proto(&original);
            match case {
                0 => wire.version = 99,
                1 => wire.package.pop().map(|_| ()).unwrap(),
                2 => wire.declarations[0].definition = None,
                3 => wire.declarations[0].definition.as_mut().unwrap().kind = 999,
                4 => wire.declarations.push(wire.declarations[0].clone()),
                5 => wire.declarations[0]
                    .definition
                    .as_mut()
                    .unwrap()
                    .declaration
                    .clear(),
                _ => unreachable!(),
            }
            assert!(from_proto(wire).is_err(), "case {case}");
        }
    }

    #[test]
    fn origin_budget_is_shared_with_the_entire_package_graph_before_encoding() {
        let origins = fixture(9_000);
        let mut metadata = crate::PackageMetadata::default();
        metadata.public_metadata.definition_origins = Some(origins.clone());
        metadata.dependencies.push(crate::ResolvedDependency {
            identity: Default::default(),
            import_root: "dep".into(),
            source: crate::ResolvedDependencySource::Path {
                path: "dep".into(),
                checksum: "locked".into(),
            },
            dependencies: Vec::new(),
            public_metadata: crate::PublicMetadata {
                definition_origins: Some(origins),
                ..Default::default()
            },
            effect_metadata: Default::default(),
            tool_bindings: Vec::new(),
        });
        let error = match crate::package_metadata_to_sections(&metadata) {
            Err(error) => error,
            Ok(_) => {
                panic!("combined definition graphs exceeded the shared budget but were encoded")
            }
        };
        assert!(error.to_string().contains("maximum node count"), "{error}");
    }
}
