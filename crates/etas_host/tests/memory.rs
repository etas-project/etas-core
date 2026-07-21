use etas_host::{
    AuthorityContext, HostErrorCode, HostRequestId, HostValue, InMemoryMemoryClient, MemoryClient,
    MemoryOperation, MemoryOrderKey, MemoryQuery, MemoryRegionRef, MemoryRequest, MemoryResult,
    MemoryVersion, MemoryWriteMode, SqliteMemoryClient, StoreRef, TestWorkspace, TraceContext,
    TraceId,
};

fn store() -> StoreRef {
    StoreRef {
        region: MemoryRegionRef {
            stable_id: "project_memory".to_owned(),
            schema_fingerprint: Some("schema-v1".to_owned()),
        },
        path: vec!["Notes".to_owned()],
    }
}

fn request(id: u32, operation: MemoryOperation) -> MemoryRequest {
    MemoryRequest {
        id: HostRequestId(id),
        store: store(),
        operation,
        authority: AuthorityContext::deny_all(),
        trace: TraceContext::root(TraceId(1)),
        budget: Default::default(),
    }
}

#[tokio::test]
async fn sqlite_memory_persists_values_across_clients() {
    let workspace = TestWorkspace::create("sqlite-memory-persist").expect("workspace");
    let db = workspace.path().join("memory.sqlite");
    let first = SqliteMemoryClient::open(&db).expect("sqlite memory should open");
    let write = first
        .execute(request(
            1,
            MemoryOperation::Put {
                key: HostValue::String("draft".to_owned()),
                value: HostValue::Record(vec![(
                    "summary".to_owned(),
                    HostValue::String("stored".to_owned()),
                )]),
                expected: None,
                mode: MemoryWriteMode::Put,
            },
        ))
        .await
        .expect("write should execute")
        .result
        .expect("write should succeed");
    assert_eq!(
        write,
        MemoryResult::Written {
            version: MemoryVersion {
                opaque: "1".to_owned()
            }
        }
    );

    let second = SqliteMemoryClient::open(&db).expect("sqlite memory should reopen");
    let read = second
        .execute(request(
            2,
            MemoryOperation::Get {
                key: HostValue::String("draft".to_owned()),
            },
        ))
        .await
        .expect("read should execute")
        .result
        .expect("read should succeed");
    assert_eq!(
        read,
        MemoryResult::Value {
            value: HostValue::Record(vec![(
                "summary".to_owned(),
                HostValue::String("stored".to_owned())
            )]),
            version: MemoryVersion {
                opaque: "1".to_owned()
            },
        }
    );
}

#[tokio::test]
async fn sqlite_memory_reports_optimistic_version_conflict() {
    let workspace = TestWorkspace::create("sqlite-memory-conflict").expect("workspace");
    let client =
        SqliteMemoryClient::open(workspace.path().join("memory.sqlite")).expect("sqlite memory");
    client
        .execute(request(
            1,
            MemoryOperation::Put {
                key: HostValue::String("draft".to_owned()),
                value: HostValue::String("v1".to_owned()),
                expected: None,
                mode: MemoryWriteMode::Put,
            },
        ))
        .await
        .expect("write should execute")
        .result
        .expect("write should succeed");
    let conflict = client
        .execute(request(
            2,
            MemoryOperation::Put {
                key: HostValue::String("draft".to_owned()),
                value: HostValue::String("v2".to_owned()),
                expected: Some(MemoryVersion {
                    opaque: "9".to_owned(),
                }),
                mode: MemoryWriteMode::Put,
            },
        ))
        .await
        .expect("conflicting write should execute")
        .result
        .expect("conflicting write should return memory result");
    let MemoryResult::Conflict(conflict) = conflict else {
        panic!("expected conflict, got {conflict:?}");
    };
    assert_eq!(
        conflict.actual,
        Some(MemoryVersion {
            opaque: "1".to_owned()
        })
    );
    assert_eq!(
        conflict.current_value,
        Some(HostValue::String("v1".to_owned()))
    );
}

#[tokio::test]
async fn sqlite_memory_enforces_insert_and_update_write_modes() {
    let workspace = TestWorkspace::create("sqlite-memory-write-modes").expect("workspace");
    let client =
        SqliteMemoryClient::open(workspace.path().join("memory.sqlite")).expect("sqlite memory");

    let inserted = client
        .execute(request(
            1,
            MemoryOperation::Put {
                key: HostValue::String("draft".to_owned()),
                value: HostValue::String("v1".to_owned()),
                expected: None,
                mode: MemoryWriteMode::Insert,
            },
        ))
        .await
        .expect("insert should execute")
        .result
        .expect("insert should succeed");
    assert!(matches!(inserted, MemoryResult::Written { .. }));

    let duplicate = client
        .execute(request(
            2,
            MemoryOperation::Put {
                key: HostValue::String("draft".to_owned()),
                value: HostValue::String("v2".to_owned()),
                expected: None,
                mode: MemoryWriteMode::Insert,
            },
        ))
        .await
        .expect("duplicate insert should execute")
        .result
        .expect("duplicate insert should return memory result");
    assert!(matches!(duplicate, MemoryResult::Conflict(_)));

    let missing_update = client
        .execute(request(
            3,
            MemoryOperation::Put {
                key: HostValue::String("missing".to_owned()),
                value: HostValue::String("v1".to_owned()),
                expected: None,
                mode: MemoryWriteMode::Update,
            },
        ))
        .await
        .expect("missing update should execute")
        .result
        .expect("missing update should return memory result");
    assert!(matches!(missing_update, MemoryResult::Conflict(_)));

    let updated = client
        .execute(request(
            4,
            MemoryOperation::Put {
                key: HostValue::String("draft".to_owned()),
                value: HostValue::String("v2".to_owned()),
                expected: None,
                mode: MemoryWriteMode::Update,
            },
        ))
        .await
        .expect("update should execute")
        .result
        .expect("update should succeed");
    assert!(matches!(updated, MemoryResult::Written { .. }));
}

#[tokio::test]
async fn sqlite_memory_scan_pages_stored_entries() {
    let workspace = TestWorkspace::create("sqlite-memory-scan").expect("workspace");
    let client =
        SqliteMemoryClient::open(workspace.path().join("memory.sqlite")).expect("sqlite memory");
    for key in ["a", "b"] {
        client
            .execute(request(
                1,
                MemoryOperation::Put {
                    key: HostValue::String(key.to_owned()),
                    value: HostValue::String(format!("value-{key}")),
                    expected: None,
                    mode: MemoryWriteMode::Put,
                },
            ))
            .await
            .expect("write should execute")
            .result
            .expect("write should succeed");
    }

    let page = client
        .execute(request(
            2,
            MemoryOperation::Scan {
                cursor: None,
                limit: Some(1),
            },
        ))
        .await
        .expect("scan should execute")
        .result
        .expect("scan should succeed");
    let MemoryResult::Entries { entries, cursor } = page else {
        panic!("expected entries");
    };
    assert_eq!(entries.len(), 1);
    assert!(cursor.is_some(), "first page should have continuation");
}

#[tokio::test]
async fn sqlite_memory_query_without_predicate_uses_scan_semantics() {
    let workspace = TestWorkspace::create("sqlite-memory-query").expect("workspace");
    let client =
        SqliteMemoryClient::open(workspace.path().join("memory.sqlite")).expect("sqlite memory");
    for key in ["a", "b"] {
        client
            .execute(request(
                1,
                MemoryOperation::Put {
                    key: HostValue::String(key.to_owned()),
                    value: HostValue::String(format!("value-{key}")),
                    expected: None,
                    mode: MemoryWriteMode::Put,
                },
            ))
            .await
            .expect("write should execute")
            .result
            .expect("write should succeed");
    }

    let result = client
        .execute(request(
            2,
            MemoryOperation::Query {
                query: MemoryQuery {
                    predicate: None,
                    order_by: Vec::new(),
                },
                limit: Some(1),
            },
        ))
        .await
        .expect("query should execute")
        .result
        .expect("query without predicate should use scan semantics");
    let MemoryResult::Entries { entries, cursor } = result else {
        panic!("expected query entries");
    };
    assert_eq!(entries.len(), 1);
    assert!(cursor.is_some(), "limited query should have continuation");
}

#[tokio::test]
async fn sqlite_memory_query_predicate_filters_key_or_value() {
    let workspace = TestWorkspace::create("sqlite-memory-query-predicate").expect("workspace");
    let client =
        SqliteMemoryClient::open(workspace.path().join("memory.sqlite")).expect("sqlite memory");
    for (key, value) in [("paper-1", "alpha draft"), ("paper-2", "final note")] {
        client
            .execute(request(
                1,
                MemoryOperation::Put {
                    key: HostValue::String(key.to_owned()),
                    value: HostValue::String(value.to_owned()),
                    expected: None,
                    mode: MemoryWriteMode::Put,
                },
            ))
            .await
            .expect("write should execute")
            .result
            .expect("write should succeed");
    }

    let result = client
        .execute(request(
            2,
            MemoryOperation::Query {
                query: MemoryQuery {
                    predicate: Some(HostValue::String("draft".to_owned())),
                    order_by: Vec::new(),
                },
                limit: None,
            },
        ))
        .await
        .expect("query should execute")
        .result
        .expect("query predicate should filter entries");
    let MemoryResult::Entries { entries, cursor } = result else {
        panic!("expected query entries");
    };
    assert_eq!(entries.len(), 1);
    assert!(cursor.is_none());
    assert_eq!(entries[0].key, HostValue::String("paper-1".to_owned()));
}

#[tokio::test]
async fn sqlite_memory_vector_search_ranks_embedding_field() {
    let workspace = TestWorkspace::create("sqlite-memory-vector-search").expect("workspace");
    let client =
        SqliteMemoryClient::open(workspace.path().join("memory.sqlite")).expect("sqlite memory");
    for (key, title, embedding) in [
        ("paper-a", "close", vec![1.0, 0.0]),
        ("paper-b", "far", vec![0.0, 1.0]),
    ] {
        client
            .execute(request(
                1,
                MemoryOperation::Put {
                    key: HostValue::String(key.to_owned()),
                    value: paper_record(title, embedding),
                    expected: None,
                    mode: MemoryWriteMode::Put,
                },
            ))
            .await
            .expect("write should execute")
            .result
            .expect("write should succeed");
    }

    let result = client
        .execute(request(
            2,
            MemoryOperation::VectorSearch {
                embedding: vec![0.9, 0.1],
                limit: 1,
                filter: None,
            },
        ))
        .await
        .expect("vector search should execute")
        .result
        .expect("vector search should succeed");
    let MemoryResult::Entries { entries, cursor } = result else {
        panic!("expected vector search entries");
    };
    assert_eq!(entries.len(), 1);
    assert!(cursor.is_none());
    assert_eq!(entries[0].key, HostValue::String("paper-a".to_owned()));
}

#[tokio::test]
async fn in_memory_query_without_predicate_uses_scan_semantics() {
    let client = InMemoryMemoryClient::new();
    for key in ["a", "b"] {
        client
            .execute(request(
                1,
                MemoryOperation::Put {
                    key: HostValue::String(key.to_owned()),
                    value: HostValue::String(format!("value-{key}")),
                    expected: None,
                    mode: MemoryWriteMode::Put,
                },
            ))
            .await
            .expect("write should execute")
            .result
            .expect("write should succeed");
    }

    let result = client
        .execute(request(
            2,
            MemoryOperation::Query {
                query: MemoryQuery {
                    predicate: None,
                    order_by: Vec::new(),
                },
                limit: Some(2),
            },
        ))
        .await
        .expect("query should execute")
        .result
        .expect("query without predicate should use scan semantics");
    let MemoryResult::Entries { entries, cursor } = result else {
        panic!("expected query entries");
    };
    assert_eq!(entries.len(), 2);
    assert!(cursor.is_none());
}

#[tokio::test]
async fn in_memory_enforces_insert_and_update_write_modes() {
    let client = InMemoryMemoryClient::new();
    let inserted = client
        .execute(request(
            1,
            MemoryOperation::Put {
                key: HostValue::String("draft".to_owned()),
                value: HostValue::String("v1".to_owned()),
                expected: None,
                mode: MemoryWriteMode::Insert,
            },
        ))
        .await
        .expect("insert should execute")
        .result
        .expect("insert should succeed");
    assert!(matches!(inserted, MemoryResult::Written { .. }));

    let duplicate = client
        .execute(request(
            2,
            MemoryOperation::Put {
                key: HostValue::String("draft".to_owned()),
                value: HostValue::String("v2".to_owned()),
                expected: None,
                mode: MemoryWriteMode::Insert,
            },
        ))
        .await
        .expect("duplicate insert should execute")
        .result
        .expect("duplicate insert should return memory result");
    assert!(matches!(duplicate, MemoryResult::Conflict(_)));

    let missing_update = client
        .execute(request(
            3,
            MemoryOperation::Put {
                key: HostValue::String("missing".to_owned()),
                value: HostValue::String("v1".to_owned()),
                expected: None,
                mode: MemoryWriteMode::Update,
            },
        ))
        .await
        .expect("missing update should execute")
        .result
        .expect("missing update should return memory result");
    assert!(matches!(missing_update, MemoryResult::Conflict(_)));
}

#[tokio::test]
async fn in_memory_query_predicate_filters_key_or_value() {
    let client = InMemoryMemoryClient::new();
    for (key, value) in [("paper-1", "alpha draft"), ("paper-2", "final note")] {
        client
            .execute(request(
                1,
                MemoryOperation::Put {
                    key: HostValue::String(key.to_owned()),
                    value: HostValue::String(value.to_owned()),
                    expected: None,
                    mode: MemoryWriteMode::Put,
                },
            ))
            .await
            .expect("write should execute")
            .result
            .expect("write should succeed");
    }

    let result = client
        .execute(request(
            2,
            MemoryOperation::Query {
                query: MemoryQuery {
                    predicate: Some(HostValue::String("paper-2".to_owned())),
                    order_by: Vec::new(),
                },
                limit: None,
            },
        ))
        .await
        .expect("query should execute")
        .result
        .expect("query predicate should filter entries");
    let MemoryResult::Entries { entries, cursor } = result else {
        panic!("expected query entries");
    };
    assert_eq!(entries.len(), 1);
    assert!(cursor.is_none());
    assert_eq!(entries[0].value, HostValue::String("final note".to_owned()));
}

#[tokio::test]
async fn in_memory_vector_search_ranks_embedding_field_and_applies_filter() {
    let client = InMemoryMemoryClient::new();
    for (key, title, embedding) in [
        ("paper-a", "keep", vec![1.0, 0.0]),
        ("paper-b", "skip", vec![0.0, 1.0]),
        ("note-c", "keep", vec![0.8, 0.2]),
    ] {
        client
            .execute(request(
                1,
                MemoryOperation::Put {
                    key: HostValue::String(key.to_owned()),
                    value: paper_record(title, embedding),
                    expected: None,
                    mode: MemoryWriteMode::Put,
                },
            ))
            .await
            .expect("write should execute")
            .result
            .expect("write should succeed");
    }

    let result = client
        .execute(request(
            2,
            MemoryOperation::VectorSearch {
                embedding: vec![1.0, 0.0],
                limit: 2,
                filter: Some(HostValue::String("paper".to_owned())),
            },
        ))
        .await
        .expect("vector search should execute")
        .result
        .expect("vector search should succeed");
    let MemoryResult::Entries { entries, cursor } = result else {
        panic!("expected vector search entries");
    };
    assert_eq!(entries.len(), 2);
    assert!(cursor.is_none());
    assert_eq!(entries[0].key, HostValue::String("paper-a".to_owned()));
    assert_eq!(entries[1].key, HostValue::String("paper-b".to_owned()));
}

#[tokio::test]
async fn in_memory_vector_search_rejects_empty_embedding() {
    let client = InMemoryMemoryClient::new();
    let response = client
        .execute(request(
            1,
            MemoryOperation::VectorSearch {
                embedding: Vec::new(),
                limit: 1,
                filter: None,
            },
        ))
        .await
        .expect("vector search should execute");
    let error = response
        .result
        .expect_err("empty vector search embedding must fail closed");
    assert_eq!(error.code, HostErrorCode::InvalidRequest);
}

#[tokio::test]
async fn in_memory_query_rejects_ordering_without_fallback() {
    let client = InMemoryMemoryClient::new();
    let response = client
        .execute(request(
            1,
            MemoryOperation::Query {
                query: MemoryQuery {
                    predicate: None,
                    order_by: vec![MemoryOrderKey {
                        field_path: vec!["created_at".to_owned()],
                        descending: true,
                    }],
                },
                limit: None,
            },
        ))
        .await
        .expect("query should execute");
    let error = response
        .result
        .expect_err("in-memory backend must fail closed for ordered semantic query");
    assert_eq!(error.code, HostErrorCode::InvalidRequest);
}

fn paper_record(title: &str, embedding: Vec<f32>) -> HostValue {
    HostValue::Record(vec![
        ("title".to_owned(), HostValue::String(title.to_owned())),
        (
            "embedding".to_owned(),
            HostValue::List(
                embedding
                    .into_iter()
                    .map(|value| HostValue::Float(f64::from(value)))
                    .collect(),
            ),
        ),
    ])
}
