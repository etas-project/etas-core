use crate::HostValue;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PolicyContext {
    pub active_trace_specs: Vec<String>,
    pub trace_spec_facts: Vec<HostValue>,
    pub labels: Vec<String>,
    pub boundary_policy_ref: Option<HostValue>,
}
