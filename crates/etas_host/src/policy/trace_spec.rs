use std::{future::Future, pin::Pin};

use crate::{
    ActionInstance, ActionPattern, ApprovalRequest, HostActionGrant, HostError, HostRequestId,
    HostValue, PolicyClient, PolicyDecision, PolicyEvaluationRequest, PolicyResponse,
};

pub const TRACE_SPEC_RUNTIME_REF: &str = "etas.trace-spec-runtime";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TraceSpecRuntimeClient;

impl PolicyClient for TraceSpecRuntimeClient {
    type Error = HostError;
    type EvaluateFuture<'a> =
        Pin<Box<dyn Future<Output = Result<PolicyResponse, Self::Error>> + Send + 'a>>;

    fn evaluate(&self, request: PolicyEvaluationRequest) -> Self::EvaluateFuture<'_> {
        Box::pin(async move {
            Ok(PolicyResponse {
                id: request.id,
                decision: trace_spec_decision(&request),
            })
        })
    }
}

fn trace_spec_decision(request: &PolicyEvaluationRequest) -> PolicyDecision {
    let facts = &request.authority.policy.trace_spec_facts;
    if facts.is_empty() {
        return PolicyDecision::Deny {
            reason: "trace spec runtime has no materialized trace spec facts".to_owned(),
        };
    }
    if let Some(label) = facts
        .iter()
        .filter(|fact| policy_fact_kind_value(fact) == Some("deny"))
        .find(|fact| policy_fact_matches_subject(fact, request))
        .and_then(policy_fact_target_label)
    {
        return PolicyDecision::Deny {
            reason: format!("trace spec deny clause `{label}` rejected the request"),
        };
    }

    let allow_facts = facts
        .iter()
        .filter(|fact| policy_fact_kind_value(fact) == Some("allow"))
        .collect::<Vec<_>>();
    let allowed = allow_facts
        .iter()
        .any(|fact| policy_fact_matches_subject(fact, request));
    if !allowed {
        let active = request.authority.policy.active_trace_specs.join(", ");
        let subject = subject_attr_string(request, "qualified_action")
            .unwrap_or_else(|| request.subject.kind.clone());
        return PolicyDecision::Deny {
            reason: if active.is_empty() {
                format!("trace spec runtime has no allow clause for {subject}")
            } else {
                format!("active trace specs [{active}] have no allow clause for {subject}")
            },
        };
    }

    if let Some(requirement) = facts
        .iter()
        .filter(|fact| policy_fact_kind_value(fact) == Some("require_before"))
        .find(|fact| require_before_target_matches_subject(fact, request))
    {
        if current_action_has_grant(request) {
            return PolicyDecision::Allow;
        }
        let Some(guard_label) = policy_fact_guard_label(requirement) else {
            return PolicyDecision::Deny {
                reason: "trace spec require-before fact is missing guard label".to_owned(),
            };
        };
        if !policy_label_terms(&guard_label)
            .iter()
            .any(|term| policy_label_term_is_approval_request(term))
        {
            let target =
                policy_fact_target_label(requirement).unwrap_or_else(|| "<unknown>".to_owned());
            return PolicyDecision::Deny {
                reason: format!(
                    "trace spec requirement `{guard_label} before {target}` cannot be satisfied by runtime approval mediation"
                ),
            };
        }
        let approval_id = HostRequestId(request.id.0.saturating_add(10_000_000));
        let subject = subject_attr_string(request, "qualified_action")
            .unwrap_or_else(|| request.subject.kind.clone());
        return PolicyDecision::RequireApproval {
            request: ApprovalRequest {
                id: approval_id,
                reason: format!("trace spec requires approval before {subject}"),
                requested_grants: current_action_grants(request),
                trace: request.trace.clone(),
            },
        };
    }

    PolicyDecision::Allow
}

fn require_before_target_matches_subject(
    fact: &HostValue,
    request: &PolicyEvaluationRequest,
) -> bool {
    if let Some(patterns) = policy_fact_field(fact, "target_patterns")
        && let HostValue::List(patterns) = patterns
    {
        return patterns
            .iter()
            .any(|pattern| policy_pattern_matches_subject(pattern, request));
    }
    policy_fact_target_label(fact)
        .as_deref()
        .is_some_and(|label| policy_label_matches_subject(label, request))
}

fn current_action_has_grant(request: &PolicyEvaluationRequest) -> bool {
    let Some(action) = current_action_instance(request) else {
        return false;
    };
    request
        .authority
        .grants
        .iter()
        .any(|grant| grant.allows(&action))
}

fn current_action_grants(request: &PolicyEvaluationRequest) -> Vec<HostActionGrant> {
    current_action_instance(request)
        .map(|action| vec![HostActionGrant::Allow(ActionPattern::Exact(action))])
        .unwrap_or_default()
}

fn current_action_instance(request: &PolicyEvaluationRequest) -> Option<ActionInstance> {
    let action = subject_attr_string(request, "qualified_action")?;
    let (effect, action) = action.split_once('.')?;
    let args = subject_attr_string(request, "resource")
        .map(HostValue::String)
        .into_iter()
        .collect();
    Some(ActionInstance::new(effect, action, args))
}

fn policy_fact_kind_value(fact: &HostValue) -> Option<&str> {
    policy_fact_field(fact, "kind").and_then(|value| match value {
        HostValue::String(value) => Some(value.as_str()),
        _ => None,
    })
}

fn policy_fact_target_label(fact: &HostValue) -> Option<String> {
    policy_fact_field(fact, "target_label").and_then(|value| match value {
        HostValue::String(value) => Some(value.clone()),
        _ => None,
    })
}

fn policy_fact_guard_label(fact: &HostValue) -> Option<String> {
    policy_fact_field(fact, "guard_label").and_then(|value| match value {
        HostValue::String(value) => Some(value.clone()),
        _ => None,
    })
}

fn policy_fact_matches_subject(fact: &HostValue, request: &PolicyEvaluationRequest) -> bool {
    if let Some(patterns) = policy_fact_field(fact, "target_patterns")
        && let HostValue::List(patterns) = patterns
    {
        return patterns
            .iter()
            .any(|pattern| policy_pattern_matches_subject(pattern, request));
    }
    policy_fact_target_label(fact)
        .as_deref()
        .is_some_and(|label| policy_label_matches_subject(label, request))
}

fn policy_pattern_matches_subject(pattern: &HostValue, request: &PolicyEvaluationRequest) -> bool {
    let HostValue::Record(fields) = pattern else {
        return false;
    };
    let Some(effect) = host_record_string(fields, "effect") else {
        return false;
    };
    let action = host_record_string(fields, "action");
    let Some(qualified_action) = subject_attr_string(request, "qualified_action") else {
        return false;
    };
    match action {
        Some(action) => {
            qualified_action == format!("{effect}.{action}")
                && policy_pattern_args_match_subject(fields, request)
        }
        None => {
            qualified_action
                .strip_prefix(&effect)
                .is_some_and(|suffix| suffix.starts_with('.'))
                && policy_pattern_args_match_subject(fields, request)
        }
    }
}

fn policy_pattern_args_match_subject(
    fields: &[(String, HostValue)],
    request: &PolicyEvaluationRequest,
) -> bool {
    let Some(args) = fields
        .iter()
        .find_map(|(name, value)| (name == "args").then_some(value))
    else {
        return true;
    };
    let HostValue::List(args) = args else {
        return false;
    };
    args.iter().all(|arg| match arg {
        HostValue::String(arg) => policy_label_arg_matches_subject(arg, request),
        _ => false,
    })
}

fn host_record_string(fields: &[(String, HostValue)], name: &str) -> Option<String> {
    fields.iter().find_map(|(field, value)| {
        (field == name).then(|| match value {
            HostValue::String(value) => Some(value.clone()),
            _ => None,
        })?
    })
}

fn policy_fact_field<'a>(fact: &'a HostValue, name: &str) -> Option<&'a HostValue> {
    let HostValue::Record(fields) = fact else {
        return None;
    };
    fields
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value))
}

fn policy_label_matches_subject(label: &str, request: &PolicyEvaluationRequest) -> bool {
    let Some(qualified_action) = subject_attr_string(request, "qualified_action") else {
        return false;
    };
    policy_label_terms(label)
        .iter()
        .any(|term| policy_label_term_matches_subject(term, &qualified_action, request))
}

fn policy_label_terms(label: &str) -> Vec<String> {
    let trimmed = label.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(trimmed);
    inner
        .split(',')
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn policy_label_term_matches_subject(
    term: &str,
    qualified_action: &str,
    request: &PolicyEvaluationRequest,
) -> bool {
    let (base, arg) = term
        .split_once('[')
        .map(|(base, rest)| (base.trim(), rest.trim_end_matches(']').trim()))
        .unwrap_or((term.trim(), ""));
    if base == qualified_action {
        return arg.is_empty() || policy_label_arg_matches_subject(arg, request);
    }
    if !base.contains('.') {
        return qualified_action
            .strip_prefix(base)
            .is_some_and(|suffix| suffix.starts_with('.'))
            && (arg.is_empty() || policy_label_arg_matches_subject(arg, request));
    }
    false
}

fn policy_label_term_is_approval_request(term: &str) -> bool {
    let base = term
        .split_once('[')
        .map(|(base, _)| base.trim())
        .unwrap_or_else(|| term.trim());
    base == "Approval" || base == "Approval.request"
}

fn policy_label_arg_matches_subject(arg: &str, request: &PolicyEvaluationRequest) -> bool {
    if arg.is_empty() || arg == "_" {
        return true;
    }
    let candidates = ["region", "resource", "tool", "model", "path", "program"];
    candidates.iter().any(|name| {
        subject_attr_string(request, name).is_some_and(|value| {
            value == arg
                || value.starts_with(&format!("{arg}:"))
                || value.starts_with(&format!("{arg}."))
        })
    })
}

fn subject_attr_string(request: &PolicyEvaluationRequest, name: &str) -> Option<String> {
    request.subject.attributes.iter().find_map(|(attr, value)| {
        (attr == name).then(|| match value {
            HostValue::String(text) => Some(text.clone()),
            _ => None,
        })?
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        AuthorityContext, HostActionGrant, HostRequestId, PolicyContext, PolicySubject,
        TraceContext, TraceId,
    };

    use super::*;

    #[tokio::test]
    async fn trace_spec_allows_matching_action() {
        let client = TraceSpecRuntimeClient;
        let response = client
            .evaluate(request(vec![allow("[Console]")]))
            .await
            .unwrap();
        assert!(matches!(response.decision, PolicyDecision::Allow));
    }

    #[tokio::test]
    async fn trace_spec_denies_without_matching_allow() {
        let client = TraceSpecRuntimeClient;
        let response = client
            .evaluate(request(vec![allow("[Memory.read]")]))
            .await
            .unwrap();
        assert!(matches!(response.decision, PolicyDecision::Deny { .. }));
    }

    #[tokio::test]
    async fn trace_spec_requires_approval_before_matching_target() {
        let client = TraceSpecRuntimeClient;
        let response = client
            .evaluate(request(vec![
                allow("[Console]"),
                require_before("Approval.request", "[Console]"),
            ]))
            .await
            .unwrap();
        let PolicyDecision::RequireApproval { request } = response.decision else {
            panic!("expected approval request");
        };
        assert_eq!(request.trace.trace_id, TraceId(7));
        assert_eq!(request.requested_grants.len(), 1);
    }

    #[tokio::test]
    async fn trace_spec_uses_existing_approval_grant() {
        let client = TraceSpecRuntimeClient;
        let mut request = request(vec![
            allow("[Console]"),
            require_before("Approval.request", "[Console]"),
        ]);
        request.authority.grants = vec![HostActionGrant::Allow(ActionPattern::Exact(
            ActionInstance::new(
                "Console",
                "stdout_write",
                vec![HostValue::String("stdio".to_owned())],
            ),
        ))];
        let response = client.evaluate(request).await.unwrap();
        assert!(matches!(response.decision, PolicyDecision::Allow));
    }

    #[tokio::test]
    async fn trace_spec_fails_closed_for_non_approval_runtime_guard() {
        let client = TraceSpecRuntimeClient;
        let response = client
            .evaluate(request(vec![
                allow("[Console]"),
                require_before("HumanReview", "[Console]"),
            ]))
            .await
            .unwrap();
        assert!(matches!(response.decision, PolicyDecision::Deny { .. }));
    }

    fn request(facts: Vec<HostValue>) -> PolicyEvaluationRequest {
        let mut authority = AuthorityContext::deny_all();
        authority.policy = PolicyContext {
            active_trace_specs: vec!["Gate".to_owned()],
            trace_spec_facts: facts,
            labels: Vec::new(),
            boundary_policy_ref: None,
        };
        PolicyEvaluationRequest {
            id: HostRequestId(1),
            policy_ref: HostValue::String(TRACE_SPEC_RUNTIME_REF.to_owned()),
            subject: PolicySubject {
                kind: "console".to_owned(),
                attributes: vec![
                    (
                        "qualified_action".to_owned(),
                        HostValue::String("Console.stdout_write".to_owned()),
                    ),
                    ("resource".to_owned(), HostValue::String("stdio".to_owned())),
                ],
            },
            authority,
            trace: TraceContext::root(TraceId(7)),
        }
    }

    fn allow(label: &str) -> HostValue {
        HostValue::Record(vec![
            ("kind".to_owned(), HostValue::String("allow".to_owned())),
            ("label".to_owned(), HostValue::String(label.to_owned())),
            (
                "target_label".to_owned(),
                HostValue::String(label.to_owned()),
            ),
            ("target_patterns".to_owned(), policy_label_patterns(label)),
        ])
    }

    fn require_before(guard: &str, target: &str) -> HostValue {
        HostValue::Record(vec![
            (
                "kind".to_owned(),
                HostValue::String("require_before".to_owned()),
            ),
            (
                "label".to_owned(),
                HostValue::String(format!("require before {guard} -> {target}")),
            ),
            (
                "guard_label".to_owned(),
                HostValue::String(guard.to_owned()),
            ),
            ("guard_patterns".to_owned(), policy_label_patterns(guard)),
            (
                "target_label".to_owned(),
                HostValue::String(target.to_owned()),
            ),
            ("target_patterns".to_owned(), policy_label_patterns(target)),
        ])
    }

    fn policy_label_patterns(label: &str) -> HostValue {
        HostValue::List(
            policy_label_terms(label)
                .into_iter()
                .filter_map(|term| {
                    let (base, arg) = term
                        .split_once('[')
                        .map(|(base, rest)| {
                            (
                                base.trim().to_owned(),
                                rest.trim_end_matches(']').trim().to_owned(),
                            )
                        })
                        .unwrap_or((term.trim().to_owned(), String::new()));
                    if base.is_empty() {
                        return None;
                    }
                    let mut fields = Vec::new();
                    if let Some((effect, action)) = base.split_once('.') {
                        fields.push(("effect".to_owned(), HostValue::String(effect.to_owned())));
                        fields.push(("action".to_owned(), HostValue::String(action.to_owned())));
                    } else {
                        fields.push(("effect".to_owned(), HostValue::String(base)));
                    }
                    if !arg.is_empty() {
                        fields.push((
                            "args".to_owned(),
                            HostValue::List(
                                arg.split(',')
                                    .map(str::trim)
                                    .filter(|arg| !arg.is_empty())
                                    .map(|arg| HostValue::String(arg.to_owned()))
                                    .collect(),
                            ),
                        ));
                    }
                    Some(HostValue::Record(fields))
                })
                .collect(),
        )
    }
}
