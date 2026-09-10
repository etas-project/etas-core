use super::*;
use crate::{HostValue, StorageLimits};
#[test]
fn byte_buffers_do_not_consume_one_type_node_per_byte() {
    let limits = StorageLimits {
        max_nodes: 8,
        ..Default::default()
    };
    let value = HostValue::Bytes(vec![255; 32000]);
    let encoded = encode_with_limits(&value, &limits).unwrap();
    assert_eq!(decode_with_limits(&encoded, &limits).unwrap(), value);
}
#[test]
fn codec_rejects_wire_overflow_and_nested_decode_before_materializing() {
    let limits = StorageLimits {
        max_value_bytes: 512,
        max_depth: 3,
        ..Default::default()
    };
    let escaped = HostValue::String("\u{0000}".repeat(100));
    assert_eq!(
        encode_with_limits(&escaped, &limits).unwrap_err().code,
        HostErrorCode::BudgetExceeded
    );
    let mut encoded = r#"{"kind":"unit"}"#.to_owned();
    for _ in 0..5 {
        encoded = format!(r#"{{"kind":"list","items":[{encoded}]}}"#);
    }
    assert_eq!(
        decode_with_limits(&encoded, &limits).unwrap_err().code,
        HostErrorCode::BudgetExceeded
    );
}
#[test]
fn codec_rejects_unknown_fields_duplicate_names_and_missing_values() {
    for value in [
        r#"{"kind":"unit","unexpected":true}"#,
        r#"{"kind":"int","value":"170141183460469231731687303715884105728"}"#,
        r#"{"kind":"record","fields":[{"name":"x","value":{"kind":"unit"}},{"name":"x","value":{"kind":"unit"}}]}"#,
        r#"{"kind":"json","value":{"x":1,"x":2}}"#,
        r#"{"kind":"json"}"#,
        r#"{"kind":"list","items":null}"#,
    ] {
        assert!(
            decode_with_limits(value, &StorageLimits::default()).is_err(),
            "{value}"
        );
    }
}
#[test]
fn canonical_encoding_preserves_legacy_key_bytes() {
    let record = HostValue::Record(vec![("x".into(), HostValue::Int(1))]);
    assert_eq!(
        encode_with_limits(&record, &StorageLimits::default()).unwrap(),
        r#"{"fields":[{"name":"x","value":{"kind":"int","value":"1"}}],"kind":"record"}"#
    );
    let value = HostValue::Json(crate::HostJsonValue::Null);
    assert_eq!(
        decode_with_limits(
            &encode_with_limits(&value, &StorageLimits::default()).unwrap(),
            &StorageLimits::default()
        )
        .unwrap(),
        value
    );
}

#[test]
fn borrowed_record_streaming_matches_owned_encoding_and_resource_limits() {
    use RecordValueRef::*;
    let payload = HostValue::List(vec![
        HostValue::Bytes(vec![0, 127, 255]),
        HostValue::Int(-12),
    ]);
    let provenance = HostValue::Json(crate::HostJsonValue::Object(vec![
        ("z".into(), crate::HostJsonValue::Bool(true)),
        ("a".into(), crate::HostJsonValue::Null),
    ]));
    let fields = [
        ("name", String("a\n\"b")),
        ("some", OptionalString(Some("text"))),
        ("none", OptionalString(None)),
        ("payload", Value(&payload)),
        ("provenance", OptionalValue(Some(&provenance))),
        ("missing", OptionalValue(None)),
    ];
    let owned = HostValue::Record(
        fields
            .iter()
            .map(|(name, v)| (name.to_string(), v.into_owned()))
            .collect(),
    );
    let limits = StorageLimits::default();
    let expected = encode_with_limits(&owned, &limits).unwrap();
    let mut actual = Vec::new();
    assert_eq!(
        encode_record_to(&fields, &limits, &mut actual).unwrap(),
        expected.len()
    );
    assert_eq!(actual, expected.as_bytes());
    let mut hash = blake3::Hasher::new();
    encode_to(&owned, &limits, &mut hash).unwrap();
    assert_eq!(hash.finalize(), blake3::hash(expected.as_bytes()));
    assert_eq!(
        encode_to(&owned, &limits, std::io::sink()).unwrap(),
        expected.len()
    );
    for limits in (0..60)
        .map(|max_nodes| StorageLimits {
            max_nodes,
            ..Default::default()
        })
        .chain((0..8).map(|max_depth| StorageLimits {
            max_depth,
            ..Default::default()
        }))
        .chain((0..2000).map(|max_value_bytes| StorageLimits {
            max_value_bytes,
            ..Default::default()
        }))
    {
        let owned = encode_with_limits(&owned, &limits)
            .map(|s| s.len())
            .map_err(|e| e.code);
        let borrowed = encode_record_to(&fields, &limits, std::io::sink()).map_err(|e| e.code);
        assert_eq!(owned, borrowed, "{limits:?}");
    }
}
