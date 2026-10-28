#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum PipelineInterruptionKind {
    Cancelled,
    ResourceLimit,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct PipelineInterruption {
    pub kind: PipelineInterruptionKind,
    pub reason: String,
}

/// An embedding application's cooperative execution boundary, independent of
/// any frontend, Host protocol, runtime or signal implementation.
pub trait PipelineExecutionControl: Send + Sync {
    fn checkpoint(&self) -> Result<(), PipelineInterruption>;
}
