use crate::{HostError, HostErrorCode, MemoryCursor, StorageLimits};

pub(super) struct CursorFence<'a> {
    pub scope: &'a str,
    pub generation: &'a str,
    pub revision: i64,
}
impl CursorFence<'_> {
    pub fn resume(
        &self,
        cursor: Option<&MemoryCursor>,
        limits: &StorageLimits,
    ) -> Result<Option<String>, HostError> {
        let Some(cursor) = cursor else {
            return Ok(None);
        };
        if cursor.opaque.len() > limits.max_value_bytes {
            return Err(invalid());
        }
        let (format, scope, generation, revision, key): (u8, String, String, i64, String) =
            serde_json::from_str(&cursor.opaque).map_err(|_| invalid())?;
        if format != 1
            || scope != self.scope
            || generation != self.generation
            || revision != self.revision
        {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "memory cursor is stale or belongs to another Store",
            ));
        }
        Ok(Some(key))
    }
    pub fn after(&self, key: &str, limits: &StorageLimits) -> Result<MemoryCursor, HostError> {
        let opaque = serde_json::to_string(&(1u8, self.scope, self.generation, self.revision, key))
            .map_err(|_| invalid())?;
        if opaque.len() > limits.max_value_bytes {
            return Err(invalid());
        }
        Ok(MemoryCursor { opaque })
    }
}
fn invalid() -> HostError {
    HostError::new(HostErrorCode::InvalidRequest, "invalid memory cursor")
}
