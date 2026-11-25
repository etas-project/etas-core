use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, Ordering},
};

use super::{
    ByteStreamRef, StreamHandleRef,
    admission::{AdmittedStream, OperationPermit, StreamAccess, stream_busy},
    store::{OperationDeadlines, StreamSlot, admit_stream, stream_closed},
};
use crate::HostError;

/// Runtime-owned lifetime for one compound read. Stream arguments carry weak
/// links; keeping an alias from an action handler cannot extend this lifetime.
#[derive(Clone)]
pub struct StreamReadScope(pub(crate) Arc<ReadScopeState>);

#[derive(Clone)]
pub(crate) struct ReadScopeLink(Weak<ReadScopeState>);

pub(crate) struct ReadScopeState {
    handle: StreamHandleRef,
    owner: String,
    held: tokio::sync::Mutex<Option<AdmittedStream>>,
    in_flight: AtomicBool,
    revoked: AtomicBool,
}

pub(crate) struct ReadScopeCall {
    scope: Arc<ReadScopeState>,
    slot: Arc<StreamSlot>,
}

impl Drop for ReadScopeCall {
    fn drop(&mut self) {
        self.scope.in_flight.store(false, Ordering::Release);
        self.slot.scoped_reads.fetch_sub(1, Ordering::AcqRel);
        self.slot.admission_changed.notify_waiters();
    }
}

impl StreamReadScope {
    pub fn new(stream: &ByteStreamRef, owner: String) -> Self {
        Self(Arc::new(ReadScopeState {
            handle: stream.handle().clone(),
            owner,
            held: Default::default(),
            in_flight: AtomicBool::new(false),
            revoked: AtomicBool::new(false),
        }))
    }
    pub(crate) fn link(&self) -> ReadScopeLink {
        ReadScopeLink(Arc::downgrade(&self.0))
    }
}

impl std::fmt::Debug for StreamReadScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StreamReadScope")
    }
}
impl PartialEq for StreamReadScope {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for StreamReadScope {}
impl std::fmt::Debug for ReadScopeLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadScopeLink")
    }
}
impl PartialEq for ReadScopeLink {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ReadScopeLink {}

impl ReadScopeLink {
    pub(crate) fn for_owner(&self, owner: &str) -> Option<Self> {
        self.0
            .upgrade()
            .filter(|scope| scope.owner == owner)
            .map(|_| self.clone())
    }
    pub(crate) fn upgrade(&self) -> Option<Arc<ReadScopeState>> {
        self.0.upgrade()
    }
}

impl ReadScopeState {
    pub(crate) async fn admit(
        self: &Arc<Self>,
        slot: &Arc<StreamSlot>,
        handle: &StreamHandleRef,
        deadlines: OperationDeadlines,
    ) -> Result<AdmittedStream, HostError> {
        if &self.handle != handle || self.revoked.load(Ordering::Acquire) {
            return Err(stream_closed());
        }
        self.in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| stream_busy())?;
        slot.scoped_reads.fetch_add(1, Ordering::AcqRel);
        let call = ReadScopeCall {
            scope: self.clone(),
            slot: slot.clone(),
        };
        let mut held = self.held.lock().await;
        if self.revoked.load(Ordering::Acquire) {
            return Err(stream_closed());
        }
        if held.is_none() {
            *held = Some(admit_stream(slot, handle, StreamAccess::Read, deadlines).await?);
            let mut scopes = slot.read_scopes.lock().unwrap();
            scopes.retain(|scope| scope.strong_count() != 0);
            scopes.push(Arc::downgrade(self));
        }
        if !slot.is_open() {
            self.revoked.store(true, Ordering::Release);
            *held = None;
            return Err(stream_closed());
        }
        Ok(AdmittedStream {
            stream: held.as_ref().unwrap().stream.clone(),
            _permit: OperationPermit::Scoped { _call: call },
        })
    }

    pub(crate) async fn revoke(&self) {
        self.revoked.store(true, Ordering::Release);
        self.held.lock().await.take();
    }
}
