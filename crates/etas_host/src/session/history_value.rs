use super::{MessageEnvelope, SessionRef, SessionResult, message_envelope_to_host_value};
use crate::{HostError, HostErrorCode, HostValue, StorageLimits};

pub fn session_history_page_value(
    result: &SessionResult,
    expected: &SessionRef,
    limit: u32,
    limits: &StorageLimits,
) -> Result<HostValue, HostError> {
    limits.validate()?;
    let SessionResult::History {
        session,
        fence,
        published_context,
        messages,
        summary,
        cursor,
    } = result
    else {
        return Err(invalid("expected a session history page"));
    };
    if limit == 0
        || limit as usize > limits.max_page_entries
        || session != expected
        || fence.session_id() != expected.id
        || fence.as_token().len() > limits.max_value_bytes
        || messages.iter().any(|message| &message.session != expected)
        || messages.len() > limit as usize
        || messages.len() > limits.max_page_entries
        || cursor.as_ref().is_some_and(|cursor| {
            messages.is_empty() || cursor.opaque.len() > limits.max_value_bytes
        })
        || published_context
            .as_ref()
            .is_some_and(|context| context.fence.session_id() != expected.id)
    {
        return Err(invalid(
            "session page does not match the requested selection",
        ));
    }
    let mut bytes = fence.as_token().len();
    for size in messages
        .iter()
        .map(|message| message.storage_size(limits))
        .chain(
            published_context
                .iter()
                .map(|context| context.storage_size(limits)),
        )
        .chain(summary.iter().map(|summary| Ok(summary.text.len())))
        .chain(cursor.iter().map(|cursor| Ok(cursor.opaque.len())))
    {
        bytes = bytes.checked_add(size?).ok_or_else(exceeded)?;
    }
    if bytes > limits.max_result_bytes {
        return Err(exceeded());
    }
    let value = HostValue::Record(vec![
        ("fence".into(), opaque(fence.as_token())),
        (
            "messages".into(),
            HostValue::List(
                messages
                    .iter()
                    .map(|message| {
                        message_envelope_to_host_value(&MessageEnvelope {
                            id: message.id.clone(),
                            from: message.from.clone(),
                            to: message.to.clone(),
                            role: message.role,
                            session: Some(message.session.clone()),
                            created_at: message.created_at.clone(),
                            payload: message.payload.clone(),
                            provenance: message.provenance.clone(),
                        })
                    })
                    .collect(),
            ),
        ),
        (
            "published_context".into(),
            optional(
                published_context
                    .as_ref()
                    .map(|context| published_context_value(context, limits))
                    .transpose()?,
            ),
        ),
        (
            "summary".into(),
            optional(summary.as_ref().map(|summary| {
                HostValue::Record(vec![
                    ("text".into(), HostValue::String(summary.text.clone())),
                    (
                        "message_count".into(),
                        HostValue::UInt(summary.message_count as u128),
                    ),
                ])
            })),
        ),
        (
            "cursor".into(),
            optional(cursor.as_ref().map(|cursor| opaque(&cursor.opaque))),
        ),
    ]);
    limits.result_value_size(&value)?;
    Ok(value)
}

pub fn published_context_value(
    context: &super::SessionPublishedContext,
    limits: &StorageLimits,
) -> Result<HostValue, HostError> {
    context.storage_size(limits)?;
    let value = HostValue::Record(vec![
        (
            "content".into(),
            HostValue::Record(vec![
                (
                    "text".into(),
                    HostValue::String(context.content.text.clone()),
                ),
                (
                    "provenance".into(),
                    HostValue::Map(
                        context
                            .content
                            .provenance
                            .iter()
                            .map(|(k, v)| {
                                (HostValue::String(k.clone()), HostValue::String(v.clone()))
                            })
                            .collect(),
                    ),
                ),
            ]),
        ),
        ("fence".into(), opaque(context.fence.as_token())),
        ("version".into(), HostValue::UInt(context.version.into())),
    ]);
    limits.result_value_size(&value)?;
    Ok(value)
}

fn opaque(token: &str) -> HostValue {
    HostValue::Record(vec![("opaque".into(), HostValue::String(token.into()))])
}
fn optional(value: Option<HostValue>) -> HostValue {
    match value {
        Some(value) => HostValue::Variant {
            name: "Some".into(),
            fields: vec![value],
        },
        None => HostValue::Variant {
            name: "None".into(),
            fields: vec![],
        },
    }
}
fn invalid(message: &str) -> HostError {
    HostError::new(HostErrorCode::InvalidResponse, message)
}
fn exceeded() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "session page exceeds checked result limits",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AuthorityContext, ContextPolicy, HostRequestId, RetentionPolicy, SessionClient,
        SessionConfig, SessionOperation, SessionRequest, TraceContext, TraceId,
    };

    #[tokio::test(flavor = "current_thread")]
    async fn history_page_codec_rejects_foreign_identity_and_invalid_bounds() {
        let client = crate::InMemorySessionClient::new();
        let request = |operation| SessionRequest {
            id: HostRequestId(1),
            operation,
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        };
        client
            .execute(request(SessionOperation::Resolve {
                config: SessionConfig {
                    id: "s".into(),
                    context: ContextPolicy::All,
                    retention: RetentionPolicy::Forever,
                },
            }))
            .await
            .unwrap()
            .result
            .unwrap();
        let session = SessionRef { id: "s".into() };
        let page = client
            .execute(request(SessionOperation::Load {
                session: session.clone(),
                context: ContextPolicy::All,
                cursor: None,
                limit: Some(1),
            }))
            .await
            .unwrap()
            .result
            .unwrap();
        let limits = StorageLimits::default();
        assert!(session_history_page_value(&page, &session, 1, &limits).is_ok());
        assert!(
            session_history_page_value(
                &page,
                &SessionRef {
                    id: "foreign".into()
                },
                1,
                &limits
            )
            .is_err()
        );
        assert!(session_history_page_value(&page, &session, 0, &limits).is_err());
        for limits in [
            StorageLimits {
                max_result_bytes: 1,
                ..Default::default()
            },
            StorageLimits {
                max_nodes: 1,
                ..Default::default()
            },
            StorageLimits {
                max_value_bytes: 1,
                ..Default::default()
            },
        ] {
            assert!(session_history_page_value(&page, &session, 1, &limits).is_err());
        }
        let mut non_advancing = page;
        if let SessionResult::History { cursor, .. } = &mut non_advancing {
            *cursor = Some(crate::SessionCursor {
                opaque: "forged".into(),
            });
        }
        assert!(session_history_page_value(&non_advancing, &session, 1, &limits).is_err());
    }
}
