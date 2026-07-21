use crate::{HostError, HostErrorCode, HostValue};

use super::{SessionMessage, SessionMessageRole, SessionRef};

#[derive(Clone, Debug, PartialEq)]
pub struct MessageEnvelope {
    pub id: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub role: SessionMessageRole,
    pub session: Option<SessionRef>,
    pub created_at: String,
    pub payload: HostValue,
    pub provenance: Option<HostValue>,
}

pub fn message_envelope_to_host_value(message: &MessageEnvelope) -> HostValue {
    HostValue::Record(message_envelope_fields(message))
}

fn message_envelope_fields(message: &MessageEnvelope) -> Vec<(String, HostValue)> {
    vec![
        ("id".to_owned(), HostValue::String(message.id.clone())),
        ("from".to_owned(), optional_string(message.from.as_deref())),
        ("to".to_owned(), optional_string(message.to.as_deref())),
        (
            "role".to_owned(),
            HostValue::String(role_name(message.role).to_owned()),
        ),
        (
            "session".to_owned(),
            optional_string(message.session.as_ref().map(|session| session.id.as_str())),
        ),
        (
            "created_at".to_owned(),
            HostValue::String(message.created_at.clone()),
        ),
        ("payload".to_owned(), message.payload.clone()),
        (
            "provenance".to_owned(),
            optional_value(message.provenance.clone()),
        ),
    ]
}

pub fn message_envelope_from_host_value(value: HostValue) -> Result<MessageEnvelope, HostError> {
    let HostValue::Record(mut fields) = value else {
        return Err(schema_error("Message envelope must be a host record"));
    };
    let id = required_string(&mut fields, "id")?;
    let from = optional_string_field(&mut fields, "from")?;
    let to = optional_string_field(&mut fields, "to")?;
    let role = parse_role(&required_string(&mut fields, "role")?)?;
    let session = optional_string_field(&mut fields, "session")?.map(|id| SessionRef { id });
    let created_at = required_string(&mut fields, "created_at")?;
    let payload = take_required(&mut fields, "payload")?;
    let provenance = optional_value_field(&mut fields, "provenance")?;
    if let Some((name, _)) = fields.into_iter().next() {
        return Err(schema_error(format!(
            "Message envelope contains unknown field `{name}`"
        )));
    }
    Ok(MessageEnvelope {
        id,
        from,
        to,
        role,
        session,
        created_at,
        payload,
        provenance,
    })
}

pub fn session_message_to_host_value(message: &SessionMessage) -> HostValue {
    let mut fields = message_envelope_fields(&MessageEnvelope {
        id: message.id.clone(),
        from: message.from.clone(),
        to: message.to.clone(),
        role: message.role,
        session: Some(message.session.clone()),
        created_at: message.created_at.clone(),
        payload: message.payload.clone(),
        provenance: message.provenance.clone(),
    });
    fields.push((
        "dedup_key".to_owned(),
        optional_string(message.dedup_key.as_deref()),
    ));
    HostValue::Record(fields)
}

pub fn session_message_from_host_value(value: HostValue) -> Result<SessionMessage, HostError> {
    let HostValue::Record(mut fields) = value else {
        return Err(schema_error(
            "SessionMessage envelope must be a host record",
        ));
    };
    let dedup_key = optional_string_field(&mut fields, "dedup_key")?;
    let message = message_envelope_from_host_value(HostValue::Record(fields))?;
    let Some(session) = message.session else {
        return Err(schema_error("SessionMessage envelope is missing `session`"));
    };
    Ok(SessionMessage {
        id: message.id,
        from: message.from,
        to: message.to,
        role: message.role,
        session,
        created_at: message.created_at,
        payload: message.payload,
        provenance: message.provenance,
        dedup_key,
    })
}

fn take_required(
    fields: &mut Vec<(String, HostValue)>,
    name: &str,
) -> Result<HostValue, HostError> {
    let Some(index) = fields.iter().position(|(field, _)| field == name) else {
        return Err(schema_error(format!(
            "Message envelope is missing `{name}`"
        )));
    };
    Ok(fields.remove(index).1)
}

fn required_string(fields: &mut Vec<(String, HostValue)>, name: &str) -> Result<String, HostError> {
    match take_required(fields, name)? {
        HostValue::String(value) => Ok(value),
        _ => Err(schema_error(format!(
            "Message envelope field `{name}` must be a string"
        ))),
    }
}

fn optional_string_field(
    fields: &mut Vec<(String, HostValue)>,
    name: &str,
) -> Result<Option<String>, HostError> {
    match optional_value_field(fields, name)? {
        Some(HostValue::String(value)) => Ok(Some(value)),
        Some(_) => Err(schema_error(format!(
            "Message envelope field `{name}` must contain a string"
        ))),
        None => Ok(None),
    }
}

fn optional_value_field(
    fields: &mut Vec<(String, HostValue)>,
    name: &str,
) -> Result<Option<HostValue>, HostError> {
    match take_required(fields, name)? {
        HostValue::Variant { name, fields } if name == "None" && fields.is_empty() => Ok(None),
        HostValue::Variant { name, mut fields } if name == "Some" && fields.len() == 1 => {
            Ok(fields.pop())
        }
        _ => Err(schema_error(format!(
            "Message envelope field `{name}` must be an Option value"
        ))),
    }
}

fn optional_string(value: Option<&str>) -> HostValue {
    optional_value(value.map(|value| HostValue::String(value.to_owned())))
}

fn optional_value(value: Option<HostValue>) -> HostValue {
    match value {
        Some(value) => HostValue::Variant {
            name: "Some".to_owned(),
            fields: vec![value],
        },
        None => HostValue::Variant {
            name: "None".to_owned(),
            fields: Vec::new(),
        },
    }
}

fn role_name(role: SessionMessageRole) -> &'static str {
    match role {
        SessionMessageRole::System => "system",
        SessionMessageRole::User => "user",
        SessionMessageRole::Assistant => "assistant",
        SessionMessageRole::Tool => "tool",
    }
}

fn parse_role(value: &str) -> Result<SessionMessageRole, HostError> {
    match value {
        "system" => Ok(SessionMessageRole::System),
        "user" => Ok(SessionMessageRole::User),
        "assistant" => Ok(SessionMessageRole::Assistant),
        "tool" => Ok(SessionMessageRole::Tool),
        _ => Err(schema_error(format!(
            "Message envelope contains unknown role `{value}`"
        ))),
    }
}

fn schema_error(message: impl Into<String>) -> HostError {
    HostError::new(HostErrorCode::SchemaMismatch, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_envelope_round_trips_without_synthesizing_fields() {
        let message = SessionMessage {
            id: "message-1".to_owned(),
            from: Some("agent-a".to_owned()),
            to: Some("agent-b".to_owned()),
            role: SessionMessageRole::Assistant,
            session: SessionRef {
                id: "session-1".to_owned(),
            },
            created_at: "2026-07-18T00:00:00Z".to_owned(),
            payload: HostValue::Float(1.5),
            provenance: Some(HostValue::String("trace-1".to_owned())),
            dedup_key: Some("dedup-1".to_owned()),
        };
        let decoded = session_message_from_host_value(session_message_to_host_value(&message))
            .expect("valid envelope");
        assert_eq!(decoded, message);
    }

    #[test]
    fn message_envelope_rejects_missing_runtime_identity() {
        let error = session_message_from_host_value(HostValue::Record(vec![]))
            .expect_err("missing envelope fields must fail closed");
        assert_eq!(error.code, HostErrorCode::SchemaMismatch);
    }

    #[test]
    fn generic_message_envelope_round_trips_without_session() {
        let message = MessageEnvelope {
            id: "message-2".to_owned(),
            from: None,
            to: None,
            role: SessionMessageRole::User,
            session: None,
            created_at: "2026-07-18T00:00:01Z".to_owned(),
            payload: HostValue::String("hello".to_owned()),
            provenance: None,
        };
        let decoded = message_envelope_from_host_value(message_envelope_to_host_value(&message))
            .expect("generic Message envelope should not require a session");
        assert_eq!(decoded, message);
    }

    #[test]
    fn generic_message_envelope_rejects_every_missing_or_protocol_only_field() {
        let message = MessageEnvelope {
            id: "message-3".to_owned(),
            from: None,
            to: None,
            role: SessionMessageRole::User,
            session: None,
            created_at: "2026-07-18T00:00:02Z".to_owned(),
            payload: HostValue::String("payload".to_owned()),
            provenance: None,
        };
        let HostValue::Record(fields) = message_envelope_to_host_value(&message) else {
            panic!("message envelope must encode as a record");
        };
        for required in [
            "id",
            "from",
            "to",
            "role",
            "session",
            "created_at",
            "payload",
            "provenance",
        ] {
            let damaged = fields
                .iter()
                .filter(|(name, _)| name != required)
                .cloned()
                .collect();
            let error = message_envelope_from_host_value(HostValue::Record(damaged))
                .expect_err("missing message field must fail closed");
            assert!(error.message.contains(required), "{error:?}");
        }

        let mut damaged = fields;
        damaged.push((
            "dedup_key".to_owned(),
            HostValue::String("protocol-only".to_owned()),
        ));
        let error = message_envelope_from_host_value(HostValue::Record(damaged))
            .expect_err("session protocol fields must not enter MessageEnvelope");
        assert!(error.message.contains("dedup_key"), "{error:?}");
    }
}
