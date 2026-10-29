pub(super) mod schema;

use etas_code::{
    CodeFingerprint, PackageKey, RevisionInputs, RevisionKey,
    descriptor::{DefinitionSnapshotInfo, DefinitionSnapshots},
    report::{DefinitionBindingInfo, DependencyArtifactInfo, DependencyInfo, ModuleOriginInfo},
};

use super::{identities::*, invalid, required};
use crate::MetadataArtifactError as Error;

pub(super) fn encode(snapshots: &DefinitionSnapshots) -> schema::Snapshots {
    let encode = |snapshot: &DefinitionSnapshotInfo| {
        let revision = &snapshot.revision;
        let dependencies = &snapshot.dependencies;
        schema::DefinitionSnapshot {
            identity: Some(encode_key(&snapshot.identity)),
            revision: Some(schema::Revision {
                definition: Some(encode_callable(&revision.definition)),
                content: bytes(revision.content),
                environment: bytes(revision.environment),
                contract: bytes(revision.contract),
                compilation: bytes(revision.compilation),
                parent: revision.parent.map(|parent| bytes(parent.fingerprint())),
            }),
            dependencies: Some(schema::Dependencies {
                environment: bytes(dependencies.environment),
                lexical_owner: Some(encode_key(&dependencies.lexical_owner)),
                standard_package: bytes(dependencies.standard_package.fingerprint()),
                modules: dependencies
                    .modules
                    .iter()
                    .map(|module| schema::Module {
                        path: module.module.clone(),
                        package: bytes(module.package.fingerprint()),
                    })
                    .collect(),
                bindings: dependencies
                    .bindings
                    .iter()
                    .map(|binding| schema::Binding {
                        path: binding.path.clone(),
                        definition: Some(encode_key(&binding.definition)),
                    })
                    .collect(),
                artifacts: dependencies
                    .artifacts
                    .iter()
                    .map(|artifact| schema::Artifact {
                        import_root: artifact.import_root.clone(),
                        fingerprint: artifact.fingerprint.clone(),
                    })
                    .collect(),
            }),
        }
    };
    schema::Snapshots {
        agents: snapshots.agents.iter().map(&encode).collect(),
        flows: snapshots.flows.iter().map(&encode).collect(),
        methods: snapshots.methods.iter().map(encode).collect(),
    }
}

pub(super) fn decode(snapshots: schema::Snapshots) -> Result<DefinitionSnapshots, Error> {
    let decode = |snapshot: schema::DefinitionSnapshot| {
        let revision = required(snapshot.revision, "definition revision inputs")?;
        let dependencies = required(snapshot.dependencies, "definition dependencies")?;
        Ok(DefinitionSnapshotInfo {
            identity: decode_key(required(snapshot.identity, "definition snapshot identity")?)?,
            revision: RevisionInputs {
                definition: decode_callable(required(revision.definition, "revision definition")?)?,
                content: fingerprint(revision.content)?,
                environment: fingerprint(revision.environment)?,
                contract: fingerprint(revision.contract)?,
                compilation: fingerprint(revision.compilation)?,
                parent: revision
                    .parent
                    .map(fingerprint)
                    .transpose()?
                    .map(RevisionKey::from_canonical_fingerprint),
            },
            dependencies: DependencyInfo {
                environment: fingerprint(dependencies.environment)?,
                lexical_owner: decode_key(required(
                    dependencies.lexical_owner,
                    "snapshot lexical owner",
                )?)?,
                standard_package: PackageKey::from_canonical_fingerprint(fingerprint(
                    dependencies.standard_package,
                )?),
                modules: dependencies
                    .modules
                    .into_iter()
                    .map(|module| {
                        Ok(ModuleOriginInfo {
                            module: module.path,
                            package: PackageKey::from_canonical_fingerprint(fingerprint(
                                module.package,
                            )?),
                        })
                    })
                    .collect::<Result<_, Error>>()?,
                bindings: dependencies
                    .bindings
                    .into_iter()
                    .map(|binding| {
                        Ok(DefinitionBindingInfo {
                            path: binding.path,
                            definition: decode_key(required(
                                binding.definition,
                                "snapshot binding identity",
                            )?)?,
                        })
                    })
                    .collect::<Result<_, Error>>()?,
                artifacts: dependencies
                    .artifacts
                    .into_iter()
                    .map(|artifact| DependencyArtifactInfo {
                        import_root: artifact.import_root,
                        fingerprint: artifact.fingerprint,
                    })
                    .collect(),
            },
        })
    };
    Ok(DefinitionSnapshots {
        agents: snapshots
            .agents
            .into_iter()
            .map(&decode)
            .collect::<Result<_, Error>>()?,
        flows: snapshots
            .flows
            .into_iter()
            .map(&decode)
            .collect::<Result<_, Error>>()?,
        methods: snapshots
            .methods
            .into_iter()
            .map(decode)
            .collect::<Result<_, Error>>()?,
    })
}

pub(super) fn validate_wire(
    snapshots: &schema::Snapshots,
    budget: &mut super::super::budget::MetadataGraphBudget,
) -> Result<(), Error> {
    budget.charge(
        snapshots
            .agents
            .len()
            .saturating_add(snapshots.flows.len())
            .saturating_add(snapshots.methods.len()),
        "definition snapshots",
    )?;
    for snapshot in snapshots
        .agents
        .iter()
        .chain(&snapshots.flows)
        .chain(&snapshots.methods)
    {
        super::header_budget::key(snapshot.identity.as_ref(), budget)?;
        if let Some(revision) = &snapshot.revision {
            if let Some(definition) = &revision.definition {
                super::header_budget::callable(definition, budget)?;
            }
            budget.charge(6, "revision coordinates")?;
        }
        if let Some(dependencies) = &snapshot.dependencies {
            super::header_budget::key(dependencies.lexical_owner.as_ref(), budget)?;
            budget.charge(dependencies.modules.len(), "snapshot modules")?;
            for module in &dependencies.modules {
                budget.charge(module.path.len(), "snapshot module path")?;
            }
            budget.charge(dependencies.bindings.len(), "snapshot bindings")?;
            for binding in &dependencies.bindings {
                budget.charge(binding.path.len(), "snapshot binding path")?;
                super::header_budget::key(binding.definition.as_ref(), budget)?;
            }
            budget.charge(dependencies.artifacts.len(), "snapshot artifacts")?;
        }
    }
    Ok(())
}

fn bytes(fingerprint: CodeFingerprint) -> Vec<u8> {
    fingerprint.as_bytes().to_vec()
}

fn fingerprint(bytes: Vec<u8>) -> Result<CodeFingerprint, Error> {
    bytes
        .try_into()
        .map(CodeFingerprint::from_bytes)
        .map_err(|_| invalid("snapshot fingerprint must contain exactly 32 bytes"))
}
