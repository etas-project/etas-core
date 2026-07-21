use crate::{AuthorityContext, Budget, HostError, HostRequestId, HostValue, TraceContext};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryRegionRef {
    pub stable_id: String,
    pub schema_fingerprint: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreRef {
    pub region: MemoryRegionRef,
    pub path: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryRequest {
    pub id: HostRequestId,
    pub store: StoreRef,
    pub operation: MemoryOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MemoryOperation {
    Get {
        key: HostValue,
    },
    Put {
        key: HostValue,
        value: HostValue,
        expected: Option<MemoryVersion>,
        mode: MemoryWriteMode,
    },
    Delete {
        key: HostValue,
        expected: Option<MemoryVersion>,
    },
    Scan {
        cursor: Option<MemoryCursor>,
        limit: Option<u32>,
    },
    Query {
        query: MemoryQuery,
        limit: Option<u32>,
    },
    VectorSearch {
        embedding: Vec<f32>,
        limit: u32,
        filter: Option<HostValue>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryWriteMode {
    Put,
    Insert,
    Update,
    Upsert,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryResponse {
    pub id: HostRequestId,
    pub result: Result<MemoryResult, HostError>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MemoryResult {
    None,
    Value {
        value: HostValue,
        version: MemoryVersion,
    },
    Entries {
        entries: Vec<MemoryEntry>,
        cursor: Option<MemoryCursor>,
    },
    Written {
        version: MemoryVersion,
    },
    Deleted {
        version: MemoryVersion,
    },
    Conflict(MemoryConflict),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryEntry {
    pub key: HostValue,
    pub value: HostValue,
    pub version: MemoryVersion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryVersion {
    pub opaque: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryConflict {
    pub expected: Option<MemoryVersion>,
    pub actual: Option<MemoryVersion>,
    pub current_value: Option<HostValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryCursor {
    pub opaque: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryQuery {
    pub predicate: Option<HostValue>,
    pub order_by: Vec<MemoryOrderKey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryOrderKey {
    pub field_path: Vec<String>,
    pub descending: bool,
}
