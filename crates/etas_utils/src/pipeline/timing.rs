use std::time::Duration;

use super::{ArtifactSet, PassControl, UnitKey};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PipelineStats {
    pub executed: u32,
    pub skipped: u32,
    pub changed: u32,
    pub failed: u32,
    pub stopped: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassTiming {
    pub pass: &'static str,
    pub unit: Option<UnitKey>,
    pub duration: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassRunRecord {
    pub pass: &'static str,
    pub unit: Option<UnitKey>,
    pub control: PassControl,
    pub changed: bool,
    pub skipped: bool,
    pub produced: ArtifactSet,
    pub invalidated: ArtifactSet,
    pub timing: Option<PassTiming>,
}
