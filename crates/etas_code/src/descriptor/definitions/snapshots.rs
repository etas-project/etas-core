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
    pub agents: Vec<AgentSnapshotInfo>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct AgentSnapshotInfo {
    pub identity: DefinitionKey,
    pub revision: RevisionInputs,
    pub dependencies: DependencyInfo,
}

impl AgentSnapshotInfo {
    pub fn revision_info(&self) -> RevisionInfo {
        RevisionInfo::new(self.revision.clone())
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
    for snapshot in &snapshots.agents {
        if !seen.insert(&snapshot.identity) {
            return Err(invalid("duplicate agent snapshot identity"));
        }
        let header = agents
            .get(&snapshot.identity)
            .ok_or(invalid("agent snapshot has no definition header"))?;
        let definition_matches = match &header.default_entry {
            Some(default) => snapshot.revision.definition == *default,
            None => snapshot.revision.definition.as_declaration() == Some(&header.identity),
        };
        if !definition_matches
            || snapshot.dependencies.lexical_owner != header.identity
            || snapshot.dependencies.environment != snapshot.revision.environment
        {
            return Err(invalid(
                "agent snapshot disagrees with its definition or environment",
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
            module.module == header.identity.module() && module.package == header.identity.package()
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
    }
    if seen.len() != headers.agents.len() {
        return Err(invalid("definition snapshots omit an agent header"));
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
