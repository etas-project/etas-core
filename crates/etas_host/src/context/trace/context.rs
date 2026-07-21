etas_core::id_type!(TraceId);
etas_core::id_type!(TraceSpanId);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceContext {
    pub trace_id: TraceId,
    pub parent_span: Option<TraceSpanId>,
}

impl TraceContext {
    pub fn root(trace_id: TraceId) -> Self {
        Self {
            trace_id,
            parent_span: None,
        }
    }
}
