use super::{ArtifactKey, ArtifactRef, ArtifactSet, PassScope, PreservedArtifacts, UnitKey};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassResult {
    pub control: PassControl,
    pub changed: bool,
    pub preserved: PreservedArtifacts,
    pub produced: ArtifactSet,
}

impl PassResult {
    pub fn unchanged() -> Self {
        Self {
            control: PassControl::Continue,
            changed: false,
            preserved: PreservedArtifacts::All,
            produced: ArtifactSet::new(),
        }
    }

    pub fn changed(preserved: PreservedArtifacts, produced: ArtifactSet) -> Self {
        Self {
            control: PassControl::Continue,
            changed: true,
            preserved,
            produced,
        }
    }

    pub fn stop() -> Self {
        Self {
            control: PassControl::Stop,
            changed: false,
            preserved: PreservedArtifacts::All,
            produced: ArtifactSet::new(),
        }
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            control: PassControl::Failed(PassFailure::new(message)),
            changed: false,
            preserved: PreservedArtifacts::All,
            produced: ArtifactSet::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PassControl {
    Continue,
    Stop,
    Failed(PassFailure),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassFailure {
    pub message: String,
    pub missing_artifact: Option<ArtifactKey>,
}

impl PassFailure {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            missing_artifact: None,
        }
    }

    pub fn missing_artifact(key: ArtifactKey) -> Self {
        Self {
            message: format!(
                "required artifact {}.{} is not available",
                key.namespace, key.name
            ),
            missing_artifact: Some(key),
        }
    }

    pub fn missing_artifact_ref(artifact: ArtifactRef) -> Self {
        Self {
            message: format!(
                "required artifact {}.{} with scope {:?} is not available",
                artifact.key.namespace, artifact.key.name, artifact.scope
            ),
            missing_artifact: Some(artifact.key),
        }
    }

    pub fn scope_mismatch(
        pass: &'static str,
        scope: PassScope,
        current_unit: Option<UnitKey>,
    ) -> Self {
        Self {
            message: format!(
                "pass {pass} has scope {scope:?} but current unit is {current_unit:?}"
            ),
            missing_artifact: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipelineRunResult {
    pub control: PassControl,
    pub stats: super::PipelineStats,
    pub records: Vec<super::PassRunRecord>,
}

impl PipelineRunResult {
    pub fn completed(stats: super::PipelineStats, records: Vec<super::PassRunRecord>) -> Self {
        Self {
            control: PassControl::Continue,
            stats,
            records,
        }
    }

    pub fn stopped(
        control: PassControl,
        stats: super::PipelineStats,
        records: Vec<super::PassRunRecord>,
    ) -> Self {
        Self {
            control,
            stats,
            records,
        }
    }
}
