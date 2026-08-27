use crate::{
    ActionArgPattern, ActionPattern, ApprovalRequest, HostActionGrant, HostTraceFieldSensitivity,
    HostTracePayload, HostValue, PolicyEvaluationRequest,
};

use super::{HostTraceRequest, record, strings, variant};

impl HostTraceRequest for PolicyEvaluationRequest {
    fn trace_payload(&self) -> HostTracePayload {
        HostTracePayload::new("policy", "Policy.evaluate")
            .with_field(
                "policy_ref",
                self.policy_ref.clone(),
                HostTraceFieldSensitivity::Sensitive,
            )
            .with_field(
                "subject",
                record([
                    ("kind", HostValue::String(self.subject.kind.clone())),
                    (
                        "attributes",
                        HostValue::Record(self.subject.attributes.clone()),
                    ),
                ]),
                HostTraceFieldSensitivity::Sensitive,
            )
    }
}

impl HostTraceRequest for ApprovalRequest {
    fn trace_payload(&self) -> HostTracePayload {
        HostTracePayload::new("approval", "Approval.request")
            .with_field(
                "reason",
                HostValue::String(self.reason.clone()),
                HostTraceFieldSensitivity::Sensitive,
            )
            .with_field(
                "requested_grants",
                HostValue::List(self.requested_grants.iter().map(grant).collect()),
                HostTraceFieldSensitivity::Sensitive,
            )
    }
}

fn grant(grant: &HostActionGrant) -> HostValue {
    match grant {
        HostActionGrant::Allow(pattern) => variant("Allow", vec![action_pattern(pattern)]),
    }
}

fn action_pattern(pattern: &ActionPattern) -> HostValue {
    match pattern {
        ActionPattern::Exact(action) => variant(
            "Exact",
            vec![record([
                ("effect", HostValue::String(action.effect.clone())),
                ("action", HostValue::String(action.action.clone())),
                ("args", HostValue::List(action.args.clone())),
            ])],
        ),
        ActionPattern::Pattern {
            effect,
            action,
            args,
        } => variant(
            "Pattern",
            vec![record([
                ("effect", HostValue::String(effect.clone())),
                ("action", HostValue::String(action.clone())),
                (
                    "args",
                    HostValue::List(args.iter().map(arg_pattern).collect()),
                ),
            ])],
        ),
    }
}

fn arg_pattern(pattern: &ActionArgPattern) -> HostValue {
    match pattern {
        ActionArgPattern::Any => variant("Any", Vec::new()),
        ActionArgPattern::Exact(value) => variant("Exact", vec![value.clone()]),
        ActionArgPattern::Prefix(parts) => variant("Prefix", vec![strings(parts)]),
    }
}
