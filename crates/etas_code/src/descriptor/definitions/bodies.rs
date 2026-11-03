use std::collections::{BTreeMap, BTreeSet};

use super::{super::DescriptorError, DefinitionHeaders};
use crate::{CallableId, syntax::RetainedBodySource};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum BodyAccess {
    Package,
    Exported,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct CallableBodyInfo {
    pub callable: CallableId,
    pub access: BodyAccess,
    pub source: RetainedBodySource,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DefinitionBodies {
    pub entries: Vec<CallableBodyInfo>,
}

impl DefinitionBodies {
    pub fn validate_origins(
        &self,
        origins: &crate::identity::PackageDefinitionOrigins,
    ) -> Result<(), DescriptorError> {
        let modules = origins
            .modules
            .iter()
            .map(|module| module.canonical_path.as_slice())
            .collect::<BTreeSet<_>>();
        for body in &self.entries {
            let (package, module) = match body.callable.identity_ref() {
                crate::identity::CallableIdentityRef::Declaration(key) => {
                    (key.package(), key.module())
                }
                crate::identity::CallableIdentityRef::Method(method) => (
                    method.implementation().package(),
                    method.implementation().module(),
                ),
            };
            if package != origins.package || !modules.contains(module) {
                return Err(invalid(
                    "retained body does not belong to its package module origin",
                ));
            }
        }
        Ok(())
    }
}

pub(super) fn validate(
    bodies: &DefinitionBodies,
    headers: &DefinitionHeaders,
) -> Result<(), DescriptorError> {
    let public = headers
        .callables
        .iter()
        .map(|header| &header.id)
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeMap::new();
    for body in &bodies.entries {
        body.source.validate().map_err(invalid)?;
        if seen.insert(&body.callable, body).is_some() {
            return Err(invalid("duplicate retained callable body identity"));
        }
        if body.access == BodyAccess::Exported && !public.contains(&body.callable) {
            return Err(invalid("exported body has no public callable header"));
        }
        if body.callable.as_declaration().is_some_and(|key| {
            !matches!(
                key.kind(),
                crate::DefinitionKind::Flow | crate::DefinitionKind::Tool
            )
        }) {
            return Err(invalid(
                "retained body does not identify a callable implementation",
            ));
        }
    }
    if let Some(snapshots) = &headers.snapshots {
        for snapshot in snapshots
            .agents
            .iter()
            .chain(&snapshots.flows)
            .chain(&snapshots.methods)
        {
            if let Some(body) = seen.get(&snapshot.revision.definition)
                && body.source.content != snapshot.revision.content
            {
                return Err(invalid("retained body disagrees with its revision"));
            }
        }
    }
    Ok(())
}

fn invalid(reason: &'static str) -> DescriptorError {
    DescriptorError::InvalidDescription { reason }
}
