use std::collections::{BTreeMap, BTreeSet};

use crate::{DefinitionKey, RevisionInfo, RevisionInputs, report::DependencyInfo};

use super::{super::DescriptorError, DefinitionHeaders};

/// Producer snapshot coordinates. Neither these hashes nor saved analysis
/// authorize execution or reconstruct a native lexical environment.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DefinitionSnapshots {
    pub agents: Vec<DefinitionSnapshotInfo>,
    pub flows: Vec<DefinitionSnapshotInfo>,
    pub methods: Vec<DefinitionSnapshotInfo>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DefinitionSnapshotInfo {
    pub identity: DefinitionKey,
    pub revision: RevisionInputs,
    pub dependencies: DependencyInfo,
}

impl DefinitionSnapshotInfo {
    pub fn revision_info(&self) -> RevisionInfo {
        RevisionInfo::new(self.revision.clone())
    }

    /// Descriptive coordinates, not independently editable method evidence.
    pub fn method_snapshot(
        &self,
        callable: &crate::CallableId,
        content: crate::CodeFingerprint,
    ) -> Result<Self, DescriptorError> {
        let method = callable
            .as_method()
            .ok_or(invalid("method snapshot has no method identity"))?;
        if self.identity.kind() != crate::DefinitionKind::Agent
            || method.implementation().subject() != &self.identity
        {
            return Err(invalid(
                "method snapshot belongs to a different nominal owner",
            ));
        }
        if callable == &self.revision.definition {
            if content != self.revision.content {
                return Err(invalid(
                    "default method body differs from its owning revision",
                ));
            }
            return Ok(self.clone());
        }
        let key = self.revision_info().key();
        let environment = crate::CodeFingerprint::of_parts(
            "etas.code.method-revision-environment.v1",
            [
                self.revision.environment.as_bytes().as_slice(),
                key.fingerprint().as_bytes().as_slice(),
            ],
        );
        let mut dependencies = self.dependencies.clone();
        dependencies.environment = environment;
        Ok(Self {
            identity: self.identity.clone(),
            revision: RevisionInputs {
                definition: callable.clone(),
                content,
                environment,
                contract: self.revision.contract,
                compilation: self.revision.compilation,
                // The API revises the enclosing default body, not this method.
                parent: None,
            },
            dependencies,
        })
    }
}

pub(super) fn validate(
    snapshots: &DefinitionSnapshots,
    headers: &DefinitionHeaders,
) -> Result<(), DescriptorError> {
    let mut seen = BTreeSet::new();
    let agents = headers
        .agents
        .iter()
        .map(|header| (&header.identity, header))
        .collect::<BTreeMap<_, _>>();
    let flows = headers
        .callables
        .iter()
        .filter_map(|header| {
            header
                .id
                .as_declaration()
                .filter(|identity| identity.kind() == crate::DefinitionKind::Flow)
                .map(|identity| (identity, header))
        })
        .collect::<BTreeMap<_, _>>();
    for (kind, snapshot) in snapshots
        .agents
        .iter()
        .map(|snapshot| (crate::DefinitionKind::Agent, snapshot))
        .chain(
            snapshots
                .flows
                .iter()
                .map(|snapshot| (crate::DefinitionKind::Flow, snapshot)),
        )
    {
        if snapshot.identity.kind() != kind {
            return Err(invalid("snapshot has the wrong definition kind"));
        }
        if !seen.insert(&snapshot.identity) {
            return Err(invalid("duplicate definition snapshot identity"));
        }
        let definition_matches = if kind == crate::DefinitionKind::Agent {
            let header = agents
                .get(&snapshot.identity)
                .ok_or(invalid("agent snapshot has no definition header"))?;
            match &header.default_entry {
                Some(default) => snapshot.revision.definition == *default,
                None => snapshot.revision.definition.as_declaration() == Some(&header.identity),
            }
        } else {
            let header = flows
                .get(&snapshot.identity)
                .ok_or(invalid("flow snapshot has no definition header"))?;
            snapshot.revision.definition == header.id
        };
        if !definition_matches
            || snapshot.dependencies.lexical_owner != snapshot.identity
            || snapshot.dependencies.environment != snapshot.revision.environment
        {
            return Err(invalid(
                "snapshot disagrees with its definition or environment",
            ));
        }
        validate_dependencies(snapshot)?;
    }
    let methods = headers
        .callables
        .iter()
        .filter(|header| header.id.as_method().is_some())
        .map(|header| &header.id)
        .collect::<BTreeSet<_>>();
    let mut seen_methods = BTreeSet::new();
    let agent_snapshots = snapshots
        .agents
        .iter()
        .map(|snapshot| (&snapshot.identity, snapshot))
        .collect::<BTreeMap<_, _>>();
    for snapshot in &snapshots.methods {
        let callable = &snapshot.revision.definition;
        let method = callable
            .as_method()
            .ok_or(invalid("method snapshot has no method identity"))?;
        if method.implementation().subject() != &snapshot.identity
            || !agents.contains_key(&snapshot.identity)
            || !methods.contains(callable)
            || !seen_methods.insert(callable)
        {
            return Err(invalid(
                "method snapshot has an unknown, duplicate or inconsistent owner",
            ));
        }
        validate_dependencies(snapshot)?;
        let owner = agent_snapshots
            .get(&snapshot.identity)
            .ok_or(invalid("method snapshot has no owning revision"))?;
        if owner.method_snapshot(callable, snapshot.revision.content)? != *snapshot {
            return Err(invalid(
                "method coordinates disagree with the owning revision",
            ));
        }
        if !snapshot.dependencies.modules.iter().any(|module| {
            module.package == method.implementation().package()
                && module.module == method.implementation().module()
        }) {
            return Err(invalid(
                "method snapshot is missing its implementation module origin",
            ));
        }
    }
    if !snapshots.methods.is_empty() && seen_methods != methods {
        return Err(invalid("method snapshots omit a callable header"));
    }
    if snapshots.agents.len() != agents.len() || snapshots.flows.len() != flows.len() {
        return Err(invalid("definition snapshots omit an agent or flow header"));
    }
    Ok(())
}

fn validate_dependencies(snapshot: &DefinitionSnapshotInfo) -> Result<(), DescriptorError> {
    if snapshot.dependencies.lexical_owner != snapshot.identity
        || snapshot.dependencies.environment != snapshot.revision.environment
    {
        return Err(invalid(
            "snapshot disagrees with its lexical dependency environment",
        ));
    }
    let mut modules = BTreeSet::new();
    for module in &snapshot.dependencies.modules {
        path(&module.module)?;
        if !modules.insert((module.package, &module.module)) {
            return Err(invalid("duplicate snapshot module origin"));
        }
    }
    if !snapshot.dependencies.modules.iter().any(|module| {
        module.module == snapshot.identity.module() && module.package == snapshot.identity.package()
    }) {
        return Err(invalid(
            "snapshot is missing its lexical owner's module origin",
        ));
    }
    let mut bindings = BTreeSet::new();
    for binding in &snapshot.dependencies.bindings {
        path(&binding.path)?;
        if !bindings.insert(&binding.path) {
            return Err(invalid("duplicate snapshot lexical binding"));
        }
    }
    let mut artifacts = BTreeSet::new();
    for artifact in &snapshot.dependencies.artifacts {
        if artifact
            .import_root
            .split('.')
            .any(|part| part.is_empty() || part.contains('\0'))
            || artifact.fingerprint.len() != 64
            || !artifact
                .fingerprint
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || !artifacts.insert(&artifact.import_root)
        {
            return Err(invalid("invalid or duplicate snapshot dependency artifact"));
        }
    }
    Ok(())
}

fn path(path: &[String]) -> Result<(), DescriptorError> {
    if path.is_empty()
        || path
            .iter()
            .any(|part| part.is_empty() || part.contains('\0'))
    {
        return Err(invalid("invalid snapshot dependency path"));
    }
    Ok(())
}

fn invalid(reason: &'static str) -> DescriptorError {
    DescriptorError::InvalidDescription { reason }
}
