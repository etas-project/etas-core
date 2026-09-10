use super::StorageLimits;
use crate::{HostError, HostErrorCode};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) struct Admission {
    limits: StorageLimits,
    state: Mutex<(usize, usize)>,
}
impl Admission {
    pub fn new(limits: StorageLimits) -> Self {
        Self {
            limits,
            state: Mutex::new((0, 0)),
        }
    }
    pub fn try_reserve(self: &Arc<Self>, bytes: usize) -> Result<Reservation, HostError> {
        let mut state = self.state.lock().map_err(|_| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "storage admission state poisoned",
            )
        })?;
        let total = state
            .1
            .checked_add(bytes)
            .ok_or_else(super::config::exceeded)?;
        if state.0 >= self.limits.max_pending_jobs || total > self.limits.max_pending_bytes {
            return Err(HostError::new(
                HostErrorCode::ProviderUnavailable,
                "storage admission capacity exhausted",
            )
            .with_detail("reason", "overloaded"));
        }
        state.0 += 1;
        state.1 = total;
        Ok(Reservation {
            admission: self.clone(),
            bytes,
        })
    }
}
pub(crate) struct Reservation {
    admission: Arc<Admission>,
    bytes: usize,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        // Even after a poisoned lock, release already-owned accounting. New
        // admissions still observe poison and fail closed.
        let mut state = self
            .admission
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.0 -= 1;
        state.1 -= self.bytes;
    }
}
