use std::{future::Future, time::Duration};

use crate::{HostError, HostErrorCode, StorageOperationKey, storage::version::StoreGeneration};

use super::{
    SessionGeneration, SessionOperation, SessionRef, SessionRequest, SessionResponse, SessionResult,
};
use crate::execution::{ExternalOutcome, OperationContext};

/// Runtime-owned identity used to revoke a lease chain when its latest
/// generation is unknown. This identity is never a language value.
#[derive(Clone)]
pub struct SessionLeaseScope {
    session: SessionRef,
    key: StorageOperationKey,
    released: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl PartialEq for SessionLeaseScope {
    fn eq(&self, other: &Self) -> bool {
        self.session == other.session && self.key == other.key
    }
}
impl Eq for SessionLeaseScope {}

impl SessionLeaseScope {
    pub fn new(session: SessionRef, lifetime: Duration) -> Result<Self, HostError> {
        if session.id.is_empty() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "session lease requires a session id",
            ));
        }
        Ok(Self {
            session,
            key: StorageOperationKey::new(lifetime)?,
            released: Default::default(),
        })
    }

    pub fn session(&self) -> &SessionRef {
        &self.session
    }

    pub fn expires_at(&self) -> u64 {
        self.key.expires_at()
    }

    pub(super) fn key(&self) -> &str {
        self.key.as_str()
    }

    pub(super) fn issue(&self, now: u64) -> Result<SessionLease, HostError> {
        if self.expires_at() <= now {
            return Err(inactive());
        }
        let generation = StoreGeneration::new()?;
        Ok(SessionLease {
            scope: self.clone(),
            generation: SessionGeneration::issue(&self.session.id, generation.as_str())?,
        })
    }
}

impl std::fmt::Debug for SessionLeaseScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionLeaseScope")
            .field("session", &self.session)
            .field("expires_at", &self.expires_at())
            .finish_non_exhaustive()
    }
}

/// Backend-issued generation. Only the current, unexpired generation can
/// transfer or release its session lease.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionLease {
    scope: SessionLeaseScope,
    generation: SessionGeneration,
}

impl SessionLease {
    pub fn session(&self) -> &SessionRef {
        self.scope.session()
    }

    pub fn generation(&self) -> &SessionGeneration {
        &self.generation
    }

    pub fn scope(&self) -> &SessionLeaseScope {
        &self.scope
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionLeaseOperation {
    Acquire(SessionLeaseScope),
    Handoff(SessionLease),
    Release(SessionLease),
    /// End the entire scope, including any unconfirmed successor.
    Revoke(SessionLeaseScope),
}

impl SessionLeaseOperation {
    pub fn session(&self) -> &SessionRef {
        self.scope().session()
    }

    pub fn qualified_action(&self) -> &'static str {
        match self {
            Self::Acquire(_) => "Session.acquire",
            Self::Handoff(_) => "Session.handoff",
            Self::Release(_) | Self::Revoke(_) => "Session.release",
        }
    }

    fn scope(&self) -> &SessionLeaseScope {
        match self {
            Self::Acquire(scope) | Self::Revoke(scope) => scope,
            Self::Handoff(lease) | Self::Release(lease) => lease.scope(),
        }
    }

    pub(super) fn storage_size(&self) -> Result<usize, HostError> {
        let generation_size = match self {
            Self::Handoff(lease) | Self::Release(lease) => lease.generation().as_token().len(),
            _ => 0,
        };
        self.session()
            .id
            .len()
            .checked_add(self.scope().key().len())
            .and_then(|length| length.checked_add(generation_size))
            .ok_or_else(super::write::limit_error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionLeaseResult {
    Acquired(SessionLease),
    Transferred(SessionLease),
    Released,
    Revoked,
}

/// Keep the acquire and revocation owned even when the caller drops its
/// response future. The cleanup uses the scope identity, independent of how
/// far the lease has been transferred.
pub(super) async fn execute_scoped<F, Fut>(
    request: SessionRequest,
    operation: &OperationContext,
    dispatch: F,
) -> Result<SessionResponse, HostError>
where
    F: Fn(SessionRequest, Option<OperationContext>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<SessionResponse, HostError>> + Send + 'static,
{
    if let SessionOperation::Lease(SessionLeaseOperation::Release(lease)) = &request.operation {
        let released = lease.scope().released.clone();
        let response = dispatch(request, Some(operation.clone())).await?;
        if matches!(
            response.result,
            Ok(SessionResult::Lease(SessionLeaseResult::Released))
        ) {
            released.store(true, std::sync::atomic::Ordering::Release);
        }
        return Ok(response);
    }
    let SessionOperation::Lease(SessionLeaseOperation::Acquire(scope)) = &request.operation else {
        return dispatch(request, Some(operation.clone())).await;
    };
    let registration = operation.register_resource()?;
    if let Err(error) = registration.begin_dispatch() {
        registration.complete(ExternalOutcome::NotDispatched, Vec::new())?;
        return Err(error);
    }
    let mut revoke = request.clone();
    let cleanup_action =
        crate::execution::ResourceCleanup::RevokeSessionLease(scope.session().clone());
    revoke.operation = SessionOperation::Lease(SessionLeaseOperation::Revoke(scope.clone()));
    revoke.budget = crate::ExecutionBudget::default();
    let resource = registration.context().clone();
    let released = scope.released.clone();
    let operation = operation.clone();
    let (send, receive) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let result = dispatch(request, Some(operation)).await;
        let _ = send.send(result);
        let release = resource
            .signal()
            .release_requested(resource.operation())
            .await;
        if release.is_ok() && released.load(std::sync::atomic::Ordering::Acquire) {
            let _ = registration.complete(ExternalOutcome::Confirmed, Vec::new());
            return;
        }
        let cleanup = dispatch(revoke, None)
            .await
            .and_then(|response| match response.result? {
                SessionResult::Lease(SessionLeaseResult::Revoked) => Ok(()),
                _ => Err(HostError::new(
                    HostErrorCode::InvalidResponse,
                    "session lease revocation has no confirmation",
                )),
            });
        let outcome = if cleanup.is_ok() {
            ExternalOutcome::Confirmed
        } else {
            ExternalOutcome::Unknown
        };
        let errors = release
            .err()
            .into_iter()
            .chain(cleanup.err())
            .map(|error| error.with_detail("resource_kind", "session_lease"))
            .collect();
        let _ = registration.complete_with_cleanup(outcome, errors, Some(cleanup_action));
    });
    receive.await.map_err(|_| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "session lease acquisition task stopped",
        )
    })?
}

pub(super) fn held() -> HostError {
    HostError::new(
        HostErrorCode::ProviderRejected,
        "session already has a routing lease",
    )
}

pub(super) fn inactive() -> HostError {
    HostError::new(
        HostErrorCode::ProviderRejected,
        "session lease generation is no longer active",
    )
}
