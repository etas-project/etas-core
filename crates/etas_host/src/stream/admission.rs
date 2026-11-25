use std::sync::{Arc, atomic::Ordering};

use super::store::{ManagedStream, StreamLifecycle, StreamSlot, stream_closed};
use crate::{HostError, HostErrorCode, StreamHandleRef};

#[derive(Clone, Copy)]
pub(crate) enum StreamAccess {
    Read,
    Write,
    Exclusive,
}

impl StreamAccess {
    fn mask(self) -> u8 {
        match self {
            Self::Read => 1,
            Self::Write => 2,
            Self::Exclusive => 3,
        }
    }
}

pub(crate) struct StreamPermit {
    slot: Arc<StreamSlot>,
    mask: u8,
}

impl Drop for StreamPermit {
    fn drop(&mut self) {
        self.slot.admission.fetch_and(!self.mask, Ordering::AcqRel);
        self.slot.admission_changed.notify_waiters();
    }
}

pub(crate) struct AdmittedStream {
    // Drop the connection clone before reopening admission for TLS transfer.
    pub(crate) stream: ManagedStream,
    pub(crate) _permit: OperationPermit,
}

pub(crate) enum OperationPermit {
    Native {
        _permit: StreamPermit,
    },
    Scoped {
        _call: super::read_scope::ReadScopeCall,
    },
}

impl StreamSlot {
    pub(crate) fn admit(
        self: &Arc<Self>,
        handle: &StreamHandleRef,
        access: StreamAccess,
    ) -> Result<StreamPermit, HostError> {
        self.validate_generation(handle)?;
        if !matches!(
            self.lifecycle(),
            StreamLifecycle::OpenTcp | StreamLifecycle::OpenTls | StreamLifecycle::OpenFile
        ) {
            return Err(stream_closed());
        }
        let mask = if self.lifecycle() == StreamLifecycle::OpenFile {
            3
        } else {
            access.mask()
        };
        let mut active = self.admission.load(Ordering::Acquire);
        loop {
            if active & mask != 0 {
                return Err(stream_busy());
            }
            match self.admission.compare_exchange_weak(
                active,
                active | mask,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(current) => active = current,
            }
        }
        let permit = StreamPermit {
            slot: self.clone(),
            mask,
        };
        self.validate_generation(handle)?;
        if !matches!(
            self.lifecycle(),
            StreamLifecycle::OpenTcp | StreamLifecycle::OpenTls | StreamLifecycle::OpenFile
        ) {
            return Err(stream_closed());
        }
        Ok(permit)
    }
}

pub(crate) fn stream_busy() -> HostError {
    HostError::new(HostErrorCode::InvalidRequest, "byte stream is busy")
        .with_detail("stream_failure", "busy")
}
