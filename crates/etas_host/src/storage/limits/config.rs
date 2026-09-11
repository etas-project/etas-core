use crate::{HostError, HostErrorCode, HostJsonValue, HostValue};

#[derive(Clone, Debug)]
pub struct StorageLimits {
    pub max_pending_jobs: usize,
    pub max_pending_bytes: usize,
    pub max_value_bytes: usize,
    pub max_result_bytes: usize,
    pub max_nodes: usize,
    pub max_depth: usize,
    pub max_page_entries: usize,
    pub max_scan_rows: usize,
    pub max_receipts: usize,
    pub max_receipt_bytes: usize,
    pub max_receipt_retention_seconds: u64,
}
impl Default for StorageLimits {
    fn default() -> Self {
        Self {
            max_pending_jobs: 32,
            max_pending_bytes: 64 * 1024 * 1024,
            max_value_bytes: 1024 * 1024,
            max_result_bytes: 4 * 1024 * 1024,
            max_nodes: 16384,
            max_depth: 48,
            max_page_entries: 1000,
            max_scan_rows: 10000,
            max_receipts: 10000,
            max_receipt_bytes: 64 * 1024 * 1024,
            max_receipt_retention_seconds: 86400,
        }
    }
}
impl StorageLimits {
    pub(crate) fn value_budget(&self) -> ValueBudget<'_> {
        ValueBudget {
            limits: self,
            max_bytes: self.max_value_bytes,
            nodes: 0,
            bytes: 0,
        }
    }
    pub fn validate(&self) -> Result<(), HostError> {
        if self.max_pending_jobs == 0
            || self.max_pending_jobs > 1024
            || self.max_value_bytes == 0
            || self.max_result_bytes == 0
            || self.max_nodes == 0
            || self.max_depth == 0
            || self.max_depth > 64
            || self.max_page_entries == 0
            || self.max_scan_rows == 0
            || self.max_receipts == 0
            || self.max_receipt_bytes == 0
            || self.max_receipt_retention_seconds == 0
            || self
                .max_value_bytes
                .checked_add(self.max_result_bytes)
                .is_none_or(|n| n > self.max_pending_bytes)
        {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "invalid storage limits",
            ));
        }
        Ok(())
    }
    pub fn value_size(&self, value: &HostValue) -> Result<usize, HostError> {
        self.bounded_value_size(value, self.max_value_bytes)
    }
    pub fn result_value_size(&self, value: &HostValue) -> Result<usize, HostError> {
        self.bounded_value_size(value, self.max_result_bytes)
    }
    fn bounded_value_size(&self, value: &HostValue, max_bytes: usize) -> Result<usize, HostError> {
        let mut budget = ValueBudget {
            limits: self,
            max_bytes,
            nodes: 0,
            bytes: 0,
        };
        budget.host(value, 0)?;
        Ok(budget.bytes)
    }
}
pub(crate) struct ValueBudget<'a> {
    limits: &'a StorageLimits,
    max_bytes: usize,
    nodes: usize,
    bytes: usize,
}
impl ValueBudget<'_> {
    pub(crate) fn add(&mut self, depth: usize, bytes: usize) -> Result<(), HostError> {
        self.nodes = self.nodes.checked_add(1).ok_or_else(exceeded)?;
        self.bytes = self.bytes.checked_add(bytes).ok_or_else(exceeded)?;
        if depth > self.limits.max_depth
            || self.nodes > self.limits.max_nodes
            || self.bytes > self.max_bytes
        {
            return Err(exceeded());
        }
        Ok(())
    }
    pub(crate) fn host(&mut self, value: &HostValue, depth: usize) -> Result<(), HostError> {
        self.add(depth, std::mem::size_of::<HostValue>())?;
        match value {
            HostValue::String(s) => self.add(depth, s.len())?,
            HostValue::Bytes(s) => self.add(depth, s.len())?,
            HostValue::List(values) => {
                for value in values {
                    self.host(value, depth + 1)?;
                }
            }
            HostValue::Map(values) => {
                for (k, v) in values {
                    self.host(k, depth + 1)?;
                    self.host(v, depth + 1)?;
                }
            }
            HostValue::Record(values) => {
                for (k, v) in values {
                    self.add(depth, k.len())?;
                    self.host(v, depth + 1)?;
                }
            }
            HostValue::Variant { name, fields } => {
                self.add(depth, name.len())?;
                for v in fields {
                    self.host(v, depth + 1)?;
                }
            }
            HostValue::Json(value) => self.json(value, depth + 1)?,
            _ => {}
        }
        Ok(())
    }
    fn json(&mut self, value: &HostJsonValue, depth: usize) -> Result<(), HostError> {
        self.add(depth, std::mem::size_of::<HostJsonValue>())?;
        match value {
            HostJsonValue::String(s) => self.add(depth, s.len())?,
            HostJsonValue::Array(values) => {
                for v in values {
                    self.json(v, depth + 1)?;
                }
            }
            HostJsonValue::Object(values) => {
                for (k, v) in values {
                    self.add(depth, k.len())?;
                    self.json(v, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
pub(crate) fn exceeded() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "storage resource limit exceeded",
    )
}
