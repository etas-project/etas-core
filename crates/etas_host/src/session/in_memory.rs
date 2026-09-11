use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
};

use crate::{
    ContextPolicy, HostError, HostErrorCode, SessionClient, SessionConfig, SessionCursor,
    SessionMessage, SessionOperation, SessionRef, SessionRequest, SessionResponse, SessionResult,
    SessionSummary,
};

mod context;
mod maintenance;
mod paging;
mod resolve;
mod write;

#[derive(Clone, Debug, Default)]
pub struct InMemorySessionClient {
    executor: crate::storage::volatile::VolatileExecutor,
    sessions: Arc<RwLock<BTreeMap<String, SessionState>>>,
    limits: crate::StorageLimits,
}

#[derive(Clone, Debug)]
struct SessionState {
    retention_receipts: BTreeMap<String, super::SessionRetentionReceipt>,
    context_receipts: BTreeMap<String, super::SessionContextEvidence>,
    published_context: Option<super::SessionPublishedContext>,
    history_key: super::context::fence::HistoryKey,
    context_version: u64,
    generation: crate::storage::version::StoreGeneration,
    storage_generation: crate::storage::version::StoreGeneration,
    receipts: BTreeMap<String, super::SessionWriteReceipt>,
    config: SessionConfig,
    messages: BTreeMap<i64, SessionMessage>,
    last_ordinal: i64,
    dedup: BTreeMap<String, String>,
    summary: Option<SessionSummary>,
}

impl InMemorySessionClient {
    pub async fn execute_scoped(
        &self,
        request: SessionRequest,
        operation: &crate::execution::OperationContext,
    ) -> Result<SessionResponse, HostError> {
        self.dispatch(request, Some(operation)).await
    }
    async fn dispatch(
        &self,
        request: SessionRequest,
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<SessionResponse, HostError> {
        let bytes = request.storage_size(&self.limits)?;
        let client = self.clone();
        self.executor
            .execute(
                operation,
                request.id,
                request.trace.clone(),
                request.budget.clone(),
                bytes,
                move |_| {
                    let id = request.id;
                    let result = client.execute_operation(request.operation);
                    Ok(SessionResponse { id, result })
                },
            )
            .await
            .map_err(crate::execution::DispatchError::into_host_error)
    }
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_limits(limits: crate::StorageLimits) -> Result<Self, HostError> {
        limits.validate()?;
        Ok(Self {
            executor: crate::storage::volatile::VolatileExecutor::new(limits.clone()),
            sessions: Default::default(),
            limits,
        })
    }
}

impl SessionClient for InMemorySessionClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<SessionResponse, Self::Error>> + Send + 'a>>;

    type WriteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<super::SessionWriteResponse, HostError>> + Send + 'a>>;
    fn write(&self, request: super::SessionWriteRequest) -> Self::WriteFuture<'_> {
        Box::pin(self.dispatch_write(request, None))
    }

    fn execute(&self, request: SessionRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(self.dispatch(request, None))
    }
}

impl InMemorySessionClient {
    fn execute_operation(&self, operation: SessionOperation) -> Result<SessionResult, HostError> {
        match operation {
            SessionOperation::Resolve { config } => self.resolve(config),
            SessionOperation::Append { message } => self.append(message),
            SessionOperation::Load {
                session,
                context,
                cursor,
                limit,
            } => self.load(session, context, cursor, limit),
        }
    }

    fn resolve(&self, config: SessionConfig) -> Result<SessionResult, HostError> {
        if config.id.is_empty() {
            return Err(invalid_request("session id must not be empty"));
        }
        let mut sessions = self.sessions.write().map_err(lock_error)?;
        let created = !sessions.contains_key(&config.id);
        if let Some(existing) = sessions.get(&config.id) {
            if existing.config != config {
                return Err(invalid_request(
                    "session identity already has a different configuration",
                ));
            }
        }
        if created {
            sessions.insert(
                config.id.clone(),
                SessionState {
                    retention_receipts: BTreeMap::new(),
                    context_receipts: BTreeMap::new(),
                    published_context: None,
                    history_key: super::context::fence::HistoryKey::new()?,
                    context_version: 0,
                    generation: crate::storage::version::StoreGeneration::new()?,
                    storage_generation: crate::storage::version::StoreGeneration::new()?,
                    receipts: BTreeMap::new(),
                    config: config.clone(),
                    messages: BTreeMap::new(),
                    last_ordinal: -1,
                    dedup: BTreeMap::new(),
                    summary: None,
                },
            );
        }
        Ok(SessionResult::Resolved {
            session: SessionRef { id: config.id },
            created,
        })
    }

    fn append(&self, message: SessionMessage) -> Result<SessionResult, HostError> {
        message.storage_size(&self.limits)?;
        let mut sessions = self.sessions.write().map_err(lock_error)?;
        let state = sessions
            .get_mut(&message.session.id)
            .ok_or_else(|| invalid_request("cannot append message to an unresolved session"))?;
        let (message, deduplicated, _) = write::prepare_append(state, message)?;
        if !deduplicated {
            write::apply_append(state, &message);
        }
        Ok(SessionResult::Appended {
            message,
            deduplicated,
        })
    }

    fn load(
        &self,
        session: SessionRef,
        context: ContextPolicy,
        cursor: Option<SessionCursor>,
        limit: Option<u32>,
    ) -> Result<SessionResult, HostError> {
        let sessions = self.sessions.read().map_err(lock_error)?;
        let state = sessions
            .get(&session.id)
            .ok_or_else(|| invalid_request("cannot load unresolved session history"))?;
        paging::load(state, session, context, cursor, limit, &self.limits)
    }
}

fn invalid_request(message: impl Into<String>) -> HostError {
    HostError::new(HostErrorCode::InvalidRequest, message)
}

fn lock_error<T>(_: T) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "session store lock poisoned",
    )
}
