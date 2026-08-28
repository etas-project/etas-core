use std::{future::Future, pin::Pin};

use serde_json::{Value, json};

use crate::{
    ActionArgPattern, ActionInstance, ActionPattern, ApprovalRequest, AuthConfig, HostError,
    HostErrorCode, HostRequestId, HttpTransport, PolicyClient, PolicyDecision,
    PolicyEvaluationRequest, PolicyResponse, PrivateResolutionPolicy, RetryPolicy, TraceContext,
    host_json_to_value, host_value_to_json,
};

#[derive(Clone)]
pub struct HttpPolicyClient {
    transport: HttpTransport,
    path: String,
}

impl HttpPolicyClient {
    pub fn new(base_url: impl AsRef<str>) -> Result<Self, HostError> {
        Self::try_new_with_policy(base_url, PrivateResolutionPolicy::PublicOnly)
    }

    pub fn try_new_with_policy(
        base_url: impl AsRef<str>,
        policy: PrivateResolutionPolicy,
    ) -> Result<Self, HostError> {
        Ok(Self {
            transport: HttpTransport::try_new(base_url, policy)?,
            path: "/policy/evaluate".to_owned(),
        })
    }

    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.transport = self.transport.with_auth(auth);
        self
    }

    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.transport = self.transport.with_retry(retry);
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    async fn evaluate_request(
        &self,
        request: PolicyEvaluationRequest,
    ) -> Result<PolicyResponse, HostError> {
        let id = request.id;
        let body = encode_policy_request(&request)?.to_string();
        let response = self.transport.send_json(&self.path, body).await?;
        if !(200..300).contains(&response.status) {
            return Err(HostError::new(
                HostErrorCode::ProviderRejected,
                "HTTP policy provider returned an error status",
            )
            .with_detail("status", response.status.to_string()));
        }
        decode_policy_response(id, request.trace, &response.body)
    }
}

impl PolicyClient for HttpPolicyClient {
    type Error = HostError;
    type EvaluateFuture<'a> =
        Pin<Box<dyn Future<Output = Result<PolicyResponse, Self::Error>> + Send + 'a>>;

    fn evaluate(&self, request: PolicyEvaluationRequest) -> Self::EvaluateFuture<'_> {
        Box::pin(async move { self.evaluate_request(request).await })
    }
}

fn encode_policy_request(request: &PolicyEvaluationRequest) -> Result<Value, HostError> {
    let attributes = request
        .subject
        .attributes
        .iter()
        .map(|(name, value)| Ok(json!({ "name": name, "value": host_value_to_json(value)? })))
        .collect::<Result<Vec<_>, HostError>>()?;
    let grants = request
        .authority
        .grants
        .iter()
        .map(action_grant_json)
        .collect::<Result<Vec<_>, HostError>>()?;
    let active_trace_specs = request.authority.policy.active_trace_specs.clone();
    let trace_spec_facts = request
        .authority
        .policy
        .trace_spec_facts
        .iter()
        .map(host_value_to_json)
        .collect::<Result<Vec<_>, HostError>>()?;
    let labels = request.authority.policy.labels.clone();
    Ok(json!({
        "id": request.id.0,
        "policy_ref": host_value_to_json(&request.policy_ref)?,
        "subject": {
            "kind": request.subject.kind,
            "attributes": attributes,
        },
        "authority": {
            "grants": grants,
            "policy": {
                "active_trace_specs": active_trace_specs,
                "trace_spec_facts": trace_spec_facts,
                "labels": labels,
            },
        },
        "trace": {
            "trace_id": request.trace.trace_id.to_hex(),
            "parent_span": request.trace.parent_span.map(|span| span.0),
        },
    }))
}

fn action_grant_json(grant: &crate::HostActionGrant) -> Result<Value, HostError> {
    match grant {
        crate::HostActionGrant::Allow(pattern) => match pattern {
            ActionPattern::Exact(action) => {
                let args = action
                    .args
                    .iter()
                    .map(host_value_to_json)
                    .collect::<Result<Vec<_>, HostError>>()?;
                Ok(json!({
                "kind": "allow_exact",
                "effect": action.effect,
                "action": action.action,
                "args": args,
                }))
            }
            ActionPattern::Pattern {
                effect,
                action,
                args,
            } => {
                let args = args
                    .iter()
                    .map(action_arg_pattern_json)
                    .collect::<Result<Vec<_>, HostError>>()?;
                Ok(json!({
                "kind": "allow_pattern",
                "effect": effect,
                "action": action,
                "args": args,
                }))
            }
        },
    }
}

fn action_arg_pattern_json(pattern: &crate::ActionArgPattern) -> Result<Value, HostError> {
    match pattern {
        crate::ActionArgPattern::Any => Ok(json!({ "kind": "any" })),
        crate::ActionArgPattern::Exact(value) => Ok(json!({
            "kind": "exact",
            "value": host_value_to_json(value)?,
        })),
        crate::ActionArgPattern::Prefix(parts) => Ok(json!({
            "kind": "prefix",
            "parts": parts,
        })),
    }
}

fn decode_policy_response(
    id: HostRequestId,
    trace: TraceContext,
    body: &str,
) -> Result<PolicyResponse, HostError> {
    let value = serde_json::from_str::<Value>(body).map_err(|error| {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "HTTP policy response is not valid JSON",
        )
        .with_detail("error", error.to_string())
    })?;
    let decision = value
        .get("decision")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "HTTP policy response is missing string decision",
            )
        })?;
    let decision = match decision {
        "allow" => PolicyDecision::Allow,
        "deny" => {
            let reason = value
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("HTTP policy provider denied request")
                .to_owned();
            PolicyDecision::Deny { reason }
        }
        "require_approval" => {
            let request = value.get("approval").ok_or_else(|| {
                HostError::new(
                    HostErrorCode::InvalidResponse,
                    "HTTP policy require_approval response is missing approval object",
                )
            })?;
            PolicyDecision::RequireApproval {
                request: decode_approval_request(request, trace)?,
            }
        }
        other => {
            return Err(HostError::new(
                HostErrorCode::InvalidResponse,
                "HTTP policy response has unknown decision",
            )
            .with_detail("decision", other));
        }
    };
    Ok(PolicyResponse { id, decision })
}

fn decode_approval_request(
    value: &Value,
    trace: TraceContext,
) -> Result<ApprovalRequest, HostError> {
    let id = value.get("id").and_then(Value::as_u64).ok_or_else(|| {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "HTTP policy approval object is missing numeric id",
        )
    })?;
    let Ok(id) = u32::try_from(id) else {
        return Err(HostError::new(
            HostErrorCode::InvalidResponse,
            "HTTP policy approval id is outside u32 range",
        ));
    };
    let reason = value
        .get("reason")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "HTTP policy approval object is missing reason",
            )
        })?
        .to_owned();
    let requested_grants = value
        .get("requested_grants")
        .and_then(Value::as_array)
        .map(|grants| {
            grants
                .iter()
                .map(decode_approval_grant)
                .collect::<Result<Vec<_>, HostError>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(ApprovalRequest {
        id: HostRequestId(id),
        reason,
        requested_grants,
        trace,
    })
}

fn decode_approval_grant(value: &Value) -> Result<crate::HostActionGrant, HostError> {
    let kind = value.get("kind").and_then(Value::as_str).ok_or_else(|| {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "HTTP policy approval grant is missing kind",
        )
    })?;
    let effect = value.get("effect").and_then(Value::as_str).ok_or_else(|| {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "HTTP policy approval grant is missing effect",
        )
    })?;
    let action = value.get("action").and_then(Value::as_str).ok_or_else(|| {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "HTTP policy approval grant is missing action",
        )
    })?;
    match kind {
        "allow_exact" => Ok(crate::HostActionGrant::Allow(ActionPattern::Exact(
            ActionInstance::new(effect, action, decode_host_value_args(value)?),
        ))),
        "allow_pattern" => Ok(crate::HostActionGrant::Allow(ActionPattern::Pattern {
            effect: effect.to_owned(),
            action: action.to_owned(),
            args: decode_action_arg_patterns(value)?,
        })),
        other => Err(HostError::new(
            HostErrorCode::InvalidResponse,
            "HTTP policy approval grant has unknown kind",
        )
        .with_detail("kind", other)),
    }
}

fn decode_host_value_args(value: &Value) -> Result<Vec<crate::HostValue>, HostError> {
    value
        .get("args")
        .and_then(Value::as_array)
        .map(|args| {
            args.iter()
                .cloned()
                .map(host_json_to_value)
                .collect::<Result<Vec<_>, HostError>>()
        })
        .transpose()
        .map(Option::unwrap_or_default)
}

fn decode_action_arg_patterns(value: &Value) -> Result<Vec<ActionArgPattern>, HostError> {
    value
        .get("args")
        .and_then(Value::as_array)
        .map(|args| {
            args.iter()
                .map(decode_action_arg_pattern)
                .collect::<Result<Vec<_>, HostError>>()
        })
        .transpose()
        .map(Option::unwrap_or_default)
}

fn decode_action_arg_pattern(value: &Value) -> Result<ActionArgPattern, HostError> {
    let kind = value.get("kind").and_then(Value::as_str).ok_or_else(|| {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "HTTP policy approval grant arg is missing kind",
        )
    })?;
    match kind {
        "any" => Ok(ActionArgPattern::Any),
        "exact" => {
            let value = value.get("value").ok_or_else(|| {
                HostError::new(
                    HostErrorCode::InvalidResponse,
                    "HTTP policy exact approval grant arg is missing value",
                )
            })?;
            host_json_to_value(value.clone()).map(ActionArgPattern::Exact)
        }
        "prefix" => {
            let parts = value
                .get("parts")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    HostError::new(
                        HostErrorCode::InvalidResponse,
                        "HTTP policy prefix approval grant arg is missing parts",
                    )
                })?
                .iter()
                .map(|part| {
                    part.as_str().map(ToOwned::to_owned).ok_or_else(|| {
                        HostError::new(
                            HostErrorCode::InvalidResponse,
                            "HTTP policy prefix approval grant arg parts must be strings",
                        )
                    })
                })
                .collect::<Result<Vec<_>, HostError>>()?;
            Ok(ActionArgPattern::Prefix(parts))
        }
        other => Err(HostError::new(
            HostErrorCode::InvalidResponse,
            "HTTP policy approval grant arg has unknown kind",
        )
        .with_detail("kind", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_policy_response_rejects_unknown_decision() {
        let err = decode_policy_response(
            HostRequestId(1),
            TraceContext::root(crate::TraceId(1)),
            r#"{"decision":"maybe"}"#,
        )
        .expect_err("unknown policy decision must fail closed");
        assert_eq!(err.code, HostErrorCode::InvalidResponse);
    }

    #[test]
    fn decode_policy_response_accepts_deny_reason() {
        let response = decode_policy_response(
            HostRequestId(7),
            TraceContext::root(crate::TraceId(7)),
            r#"{"decision":"deny","reason":"no"}"#,
        )
        .expect("deny response should decode");
        assert_eq!(response.id, HostRequestId(7));
        assert_eq!(
            response.decision,
            PolicyDecision::Deny {
                reason: "no".to_owned()
            }
        );
    }

    #[test]
    fn decode_policy_response_preserves_approval_trace_and_grant_args() {
        let response = decode_policy_response(
            HostRequestId(11),
            TraceContext {
                trace_id: crate::TraceId(99),
                parent_trace: None,
                parent_span: Some(crate::TraceSpanId(3)),
            },
            r#"{
                "decision":"require_approval",
                "approval":{
                    "id":42,
                    "reason":"needs grant",
                    "requested_grants":[{
                        "kind":"allow_pattern",
                        "effect":"Memory",
                        "action":"write",
                        "args":[
                            {"kind":"prefix","parts":["ProjectMemory"]},
                            {"kind":"exact","value":"draft"}
                        ]
                    }]
                }
            }"#,
        )
        .expect("approval response should decode");
        let PolicyDecision::RequireApproval { request } = response.decision else {
            panic!("expected approval decision");
        };
        assert_eq!(request.trace.trace_id, crate::TraceId(99));
        assert_eq!(request.trace.parent_span, Some(crate::TraceSpanId(3)));
        assert_eq!(
            request.requested_grants,
            vec![crate::HostActionGrant::Allow(ActionPattern::Pattern {
                effect: "Memory".to_owned(),
                action: "write".to_owned(),
                args: vec![
                    ActionArgPattern::Prefix(vec!["ProjectMemory".to_owned()]),
                    ActionArgPattern::Exact(crate::HostValue::Json(crate::HostJsonValue::String(
                        "draft".to_owned()
                    ))),
                ],
            })]
        );
    }

    #[test]
    fn decode_policy_response_rejects_approval_grant_without_kind() {
        let err = decode_policy_response(
            HostRequestId(12),
            TraceContext::root(crate::TraceId(12)),
            r#"{
                "decision":"require_approval",
                "approval":{
                    "id":42,
                    "reason":"needs grant",
                    "requested_grants":[{
                        "effect":"Memory",
                        "action":"write",
                        "args":[{"kind":"any"}]
                    }]
                }
            }"#,
        )
        .expect_err("approval grant kind must be explicit");
        assert_eq!(err.code, HostErrorCode::InvalidResponse);
        assert!(
            err.message.contains("approval grant is missing kind"),
            "{}",
            err.message
        );
    }

    #[test]
    fn decode_policy_response_rejects_approval_grant_arg_without_kind() {
        let err = decode_policy_response(
            HostRequestId(13),
            TraceContext::root(crate::TraceId(13)),
            r#"{
                "decision":"require_approval",
                "approval":{
                    "id":42,
                    "reason":"needs grant",
                    "requested_grants":[{
                        "kind":"allow_pattern",
                        "effect":"Memory",
                        "action":"write",
                        "args":["draft"]
                    }]
                }
            }"#,
        )
        .expect_err("approval grant arg kind must be explicit");
        assert_eq!(err.code, HostErrorCode::InvalidResponse);
        assert!(
            err.message.contains("approval grant arg is missing kind"),
            "{}",
            err.message
        );
    }
}
