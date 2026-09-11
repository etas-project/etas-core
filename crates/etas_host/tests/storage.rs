use etas_host::{
    AuthorityContext, HostError, HostErrorCode, HostRequestId, HostValue, InMemoryMemoryClient,
    MemoryClient, MemoryCursor, MemoryOperation, MemoryRegionRef, MemoryRequest, MemoryResult,
    MemoryVersion, SqliteMemoryClient, StorageLimits, StoreRef, TestWorkspace, TraceContext,
    TraceId, WriteCondition,
};

fn store(name: &str) -> StoreRef {
    StoreRef {
        region: MemoryRegionRef {
            stable_id: "storage_test".into(),
            schema_fingerprint: None,
        },
        path: vec![name.into()],
    }
}
fn request(store: StoreRef, operation: MemoryOperation) -> MemoryRequest {
    MemoryRequest {
        id: HostRequestId(1),
        store,
        operation,
        authority: AuthorityContext::deny_all(),
        trace: TraceContext::root(TraceId(1)),
        budget: Default::default(),
    }
}
async fn run<C: MemoryClient<Error = HostError>>(
    client: &C,
    target: StoreRef,
    operation: MemoryOperation,
) -> Result<MemoryResult, HostError> {
    client.execute(request(target, operation)).await?.result
}
async fn put<C: MemoryClient<Error = HostError>>(
    client: &C,
    target: StoreRef,
    key: &str,
) -> MemoryVersion {
    match run(
        client,
        target,
        MemoryOperation::Put {
            key: HostValue::String(key.into()),
            value: HostValue::String("same".into()),
            condition: WriteCondition::Any,
        },
    )
    .await
    .unwrap()
    {
        MemoryResult::Written { version } => version,
        other => panic!("expected write, got {other:?}"),
    }
}
async fn conditions<C: MemoryClient<Error = HostError>>(client: &C) {
    let target = store("values");
    let first = put(client, target.clone(), "a").await;
    let other = put(client, target.clone(), "b").await;
    assert_ne!(first, other);
    let foreign = put(client, store("another"), "a").await;
    for wrong in [other, foreign] {
        let result = run(
            client,
            target.clone(),
            MemoryOperation::Put {
                key: HostValue::String("a".into()),
                value: HostValue::Unit,
                condition: WriteCondition::Match(wrong),
            },
        )
        .await
        .unwrap();
        assert!(
            matches!(result,MemoryResult::Conflict(ref c) if c.actual.as_ref()==Some(&first) && c.current_value.is_none())
        );
    }
    let deletion = run(
        client,
        target.clone(),
        MemoryOperation::Delete {
            key: HostValue::String("a".into()),
            condition: WriteCondition::Match(first.clone()),
        },
    )
    .await
    .unwrap();
    assert!(matches!(deletion, MemoryResult::Deleted { .. }));
    let recreated = put(client, target.clone(), "a").await;
    assert_ne!(first, recreated);
    assert!(matches!(
        run(
            client,
            target.clone(),
            MemoryOperation::Delete {
                key: HostValue::String("a".into()),
                condition: WriteCondition::Match(first)
            }
        )
        .await
        .unwrap(),
        MemoryResult::Conflict(_)
    ));
    for condition in [WriteCondition::Any, WriteCondition::Missing] {
        assert_eq!(
            run(
                client,
                target.clone(),
                MemoryOperation::Delete {
                    key: HostValue::String("absent".into()),
                    condition
                }
            )
            .await
            .unwrap(),
            MemoryResult::Unchanged
        );
    }
    for condition in [
        WriteCondition::Exists,
        WriteCondition::Match(recreated.clone()),
    ] {
        assert!(matches!(
            run(
                client,
                target.clone(),
                MemoryOperation::Delete {
                    key: HostValue::String("absent".into()),
                    condition
                }
            )
            .await
            .unwrap(),
            MemoryResult::Conflict(_)
        ));
    }
    // Failed conditions and absent deletes did not allocate a revision.
    let next = put(client, target, "c").await;
    let revision = |v: &MemoryVersion| {
        u64::from_str_radix(v.as_token().rsplit(':').next().unwrap(), 16).unwrap()
    };
    assert_eq!(revision(&next), revision(&recreated) + 1);
}
#[tokio::test]
async fn conditional_memory_semantics_are_shared_by_both_backends() {
    conditions(&InMemoryMemoryClient::new()).await;
    let workspace = TestWorkspace::create("storage-conditions").unwrap();
    conditions(&SqliteMemoryClient::open(workspace.path().join("db")).unwrap()).await;
}
#[test]
fn version_parser_rejects_decimal_oversized_zero_and_noncanonical_tokens() {
    for token in [
        "1".to_owned(),
        "v1".into(),
        "x".repeat(10000),
        format!("mv1:{}:{}:0000000000000000", "0".repeat(64), "0".repeat(32)),
        format!("mv1:{}:{}:8000000000000000", "0".repeat(64), "0".repeat(32)),
        format!("mv1:{}:{}:0000000000000001", "A".repeat(64), "0".repeat(32)),
    ] {
        assert_eq!(
            MemoryVersion::parse(&token).unwrap_err().code,
            HostErrorCode::InvalidRequest
        );
    }
}
#[tokio::test]
async fn separate_sqlite_connections_have_one_cas_winner() {
    let workspace = TestWorkspace::create("storage-cas").unwrap();
    let path = workspace.path().join("db");
    let first = SqliteMemoryClient::open(&path).unwrap();
    let version = put(&first, store("values"), "a").await;
    let gate = std::sync::Arc::new(tokio::sync::Barrier::new(8));
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let client = SqliteMemoryClient::open(&path).unwrap();
        let version = version.clone();
        let gate = gate.clone();
        tasks.push(tokio::spawn(async move {
            gate.wait().await;
            run(
                &client,
                store("values"),
                MemoryOperation::Put {
                    key: HostValue::String("a".into()),
                    value: HostValue::Unit,
                    condition: WriteCondition::Match(version),
                },
            )
            .await
        }));
    }
    let mut writes = 0;
    let mut conflicts = 0;
    for task in tasks {
        match task.await.unwrap().unwrap() {
            MemoryResult::Written { .. } => writes += 1,
            MemoryResult::Conflict(_) => conflicts += 1,
            other => panic!("{other:?}"),
        }
    }
    assert_eq!((writes, conflicts), (1, 7));
}
async fn first_page<C: MemoryClient<Error = HostError>>(client: &C) -> MemoryCursor {
    for key in ["a", "b", "c"] {
        put(client, store("values"), key).await;
    }
    let MemoryResult::Entries {
        entries,
        cursor: Some(cursor),
    } = run(
        client,
        store("values"),
        MemoryOperation::Scan {
            cursor: None,
            limit: Some(1),
        },
    )
    .await
    .unwrap()
    else {
        panic!("expected page")
    };
    assert_eq!(entries.len(), 1);
    cursor
}
async fn cursor_contract<C: MemoryClient<Error = HostError>>(client: &C) {
    let cursor = first_page(client).await;
    let page = run(
        client,
        store("values"),
        MemoryOperation::Scan {
            cursor: Some(cursor.clone()),
            limit: Some(2),
        },
    )
    .await
    .unwrap();
    assert!(matches!(page,MemoryResult::Entries{ref entries,cursor:None} if entries.len()==2));
    put(client, store("other"), "x").await;
    assert_eq!(
        run(
            client,
            store("other"),
            MemoryOperation::Scan {
                cursor: Some(cursor.clone()),
                limit: Some(1)
            }
        )
        .await
        .unwrap_err()
        .code,
        HostErrorCode::InvalidRequest
    );
    put(client, store("values"), "d").await;
    assert_eq!(
        run(
            client,
            store("values"),
            MemoryOperation::Scan {
                cursor: Some(cursor),
                limit: Some(1)
            }
        )
        .await
        .unwrap_err()
        .code,
        HostErrorCode::InvalidRequest
    );
    assert_eq!(
        run(
            client,
            store("values"),
            MemoryOperation::Scan {
                cursor: None,
                limit: Some(0)
            }
        )
        .await
        .unwrap_err()
        .code,
        HostErrorCode::InvalidRequest
    );
}
#[tokio::test]
async fn keyset_cursors_reject_changed_generation_revision_and_scope() {
    cursor_contract(&InMemoryMemoryClient::new()).await;
    let workspace = TestWorkspace::create("storage-cursor").unwrap();
    cursor_contract(&SqliteMemoryClient::open(workspace.path().join("db")).unwrap()).await;
}
#[tokio::test]
async fn legacy_rows_migrate_once_and_restart_preserves_versions() {
    let workspace = TestWorkspace::create("storage-migrate").unwrap();
    let path = workspace.path().join("db");
    let legacy = rusqlite::Connection::open(&path).unwrap();
    legacy.execute_batch(r#"CREATE TABLE memory_entries(region TEXT NOT NULL,path TEXT NOT NULL,key_json TEXT NOT NULL,value_json TEXT NOT NULL,version INTEGER NOT NULL,PRIMARY KEY(region,path,key_json));
        INSERT INTO memory_entries VALUES('storage_test','["values"]','{"kind":"string","value":"a"}','{"kind":"int","value":"7"}',1);
        INSERT INTO memory_entries VALUES('storage_test','["values"]','{"kind":"string","value":"b"}','{"kind":"int","value":"8"}',1);"#).unwrap();
    drop(legacy);
    let client = SqliteMemoryClient::open(&path).unwrap();
    let read = |key: &str| MemoryOperation::Get {
        key: HostValue::String(key.into()),
    };
    let first = run(&client, store("values"), read("a")).await.unwrap();
    let second = run(&client, store("values"), read("b")).await.unwrap();
    let (MemoryResult::Value { version: a, .. }, MemoryResult::Value { version: b, .. }) =
        (&first, &second)
    else {
        panic!("expected values")
    };
    assert_ne!(a, b);
    drop(client);
    let reopened = SqliteMemoryClient::open(&path).unwrap();
    assert_eq!(
        run(&reopened, store("values"), read("a")).await.unwrap(),
        first
    );
}
#[tokio::test]
async fn exhausted_revision_fails_before_mutation() {
    let workspace = TestWorkspace::create("storage-exhaustion").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteMemoryClient::open(&path).unwrap();
    let first = put(&client, store("values"), "a").await;
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute("UPDATE memory_stores SET revision=?1", [i64::MAX])
        .unwrap();
    let error = run(
        &client,
        store("values"),
        MemoryOperation::Put {
            key: HostValue::String("a".into()),
            value: HostValue::Unit,
            condition: WriteCondition::Any,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, HostErrorCode::ProviderUnavailable);
    assert!(
        matches!(run(&client,store("values"),MemoryOperation::Get{key:HostValue::String("a".into())}).await.unwrap(),MemoryResult::Value{version,..} if version==first)
    );
}
#[tokio::test]
async fn value_work_and_page_limits_fail_closed() {
    let workspace = TestWorkspace::create("storage-limits").unwrap();
    let limits = StorageLimits {
        max_scan_rows: 2,
        max_depth: 4,
        max_value_bytes: 512,
        max_page_entries: 2,
        ..Default::default()
    };
    let client = SqliteMemoryClient::open_with_limits(workspace.path().join("db"), limits).unwrap();
    for key in ["a", "b", "c"] {
        put(&client, store("values"), key).await;
    }
    let error = run(
        &client,
        store("values"),
        MemoryOperation::VectorSearch {
            embedding: vec![1.0],
            limit: 1,
            filter: None,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, HostErrorCode::BudgetExceeded);
    let mut deep = HostValue::Unit;
    for _ in 0..6 {
        deep = HostValue::List(vec![deep]);
    }
    for value in [deep, HostValue::String("x".repeat(513))] {
        let result = run(
            &client,
            store("values"),
            MemoryOperation::Put {
                key: HostValue::Unit,
                value,
                condition: WriteCondition::Any,
            },
        )
        .await;
        assert_eq!(result.unwrap_err().code, HostErrorCode::BudgetExceeded);
    }
}
#[tokio::test]
async fn typed_keys_and_numeric_extremes_roundtrip_losslessly() {
    let workspace = TestWorkspace::create("storage-codec").unwrap();
    let client = SqliteMemoryClient::open(workspace.path().join("db")).unwrap();
    let values = [
        HostValue::Int(i128::MIN),
        HostValue::UInt(u128::MAX),
        HostValue::Json(etas_host::HostJsonValue::Null),
        HostValue::Bytes(vec![0, 255]),
        HostValue::Variant {
            name: "Some".into(),
            fields: vec![HostValue::Int(42)],
        },
    ];
    for (index, value) in values.iter().enumerate() {
        run(
            &client,
            store("values"),
            MemoryOperation::Put {
                key: HostValue::Int(index as i128),
                value: value.clone(),
                condition: WriteCondition::Any,
            },
        )
        .await
        .unwrap();
        run(
            &client,
            store("values"),
            MemoryOperation::Put {
                key: HostValue::String(index.to_string()),
                value: HostValue::Unit,
                condition: WriteCondition::Any,
            },
        )
        .await
        .unwrap();
        assert!(
            matches!(run(&client,store("values"),MemoryOperation::Get{key:HostValue::Int(index as i128)}).await.unwrap(),MemoryResult::Value{value:actual,..} if actual==*value)
        );
    }
}

#[tokio::test]
async fn cross_process_compare_and_swap_has_one_winner() {
    let workspace = TestWorkspace::create("storage-process-cas").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteMemoryClient::open(&path).unwrap();
    let version = put(&client, store("values"), "a").await;
    struct Children(Vec<std::process::Child>);
    impl Drop for Children {
        fn drop(&mut self) {
            for child in &mut self.0 {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    let mut children = Children(Vec::new());
    for _ in 0..4 {
        children.0.push(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "storage_process_cas_helper",
                    "--ignored",
                    "--nocapture",
                ])
                .env("ETAS_STORAGE_TEST_DB", &path)
                .env("ETAS_STORAGE_TEST_VERSION", version.as_token())
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
    }
    for child in &mut children.0 {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(b"go\n").unwrap();
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut written = 0;
    let mut conflicts = 0;
    for child in &mut children.0 {
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                use std::io::Read;
                let mut output = String::new();
                child
                    .stdout
                    .take()
                    .unwrap()
                    .read_to_string(&mut output)
                    .unwrap();
                let mut errors = String::new();
                child
                    .stderr
                    .take()
                    .unwrap()
                    .read_to_string(&mut errors)
                    .unwrap();
                assert!(status.success(), "{output}\n{errors}");
                written += usize::from(output.contains("STORAGE_WRITTEN"));
                conflicts += usize::from(output.contains("STORAGE_CONFLICT"));
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "storage subprocess exceeded deadline"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    assert_eq!((written, conflicts), (1, 3));
}
#[test]
#[ignore = "subprocess helper driven by cross_process_compare_and_swap_has_one_winner"]
fn storage_process_cas_helper() {
    use std::io::BufRead;
    let path = std::env::var_os("ETAS_STORAGE_TEST_DB").expect("test database");
    let version =
        MemoryVersion::parse(&std::env::var("ETAS_STORAGE_TEST_VERSION").unwrap()).unwrap();
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).unwrap();
    assert_eq!(line, "go\n");
    let client = SqliteMemoryClient::open(path).unwrap();
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run(
            &client,
            store("values"),
            MemoryOperation::Put {
                key: HostValue::String("a".into()),
                value: HostValue::Unit,
                condition: WriteCondition::Match(version),
            },
        ))
        .unwrap();
    match result {
        MemoryResult::Written { .. } => println!("STORAGE_WRITTEN"),
        MemoryResult::Conflict(_) => println!("STORAGE_CONFLICT"),
        other => panic!("{other:?}"),
    }
}
