use super::*;
use crate::HostJsonValue;
use crate::host_value_to_json;

#[test]
fn streaming_json_matches_the_canonical_value_projection() {
    let values = vec![
        HostValue::Unit,
        HostValue::Bool(true),
        HostValue::Int(i64::MIN as i128),
        HostValue::UInt(u64::MAX as u128),
        HostValue::Float(-0.0),
        HostValue::Float(1.234567890123),
        HostValue::String("\"escape\n中🙂\\".into()),
        HostValue::Bytes(vec![0, 128, 255]),
        HostValue::Map(vec![(HostValue::Int(2), HostValue::String("two".into()))]),
        HostValue::Record(vec![
            ("z".into(), HostValue::Unit),
            ("a".into(), HostValue::Bool(false)),
        ]),
        HostValue::Variant {
            name: "Named".into(),
            fields: vec![HostValue::Int(1)],
        },
        HostValue::Json(HostJsonValue::Object(vec![
            (
                "z".into(),
                HostJsonValue::Array(vec![HostJsonValue::Null, HostJsonValue::Bool(true)]),
            ),
            ("a".into(), HostJsonValue::Number(0.25)),
            ("b".into(), HostJsonValue::String("nested".into())),
        ])),
    ];
    for value in &values {
        assert_eq!(
            host_value_to_json_string(value).unwrap(),
            host_value_to_json(value).unwrap().to_string()
        );
    }
    let nested = HostValue::List(values);
    assert_eq!(
        host_value_to_json_string(&nested).unwrap(),
        host_value_to_json(&nested).unwrap().to_string()
    );
}

#[test]
fn streaming_json_retains_number_and_duplicate_field_rejections() {
    for value in [
        HostValue::Int(i64::MIN as i128 - 1),
        HostValue::UInt(u64::MAX as u128 + 1),
        HostValue::Float(f64::INFINITY),
        HostValue::Json(HostJsonValue::Number(f64::NAN)),
        HostValue::Record(vec![
            ("x".into(), HostValue::Unit),
            ("x".into(), HostValue::Bool(true)),
        ]),
        HostValue::Json(HostJsonValue::Object(vec![
            ("x".into(), HostJsonValue::Null),
            ("x".into(), HostJsonValue::Bool(true)),
        ])),
    ] {
        assert_eq!(
            host_value_to_json_string(&value).unwrap_err(),
            host_value_to_json(&value).unwrap_err()
        );
    }
}
