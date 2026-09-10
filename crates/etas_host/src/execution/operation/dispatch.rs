use crate::HostError;

/// Certainty about whether a managed job could have executed.
#[derive(Debug)]
pub(crate) enum DispatchError {
    NotDispatched(HostError),
    OutcomeUnavailable(HostError),
}

impl DispatchError {
    pub fn into_host_error(self) -> HostError {
        match self {
            Self::NotDispatched(error) | Self::OutcomeUnavailable(error) => error,
        }
    }
}
