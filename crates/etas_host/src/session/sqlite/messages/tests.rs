use super::*;
use crate::StorageLimits;

fn database(payload: &str) -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    super::super::schema::initialize_schema(&mut db).unwrap();
    super::super::append::apply_resolve(
        &db,
        crate::SessionConfig {
            id: "s".into(),
            context: ContextPolicy::All,
            retention: RetentionPolicy::Forever,
        },
        &StorageLimits::default(),
    )
    .unwrap();
    db.execute(
        "INSERT INTO session_messages(session_id,message_id,role,from_participant,to_participant,created_at,payload_json,provenance_json,dedup_key,ordinal) VALUES ('s','m','user',NULL,NULL,'0',?1,NULL,'d',0)",
        [payload],
    )
    .unwrap();
    db.execute(
        "UPDATE session_sequences SET last_ordinal=0 WHERE session_id='s'",
        [],
    )
    .unwrap();
    db
}

#[test]
fn single_message_read_respects_result_limit_for_id_and_dedup() {
    let payload = serde_json::json!({"kind": "string", "value": "x".repeat(256)}).to_string();
    let db = database(&payload);
    let limits = StorageLimits {
        max_value_bytes: 4096,
        max_result_bytes: 128,
        ..StorageLimits::default()
    };
    limits.validate().unwrap();
    for result in [
        select_message(&db, "s", "m", &limits),
        select_message_by_dedup(&db, "s", "d", &limits),
    ] {
        assert_eq!(result.unwrap_err().code, HostErrorCode::BudgetExceeded);
    }
}

#[test]
fn all_message_read_paths_reject_over_budget_row_before_payload_decode() {
    // The stored payload is deliberately malformed: quota admission must fail
    // before decoding, not allocate the row and report a JSON schema error.
    let mut db = database(&"!".repeat(256));
    let limits = StorageLimits {
        max_value_bytes: 4096,
        max_result_bytes: 128,
        ..StorageLimits::default()
    };
    for result in [
        select_message(&db, "s", "m", &limits).map(|_| ()),
        select_message_by_dedup(&db, "s", "d", &limits).map(|_| ()),
        read_page(&mut db, &limits).map(|_| ()),
    ] {
        assert_eq!(result.unwrap_err().code, HostErrorCode::BudgetExceeded);
    }
}

#[test]
fn history_debits_previous_rows_before_decoding_next_payload() {
    let payload = serde_json::json!({"kind": "string", "value": "x".repeat(256)}).to_string();
    let mut db = database(&payload);
    let defaults = StorageLimits::default();
    let first = select_message(&db, "s", "m", &defaults).unwrap().unwrap();
    db.execute(
        "INSERT INTO session_messages(session_id,message_id,role,from_participant,to_participant,created_at,payload_json,provenance_json,dedup_key,ordinal) VALUES ('s','next','user',NULL,NULL,'0',?1,NULL,NULL,1)",
        ["!".repeat(256)],
    )
    .unwrap();
    db.execute(
        "UPDATE session_sequences SET last_ordinal=1 WHERE session_id='s'",
        [],
    )
    .unwrap();
    let selection = crate::session::paging::HistoryCursor::new(
        "s",
        &ContextPolicy::All,
        1,
        &RetentionPolicy::Forever,
    )
    .unwrap();
    let state = super::super::fence::state(&db, "s").unwrap();
    let fence_bytes = crate::session::SessionHistoryFence::issue(&selection, &state, &defaults)
        .unwrap()
        .as_token()
        .len();
    let limits = StorageLimits {
        max_result_bytes: first.storage_size(&defaults).unwrap() + fence_bytes + 128,
        ..defaults
    };
    assert_eq!(
        read_page(&mut db, &limits).unwrap_err().code,
        HostErrorCode::BudgetExceeded,
    );
}

#[test]
fn all_message_read_paths_accept_limits_above_defaults() {
    let text = "x".repeat(StorageLimits::default().max_value_bytes + 128);
    let payload = serde_json::json!({"kind": "string", "value": text}).to_string();
    let mut db = database(&payload);
    let limits = StorageLimits {
        max_value_bytes: 2 * 1024 * 1024,
        max_result_bytes: 3 * 1024 * 1024,
        ..StorageLimits::default()
    };
    limits.validate().unwrap();
    for message in [
        select_message(&db, "s", "m", &limits).unwrap().unwrap(),
        select_message_by_dedup(&db, "s", "d", &limits)
            .unwrap()
            .unwrap(),
        read_page(&mut db, &limits).unwrap().remove(0),
    ] {
        assert_eq!(message.payload, crate::HostValue::String(text.clone()));
    }
}

fn read_page(
    connection: &mut Connection,
    limits: &StorageLimits,
) -> Result<Vec<SessionMessage>, HostError> {
    let mut db = SessionDatabase {
        connection,
        operation: None,
        limits,
    };
    match super::super::paging::load(
        &mut db,
        SessionRef { id: "s".into() },
        ContextPolicy::All,
        None,
        Some(10),
    )? {
        SessionResult::History { messages, .. } => Ok(messages),
        _ => panic!("expected history"),
    }
}
