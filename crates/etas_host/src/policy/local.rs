use std::{future::Future, pin::Pin};

use crate::{
    ApprovalRequest, HostActionGrant, HostError, HostErrorCode, HostRequestId, HostValue,
    PolicyClient, PolicyDecision, PolicyEvaluationRequest, PolicyResponse,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DenyUnknownPolicyClient;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalStaticPolicyClient {
    default_decision: LocalPolicyDecision,
    rules: Vec<LocalPolicyRule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalPolicyRule {
    pub subject_kind: String,
    pub qualified_action: Option<String>,
    pub resource_prefix: Option<String>,
    pub decision: LocalPolicyDecision,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocalPolicyDecision {
    Allow,
    RequireApproval,
    #[default]
    Deny,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnsafeAllowAllLocalPolicyClient;

impl PolicyClient for DenyUnknownPolicyClient {
    type Error = HostError;
    type EvaluateFuture<'a> =
        Pin<Box<dyn Future<Output = Result<PolicyResponse, Self::Error>> + Send + 'a>>;

    fn evaluate(&self, request: PolicyEvaluationRequest) -> Self::EvaluateFuture<'_> {
        Box::pin(async move {
            Ok(PolicyResponse {
                id: request.id,
                decision: PolicyDecision::Deny {
                    reason: "CLI policy provider denies unknown policy references".to_owned(),
                },
            })
        })
    }
}

impl LocalStaticPolicyClient {
    pub fn deny_by_default(rules: Vec<LocalPolicyRule>) -> Self {
        Self {
            default_decision: LocalPolicyDecision::Deny,
            rules,
        }
    }

    pub fn new(default_decision: LocalPolicyDecision, rules: Vec<LocalPolicyRule>) -> Self {
        Self {
            default_decision,
            rules,
        }
    }
}

impl PolicyClient for LocalStaticPolicyClient {
    type Error = HostError;
    type EvaluateFuture<'a> =
        Pin<Box<dyn Future<Output = Result<PolicyResponse, Self::Error>> + Send + 'a>>;

    fn evaluate(&self, request: PolicyEvaluationRequest) -> Self::EvaluateFuture<'_> {
        let decision = self
            .rules
            .iter()
            .find(|rule| rule.matches(&request))
            .map(|rule| rule.decision)
            .unwrap_or(self.default_decision);
        Box::pin(async move {
            Ok(PolicyResponse {
                id: request.id,
                decision: decision.to_policy_decision(request),
            })
        })
    }
}

impl PolicyClient for UnsafeAllowAllLocalPolicyClient {
    type Error = HostError;
    type EvaluateFuture<'a> =
        Pin<Box<dyn Future<Output = Result<PolicyResponse, Self::Error>> + Send + 'a>>;

    fn evaluate(&self, request: PolicyEvaluationRequest) -> Self::EvaluateFuture<'_> {
        Box::pin(async move {
            if request.subject.kind.is_empty() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "policy evaluation subject kind is empty",
                ));
            }
            Ok(PolicyResponse {
                id: request.id,
                decision: PolicyDecision::Allow,
            })
        })
    }
}

impl LocalPolicyRule {
    pub fn new(subject_kind: impl Into<String>, decision: LocalPolicyDecision) -> Self {
        Self {
            subject_kind: subject_kind.into(),
            qualified_action: None,
            resource_prefix: None,
            decision,
        }
    }

    pub fn qualified_action(mut self, action: impl Into<String>) -> Self {
        self.qualified_action = Some(action.into());
        self
    }

    pub fn resource_prefix(mut self, resource_prefix: impl Into<String>) -> Self {
        self.resource_prefix = Some(resource_prefix.into());
        self
    }

    fn matches(&self, request: &PolicyEvaluationRequest) -> bool {
        self.subject_kind == request.subject.kind
            && self.qualified_action.as_deref().is_none_or(|expected| {
                subject_attr_string(request, "qualified_action").as_deref() == Some(expected)
            })
            && self.resource_prefix.as_deref().is_none_or(|prefix| {
                subject_attr_string(request, "resource")
                    .is_some_and(|resource| resource.starts_with(prefix))
            })
    }
}

impl LocalPolicyDecision {
    fn to_policy_decision(self, request: PolicyEvaluationRequest) -> PolicyDecision {
        match self {
            Self::Allow => PolicyDecision::Allow,
            Self::Deny => PolicyDecision::Deny {
                reason: format!(
                    "local static policy has no allow rule for {} boundary",
                    request.subject.kind
                ),
            },
            Self::RequireApproval => {
                let approval_id = HostRequestId(request.id.0.saturating_add(10_000_000));
                PolicyDecision::RequireApproval {
                    request: ApprovalRequest {
                        id: approval_id,
                        reason: format!(
                            "local static policy requires approval for {} boundary",
                            request.subject.kind
                        ),
                        requested_grants: approval_grants_for_subject(&request),
                        trace: request.trace,
                    },
                }
            }
        }
    }
}

fn subject_attr_string(request: &PolicyEvaluationRequest, name: &str) -> Option<String> {
    request.subject.attributes.iter().find_map(|(attr, value)| {
        (attr == name).then(|| match value {
            HostValue::String(text) => Some(text.clone()),
            _ => None,
        })?
    })
}

fn approval_grants_for_subject(request: &PolicyEvaluationRequest) -> Vec<HostActionGrant> {
    let Some(action) = subject_attr_string(request, "qualified_action") else {
        return Vec::new();
    };
    let Some((effect, action)) = action.split_once('.') else {
        return Vec::new();
    };
    vec![HostActionGrant::allow(effect, action)]
}

#[cfg(test)]
mod tests {
    use crate::{AuthorityContext, PolicySubject, TraceContext, TraceId};

    use super::*;

    #[tokio::test]
    async fn local_static_policy_defaults_to_deny() {
        let client = LocalStaticPolicyClient::deny_by_default(Vec::new());
        let response = client
            .evaluate(request("model", "Model.invoke", "gpt"))
            .await
            .unwrap();
        assert!(matches!(response.decision, PolicyDecision::Deny { .. }));
    }

    #[tokio::test]
    async fn local_static_policy_requires_approval_with_precise_grant() {
        let client = LocalStaticPolicyClient::deny_by_default(vec![LocalPolicyRule::new(
            "memory",
            LocalPolicyDecision::RequireApproval,
        )]);
        let response = client
            .evaluate(request("memory", "Memory.put", "store"))
            .await
            .unwrap();
        let PolicyDecision::RequireApproval { request } = response.decision else {
            panic!("expected approval decision");
        };
        assert_eq!(request.requested_grants.len(), 1);
    }

    #[tokio::test]
    async fn local_static_policy_matches_action_and_resource_prefix() {
        let client = LocalStaticPolicyClient::deny_by_default(vec![
            LocalPolicyRule::new("memory", LocalPolicyDecision::Allow)
                .qualified_action("Memory.put")
                .resource_prefix("project."),
        ]);
        let allowed = client
            .evaluate(request("memory", "Memory.put", "project.notes"))
            .await
            .unwrap();
        assert!(matches!(allowed.decision, PolicyDecision::Allow));
        let denied = client
            .evaluate(request("memory", "Memory.get", "project.notes"))
            .await
            .unwrap();
        assert!(matches!(denied.decision, PolicyDecision::Deny { .. }));
    }

    fn request(kind: &str, action: &str, resource: &str) -> PolicyEvaluationRequest {
        PolicyEvaluationRequest {
            id: HostRequestId(1),
            policy_ref: HostValue::String("local".to_owned()),
            subject: PolicySubject {
                kind: kind.to_owned(),
                attributes: vec![
                    (
                        "qualified_action".to_owned(),
                        HostValue::String(action.to_owned()),
                    ),
                    (
                        "resource".to_owned(),
                        HostValue::String(resource.to_owned()),
                    ),
                ],
            },
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
        }
    }
}
