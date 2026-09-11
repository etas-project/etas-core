use super::{WorkspacePath, WorkspacePathRef, WorkspaceRegionId, WorkspaceRoot};
use crate::{HostError, HostErrorCode};
use std::collections::{BTreeMap, btree_map::Entry};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceRegionRegistry {
    roots: BTreeMap<WorkspaceRegionId, WorkspaceRoot>,
}

impl WorkspaceRegionRegistry {
    pub fn insert(
        &mut self,
        region: WorkspaceRegionId,
        root: WorkspaceRoot,
    ) -> Result<(), HostError> {
        if let Entry::Vacant(entry) = self.roots.entry(region.clone()) {
            entry.insert(root);
        } else {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace region is configured more than once",
            )
            .with_detail("region", region.as_str()));
        }
        Ok(())
    }

    pub(crate) fn bind(&self, path: &WorkspacePathRef) -> Result<WorkspacePath, HostError> {
        let root = self.roots.get(&path.region).ok_or_else(|| {
            HostError::new(
                HostErrorCode::AuthorityDenied,
                "workspace region is not configured",
            )
            .with_detail("region", path.region.as_str())
        })?;
        Ok(WorkspacePath {
            root: root.clone(),
            relative: crate::sandbox::workspace::normalize_relative(&path.relative)?,
        })
    }
}
