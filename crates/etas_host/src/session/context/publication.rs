use std::collections::BTreeMap;

use crate::session::{SessionGeneration, SessionHistoryFence};
use crate::{
    HostError, HostErrorCode, HostValue, SessionRef, StorageDurability, StorageLimits,
    StorageOperationKey, StorageOperationRef, WriteOutcome,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionContextContent {
    pub text: String,
    pub provenance: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionPublishedContext {
    pub content: SessionContextContent,
    pub fence: SessionHistoryFence,
    pub version: u64,
}

impl SessionPublishedContext {
    pub fn storage_size(&self, limits: &StorageLimits) -> Result<usize, HostError> {
        if self.version == 0 || self.version > i64::MAX as u64 {
            return Err(invalid());
        }
        self.content
            .encode(limits)?
            .len()
            .checked_add(self.fence.as_token().len())
            .ok_or_else(crate::session::write::limit_error)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionContextPublication {
    pub session: SessionRef,
    pub fence: SessionHistoryFence,
    pub content: SessionContextContent,
    pub operation: StorageOperationRef,
}

impl SessionContextPublication {
    pub(crate) fn retained_size(&self, limits: &StorageLimits) -> Result<usize, HostError> {
        self.operation.validate()?;
        if self.content.provenance.len() > limits.max_nodes {
            return Err(crate::session::write::limit_error());
        }
        let mut bytes = std::mem::size_of::<Self>();
        for text in [
            self.session.id.as_str(),
            self.fence.as_token(),
            self.content.text.as_str(),
            self.operation.key.as_str(),
            self.operation.request_fingerprint.as_str(),
        ]
        .into_iter()
        .chain(
            self.content
                .provenance
                .iter()
                .flat_map(|(k, v)| [k.as_str(), v.as_str()]),
        ) {
            bytes = bytes
                .checked_add(text.len())
                .ok_or_else(crate::session::write::limit_error)?;
        }
        bytes = self
            .content
            .provenance
            .len()
            .checked_mul(std::mem::size_of::<(String, String)>())
            .and_then(|size| bytes.checked_add(size))
            .ok_or_else(crate::session::write::limit_error)?;
        if bytes > limits.max_value_bytes {
            return Err(crate::session::write::limit_error());
        }
        Ok(bytes)
    }

    pub fn prepare(
        session: SessionRef,
        fence: SessionHistoryFence,
        content: SessionContextContent,
        limits: &StorageLimits,
    ) -> Result<Self, HostError> {
        let key = StorageOperationKey::new(std::time::Duration::from_secs(
            limits.max_receipt_retention_seconds,
        ))?;
        let operation = context_operation_ref(&session, &fence, &content, key, limits)?;
        Ok(Self {
            session,
            fence,
            content,
            operation,
        })
    }

    pub(crate) fn validate(&self, limits: &StorageLimits) -> Result<usize, HostError> {
        let expected = context_operation_ref(
            &self.session,
            &self.fence,
            &self.content,
            self.operation.key.clone(),
            limits,
        )?;
        if expected != self.operation {
            return Err(crate::session::write::mismatch());
        }
        Ok(encode_request(&self.session, &self.fence, &self.content, limits)?.len())
    }
}

pub fn context_operation_ref(
    session: &SessionRef,
    fence: &SessionHistoryFence,
    content: &SessionContextContent,
    key: StorageOperationKey,
    limits: &StorageLimits,
) -> Result<StorageOperationRef, HostError> {
    let encoded = encode_request(session, fence, content, limits)?;
    let mut hash = blake3::Hasher::new_derive_key("etas.session.context-publication.v1");
    hash.update(encoded.as_bytes());
    Ok(StorageOperationRef {
        key,
        request_fingerprint: hash.finalize().to_hex().to_string(),
    })
}

fn encode_request(
    session: &SessionRef,
    fence: &SessionHistoryFence,
    content: &SessionContextContent,
    limits: &StorageLimits,
) -> Result<String, HostError> {
    if session.id.is_empty() || session.id != fence.session_id() {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "context fence belongs to another session",
        ));
    }
    // Validate before cloning into the bounded canonical wire value.
    let content_bytes = content.encode(limits)?;
    let total = session
        .id
        .len()
        .checked_add(fence.as_token().len())
        .and_then(|n| n.checked_add(content_bytes.len()))
        .ok_or_else(crate::session::write::limit_error)?;
    if total > limits.max_value_bytes {
        return Err(crate::session::write::limit_error());
    }
    crate::value::tagged::encode_with_limits(
        &HostValue::Record(vec![
            ("session".into(), HostValue::String(session.id.clone())),
            (
                "fence".into(),
                HostValue::String(fence.as_token().to_owned()),
            ),
            ("content".into(), HostValue::String(content_bytes)),
        ]),
        limits,
    )
}

impl SessionContextContent {
    pub(crate) fn encode(&self, limits: &StorageLimits) -> Result<String, HostError> {
        if self.text.trim().is_empty() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "published context must not be empty",
            ));
        }
        let mut size = self.text.len();
        if self.provenance.len() > limits.max_nodes {
            return Err(crate::session::write::limit_error());
        }
        for (key, value) in &self.provenance {
            if key.is_empty() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "context provenance key is empty",
                ));
            }
            size = size
                .checked_add(key.len())
                .and_then(|n| n.checked_add(value.len()))
                .ok_or_else(crate::session::write::limit_error)?;
        }
        if size > limits.max_value_bytes {
            return Err(crate::session::write::limit_error());
        }
        crate::value::tagged::encode_with_limits(
            &HostValue::Record(vec![
                ("text".into(), HostValue::String(self.text.clone())),
                (
                    "provenance".into(),
                    HostValue::Record(
                        self.provenance
                            .iter()
                            .map(|(k, v)| (k.clone(), HostValue::String(v.clone())))
                            .collect(),
                    ),
                ),
            ]),
            limits,
        )
    }

    pub(crate) fn decode(text: &str, limits: &StorageLimits) -> Result<Self, HostError> {
        let HostValue::Record(fields) = crate::value::tagged::decode_with_limits(text, limits)?
        else {
            return Err(invalid());
        };
        let [
            (text_key, HostValue::String(text)),
            (provenance_key, HostValue::Record(provenance)),
        ] = fields.as_slice()
        else {
            return Err(invalid());
        };
        if text_key != "text" || provenance_key != "provenance" {
            return Err(invalid());
        }
        let provenance = provenance
            .iter()
            .map(|(k, v)| match v {
                HostValue::String(v) => Ok((k.clone(), v.clone())),
                _ => Err(invalid()),
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let content = Self {
            text: text.clone(),
            provenance,
        };
        content.encode(limits)?;
        Ok(content)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionContextReceipt {
    pub operation: StorageOperationRef,
    pub session: SessionRef,
    pub generation: SessionGeneration,
    pub context_version: u64,
    pub durability: StorageDurability,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionContextRejection {
    StaleHistory,
    Rejected(HostError),
}

pub type SessionContextOutcome = WriteOutcome<SessionContextReceipt, SessionContextRejection>;

#[derive(Clone, Debug, PartialEq)]
pub enum SessionContextEvidence {
    Committed(SessionContextReceipt),
    NotCommitted {
        operation: StorageOperationRef,
        reason: SessionContextRejection,
    },
}

impl SessionContextEvidence {
    pub fn operation(&self) -> &StorageOperationRef {
        match self {
            Self::Committed(r) => &r.operation,
            Self::NotCommitted { operation, .. } => operation,
        }
    }
    pub fn into_outcome(self) -> SessionContextOutcome {
        match self {
            Self::Committed(r) => WriteOutcome::Committed(r),
            Self::NotCommitted { operation, reason } => {
                WriteOutcome::NotCommitted { operation, reason }
            }
        }
    }
    pub(crate) fn charge(&self, session: &str) -> Result<usize, HostError> {
        let op = self.operation();
        crate::storage::receipt_budget::charge([
            session,
            op.key.as_str(),
            op.request_fingerprint.as_str(),
            session,
        ])
    }
}

pub(crate) fn invalid() -> HostError {
    HostError::new(
        HostErrorCode::SchemaMismatch,
        "invalid stored session context",
    )
}

pub(crate) fn rejected(operation: StorageOperationRef, error: HostError) -> SessionContextOutcome {
    WriteOutcome::NotCommitted {
        operation,
        reason: SessionContextRejection::Rejected(error),
    }
}
