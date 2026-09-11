use super::*;
use crate::memory::query::validate_query_embedding;
impl MemoryDatabase<'_> {
    pub(super) fn query(
        &mut self,
        store: &StoreRef,
        query: MemoryQuery,
        limit: Option<u32>,
    ) -> Result<MemoryResult, HostError> {
        if !query.order_by.is_empty() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "ordered memory query is unsupported",
            ));
        }
        self.select(store, None, limit, query.predicate, None)
    }
    pub(super) fn vector_search(
        &mut self,
        store: &StoreRef,
        embedding: Vec<f32>,
        limit: u32,
        filter: Option<HostValue>,
    ) -> Result<MemoryResult, HostError> {
        validate_query_embedding(&embedding)?;
        self.select(store, None, Some(limit), filter, Some(embedding))
    }
}
