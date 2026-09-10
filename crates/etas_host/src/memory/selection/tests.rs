use super::*;
use crate::storage::version::{StoreGeneration, scope_identity};
use crate::{HostValue, MemoryVersion};

#[test]
fn top_k_validates_budget_before_materializing_or_replacing_an_entry() {
    let limits = StorageLimits {
        max_result_bytes: 1024,
        ..Default::default()
    };
    let entry = MemoryEntry {
        key: HostValue::String("a".into()),
        value: HostValue::Bool(true),
        version: MemoryVersion::issue(
            &scope_identity("volatile", "top-k", &[]),
            &StoreGeneration::new().unwrap(),
            1,
        )
        .unwrap(),
    };
    let mut top = TopK::new(1);
    top.push(
        1.0,
        "a",
        entry_size(&entry, &limits).unwrap(),
        || entry.clone(),
        &limits,
    )
    .unwrap();
    // A worse candidate neither consumes result bytes nor clones its payload.
    top.push(
        0.5,
        "z",
        2048,
        || panic!("excluded candidate materialized"),
        &limits,
    )
    .unwrap();
    // An oversized winner is a limit error, not a partial successful answer.
    let error = top
        .push(
            2.0,
            "b",
            2048,
            || panic!("oversized candidate materialized"),
            &limits,
        )
        .unwrap_err();
    assert_eq!(error.code, HostErrorCode::BudgetExceeded);
    assert_eq!(top.finish(), vec![entry]);
}
