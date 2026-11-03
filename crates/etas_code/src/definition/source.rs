use std::fmt;

use crate::{
    DefinitionKey, DefinitionKind, RevisionInputs, RevisionKey, report::DependencyInfo,
    syntax::RetainedBodySource,
};

pub const DEFINITION_SOURCE_BUNDLE_SCHEMA: u32 = 2;

/// Restorable source data only. Neither the revision nor any stored fingerprint
/// is evidence that the source has been checked, compiled, or authorized.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DefinitionSourceBundle {
    pub schema: u32,
    pub target: DefinitionKey,
    pub revision_key: RevisionKey,
    pub revision: RevisionInputs,
    pub source: RetainedBodySource,
    pub dependencies: DependencyInfo,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceBundleLimits {
    pub max_source_bytes: usize,
    pub max_environment_entries: usize,
    pub max_environment_bytes: u64,
}

impl Default for SourceBundleLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 2 * 1024 * 1024,
            max_environment_entries: 65_536,
            max_environment_bytes: 32 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceBundleError {
    UnsupportedSchema(u32),
    InvalidSource(&'static str),
    TargetMismatch,
    ContentMismatch,
    RevisionMismatch,
    EnvironmentMismatch,
    ContractMismatch,
    ConfigurationMismatch,
    DependencyMismatch,
    ResourceLimit { resource: &'static str, limit: u64 },
}

impl DefinitionSourceBundle {
    pub fn validate(&self, limits: SourceBundleLimits) -> Result<(), SourceBundleError> {
        use SourceBundleError as E;
        if self.schema != DEFINITION_SOURCE_BUNDLE_SCHEMA {
            return Err(E::UnsupportedSchema(self.schema));
        }
        if self.source.text.len() > limits.max_source_bytes {
            return Err(E::ResourceLimit {
                resource: "source bytes",
                limit: limits.max_source_bytes as u64,
            });
        }
        let entries = self
            .dependencies
            .modules
            .len()
            .checked_add(self.dependencies.bindings.len())
            .and_then(|size| size.checked_add(self.dependencies.artifacts.len()));
        if entries.is_none_or(|entries| entries > limits.max_environment_entries) {
            return Err(E::ResourceLimit {
                resource: "environment entries",
                limit: limits.max_environment_entries as u64,
            });
        }
        let bytes = etas_core::memory::MemoryFootprint::of(self).estimated_bytes();
        let metadata_bytes = bytes.saturating_sub(self.source.text.capacity() as u64);
        if metadata_bytes > limits.max_environment_bytes
            || bytes
                > limits
                    .max_environment_bytes
                    .saturating_add(limits.max_source_bytes as u64)
        {
            return Err(E::ResourceLimit {
                resource: "environment bytes",
                limit: limits.max_environment_bytes,
            });
        }
        self.source.validate().map_err(E::InvalidSource)?;
        if RevisionKey::from_inputs(&self.revision) != self.revision_key {
            return Err(E::RevisionMismatch);
        }
        let valid_target = match self.target.kind() {
            DefinitionKind::Agent => self.revision.definition.as_method().is_some_and(|method| {
                method.implementation().subject() == &self.target
                    && method.implementation().selected_spec().is_none()
                    && method.name() == "run"
            }),
            DefinitionKind::Flow => self.revision.definition.as_declaration() == Some(&self.target),
            _ => false,
        };
        if !valid_target || self.dependencies.lexical_owner != self.target {
            return Err(E::TargetMismatch);
        }
        if self.source.content != self.revision.content {
            return Err(E::ContentMismatch);
        }
        if self.dependencies.environment != self.revision.environment {
            return Err(E::EnvironmentMismatch);
        }
        Ok(())
    }
}

impl fmt::Display for SourceBundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema(schema) => {
                write!(f, "unsupported definition source bundle schema {schema}")
            }
            Self::InvalidSource(reason) => write!(f, "invalid saved definition source: {reason}"),
            Self::TargetMismatch => {
                f.write_str("saved source targets a different definition or callable")
            }
            Self::ContentMismatch => {
                f.write_str("saved body and revision content fingerprints differ")
            }
            Self::RevisionMismatch => {
                f.write_str("saved revision key does not match its source/environment inputs")
            }
            Self::EnvironmentMismatch => {
                f.write_str("saved source does not match the current locked source environment")
            }
            Self::ContractMismatch => {
                f.write_str("saved source does not match the current definition contract")
            }
            Self::ConfigurationMismatch => {
                f.write_str("saved source does not match the current compiler configuration")
            }
            Self::DependencyMismatch => {
                f.write_str("saved source does not match the current locked dependencies")
            }
            Self::ResourceLimit { resource, limit } => {
                write!(f, "saved source exceeds {resource} limit {limit}")
            }
        }
    }
}
impl std::error::Error for SourceBundleError {}
