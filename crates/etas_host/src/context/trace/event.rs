use crate::{
    AuthorityContext, HostError, HostErrorCode, HostJsonValue, HostRequestId, HostRequestKind,
    HostValue, TraceContext,
};

#[derive(Clone, Debug, PartialEq)]
pub enum TraceEvent {
    HostRequestStarted {
        id: HostRequestId,
        kind: HostRequestKind,
        metadata: HostTraceMetadata,
        authority: Box<AuthorityContext>,
        trace: TraceContext,
        started_at_unix_micros: u64,
    },
    HostRequestFinished {
        id: HostRequestId,
        outcome: HostOutcome,
        finished_at_unix_micros: u64,
        duration_micros: u64,
    },
    ApprovalRequested {
        id: HostRequestId,
        metadata: HostTraceMetadata,
        trace: TraceContext,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct HostTraceMetadata {
    pub qualified_action: String,
    pub subject_kind: String,
    pub fields: Vec<HostTraceFieldMetadata>,
    pub payload_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HostTracePayload {
    pub qualified_action: String,
    pub subject_kind: String,
    pub fields: Vec<HostTracePayloadField>,
}

impl HostTracePayload {
    pub fn new(subject_kind: impl Into<String>, qualified_action: impl Into<String>) -> Self {
        Self {
            qualified_action: qualified_action.into(),
            subject_kind: subject_kind.into(),
            fields: Vec::new(),
        }
    }

    pub fn with_field(
        mut self,
        name: impl Into<String>,
        value: HostValue,
        sensitivity: HostTraceFieldSensitivity,
    ) -> Self {
        self.fields.push(HostTracePayloadField {
            name: name.into(),
            value,
            sensitivity,
        });
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HostTracePayloadField {
    pub name: String,
    pub value: HostValue,
    pub sensitivity: HostTraceFieldSensitivity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HostTraceFieldMetadata {
    pub name: String,
    pub sensitivity: HostTraceFieldSensitivity,
    pub value: Option<HostValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostTraceFieldSensitivity {
    Public,
    Sensitive,
    Secret,
}

#[derive(Clone, PartialEq, Eq)]
pub struct HostTraceDigestKey([u8; 32]);

impl HostTraceDigestKey {
    pub fn generate() -> Result<Self, HostError> {
        let mut key = [0_u8; 32];
        getrandom::fill(&mut key).map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                format!("failed to generate the run-owned host trace digest key: {error}"),
            )
        })?;
        Ok(Self(key))
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for HostTraceDigestKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("HostTraceDigestKey(<redacted>)")
    }
}

impl HostTraceMetadata {
    pub fn from_payload(
        payload: &HostTracePayload,
        key: &HostTraceDigestKey,
    ) -> Result<Self, HostError> {
        if payload.qualified_action.is_empty() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "host trace payload requires a non-empty qualified action",
            ));
        }
        if payload.subject_kind.is_empty() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "host trace payload requires a non-empty subject kind",
            ));
        }
        let mut fields = payload.fields.iter().collect::<Vec<_>>();
        fields.sort_by(|left, right| left.name.cmp(&right.name));
        if fields.windows(2).any(|pair| pair[0].name == pair[1].name) {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "host trace payload contains duplicate field names",
            ));
        }

        let mut hasher = blake3::Hasher::new_keyed(key.bytes());
        hash_bytes(&mut hasher, b"etas.host.trace.payload.v2");
        hash_bytes(&mut hasher, payload.subject_kind.as_bytes());
        hash_bytes(&mut hasher, payload.qualified_action.as_bytes());
        for field in &fields {
            hash_bytes(&mut hasher, field.name.as_bytes());
            hash_bytes(&mut hasher, sensitivity_name(field.sensitivity).as_bytes());
            hash_host_value(&mut hasher, &field.value);
        }
        let fields = fields
            .into_iter()
            .map(|field| HostTraceFieldMetadata {
                name: field.name.clone(),
                sensitivity: field.sensitivity,
                value: matches!(field.sensitivity, HostTraceFieldSensitivity::Public)
                    .then(|| field.value.clone()),
            })
            .collect();
        Ok(Self {
            qualified_action: payload.qualified_action.clone(),
            subject_kind: payload.subject_kind.clone(),
            fields,
            payload_digest: hasher.finalize().to_hex().to_string(),
        })
    }

    pub fn for_action(
        subject_kind: impl Into<String>,
        qualified_action: impl Into<String>,
        key: &HostTraceDigestKey,
    ) -> Result<Self, HostError> {
        Self::from_payload(&HostTracePayload::new(subject_kind, qualified_action), key)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostOutcome {
    Succeeded,
    Failed(HostError),
    Cancelled { reason: String },
}

fn sensitivity_name(sensitivity: HostTraceFieldSensitivity) -> &'static str {
    match sensitivity {
        HostTraceFieldSensitivity::Public => "public",
        HostTraceFieldSensitivity::Sensitive => "sensitive",
        HostTraceFieldSensitivity::Secret => "secret",
    }
}

fn hash_host_value(hasher: &mut blake3::Hasher, value: &HostValue) {
    match value {
        HostValue::Unit => hash_bytes(hasher, b"unit"),
        HostValue::Bool(value) => {
            hash_bytes(hasher, b"bool");
            hash_bytes(hasher, &[*value as u8]);
        }
        HostValue::Int(value) => {
            hash_bytes(hasher, b"int");
            hash_bytes(hasher, &value.to_be_bytes());
        }
        HostValue::UInt(value) => {
            hash_bytes(hasher, b"uint");
            hash_bytes(hasher, &value.to_be_bytes());
        }
        HostValue::Float(value) => {
            hash_bytes(hasher, b"float");
            hash_bytes(hasher, &value.to_bits().to_be_bytes());
        }
        HostValue::String(value) => {
            hash_bytes(hasher, b"string");
            hash_bytes(hasher, value.as_bytes());
        }
        HostValue::Bytes(value) => {
            hash_bytes(hasher, b"bytes");
            hash_bytes(hasher, value);
        }
        HostValue::List(values) => {
            hash_bytes(hasher, b"list");
            hash_len(hasher, values.len());
            for value in values {
                hash_host_value(hasher, value);
            }
        }
        HostValue::Map(entries) => {
            hash_bytes(hasher, b"map");
            let mut encoded = entries
                .iter()
                .map(|(key, value)| (encoded_host_value(key), encoded_host_value(value)))
                .collect::<Vec<_>>();
            encoded.sort();
            hash_len(hasher, encoded.len());
            for (key, value) in encoded {
                hash_bytes(hasher, &key);
                hash_bytes(hasher, &value);
            }
        }
        HostValue::Record(fields) => {
            hash_bytes(hasher, b"record");
            let mut fields = fields.iter().collect::<Vec<_>>();
            fields.sort_by(|(left, _), (right, _)| left.cmp(right));
            hash_len(hasher, fields.len());
            for (name, value) in fields {
                hash_bytes(hasher, name.as_bytes());
                hash_host_value(hasher, value);
            }
        }
        HostValue::Variant { name, fields } => {
            hash_bytes(hasher, b"variant");
            hash_bytes(hasher, name.as_bytes());
            hash_len(hasher, fields.len());
            for value in fields {
                hash_host_value(hasher, value);
            }
        }
        HostValue::Json(value) => {
            hash_bytes(hasher, b"json");
            hash_json_value(hasher, value);
        }
    }
}

fn hash_json_value(hasher: &mut blake3::Hasher, value: &HostJsonValue) {
    match value {
        HostJsonValue::Null => hash_bytes(hasher, b"null"),
        HostJsonValue::Bool(value) => {
            hash_bytes(hasher, b"bool");
            hash_bytes(hasher, &[*value as u8]);
        }
        HostJsonValue::Number(value) => {
            hash_bytes(hasher, b"number");
            hash_bytes(hasher, &value.to_bits().to_be_bytes());
        }
        HostJsonValue::String(value) => {
            hash_bytes(hasher, b"string");
            hash_bytes(hasher, value.as_bytes());
        }
        HostJsonValue::Array(values) => {
            hash_bytes(hasher, b"array");
            hash_len(hasher, values.len());
            for value in values {
                hash_json_value(hasher, value);
            }
        }
        HostJsonValue::Object(fields) => {
            hash_bytes(hasher, b"object");
            let mut fields = fields.iter().collect::<Vec<_>>();
            fields.sort_by(|(left, _), (right, _)| left.cmp(right));
            hash_len(hasher, fields.len());
            for (name, value) in fields {
                hash_bytes(hasher, name.as_bytes());
                hash_json_value(hasher, value);
            }
        }
    }
}

fn encoded_host_value(value: &HostValue) -> Vec<u8> {
    let mut hasher = blake3::Hasher::new();
    hash_host_value(&mut hasher, value);
    hasher.finalize().as_bytes().to_vec()
}

fn hash_len(hasher: &mut blake3::Hasher, len: usize) {
    hash_bytes(hasher, &(len as u128).to_be_bytes());
}

fn hash_bytes(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u128).to_be_bytes());
    hasher.update(bytes);
}
