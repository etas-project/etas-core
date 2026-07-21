use etas_builtin::{
    BuiltinError, BuiltinRangeBounds, BuiltinTypeTag, BuiltinValue, PureIntrinsicRegistry,
    call_pure_intrinsic,
};
use etas_std::{StdIntrinsicId, intrinsic};

#[test]
fn dispatches_control_and_text_pure_intrinsics_by_std_id() {
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::ASSERT),
            &[BuiltinValue::Bool(true)]
        ),
        Ok(BuiltinValue::Unit)
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::TEXT_TRIM),
            &[BuiltinValue::String("  Etas  ".to_owned())],
        ),
        Ok(BuiltinValue::String("Etas".to_owned()))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::TEXT_CONTAINS),
            &[
                BuiltinValue::String("agent-native".to_owned()),
                BuiltinValue::String("native".to_owned()),
            ],
        ),
        Ok(BuiltinValue::Bool(true))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::TEXT_LINES),
            &[BuiltinValue::String("a\nb".to_owned())],
        ),
        Ok(BuiltinValue::Array(vec![
            BuiltinValue::String("a".to_owned()),
            BuiltinValue::String("b".to_owned()),
        ]))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::TEXT_PARSE_I32),
            &[BuiltinValue::String("42".to_owned())],
        ),
        Ok(BuiltinValue::ResultOk(Box::new(BuiltinValue::I32(42))))
    );
}

#[test]
fn dispatches_bytes_list_option_and_result_helpers() {
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::BYTES_LEN),
            &[BuiltinValue::Bytes(vec![1, 2, 3])],
        ),
        Ok(BuiltinValue::Usize(3))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::LIST_IS_EMPTY),
            &[BuiltinValue::List(Vec::new())],
        ),
        Ok(BuiltinValue::Bool(true))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::LIST_LEN),
            &[BuiltinValue::Slice(vec![
                BuiltinValue::I32(1),
                BuiltinValue::I32(2),
            ])],
        ),
        Ok(BuiltinValue::Usize(2))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::MAP_CONTAINS_KEY),
            &[
                BuiltinValue::Map(vec![(
                    BuiltinValue::I32(7),
                    BuiltinValue::String("x".into())
                )]),
                BuiltinValue::I32(7),
            ],
        ),
        Ok(BuiltinValue::Bool(true))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::OPTION_IS_SOME),
            &[BuiltinValue::OptionSome(Box::new(BuiltinValue::Unit))],
        ),
        Ok(BuiltinValue::Bool(true))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::OPTION_UNWRAP),
            &[BuiltinValue::OptionSome(Box::new(BuiltinValue::String(
                "value".to_owned(),
            )))],
        ),
        Ok(BuiltinValue::String("value".to_owned()))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::OPTION_UNWRAP),
            &[BuiltinValue::ResultOk(Box::new(BuiltinValue::I32(7)))],
        ),
        Ok(BuiltinValue::I32(7))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::RESULT_IS_ERR),
            &[BuiltinValue::ResultErr(Box::new(BuiltinValue::String(
                "err".to_owned(),
            )))],
        ),
        Ok(BuiltinValue::Bool(true))
    );
}

#[test]
fn dispatches_pure_codec_and_crypto_intrinsics() {
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::TEXT_UTF8_ENCODE),
            &[BuiltinValue::String("hé".to_owned())],
        ),
        Ok(BuiltinValue::Bytes("hé".as_bytes().to_vec()))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::TEXT_UTF8_DECODE),
            &[
                BuiltinValue::Bytes(vec![0x68, 0xc3, 0xa9]),
                BuiltinValue::Variant {
                    name: "Strict".to_owned(),
                    fields: Vec::new(),
                },
            ],
        ),
        Ok(BuiltinValue::ResultOk(Box::new(BuiltinValue::String(
            "hé".to_owned()
        ))))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::TEXT_UTF8_DECODE),
            &[
                BuiltinValue::Bytes(vec![0xff]),
                BuiltinValue::Variant {
                    name: "Strict".to_owned(),
                    fields: Vec::new(),
                },
            ],
        ),
        Ok(BuiltinValue::ResultErr(Box::new(BuiltinValue::Variant {
            name: "InvalidUtf8".to_owned(),
            fields: Vec::new(),
        })))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::CRYPTO_CONSTANT_TIME_EQ),
            &[
                BuiltinValue::Bytes(vec![1, 2, 3]),
                BuiltinValue::Bytes(vec![1, 2, 3]),
            ],
        ),
        Ok(BuiltinValue::Bool(true))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::CRYPTO_CONSTANT_TIME_EQ),
            &[
                BuiltinValue::Bytes(vec![1, 2, 3]),
                BuiltinValue::Bytes(vec![1, 2]),
            ],
        ),
        Ok(BuiltinValue::Bool(false))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::CRYPTO_SHA256),
            &[BuiltinValue::Bytes(b"abc".to_vec())],
        ),
        Ok(BuiltinValue::Bytes(hex_bytes(
            "ba7816bf8f01cfea414140de5dae2223\
             b00361a396177a9cb410ff61f20015ad"
        )))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::HTTP_ENCODE_REQUEST),
            &[BuiltinValue::Record(vec![
                ("method".to_owned(), BuiltinValue::String("POST".to_owned())),
                (
                    "target".to_owned(),
                    BuiltinValue::String("/submit".to_owned())
                ),
                (
                    "headers".to_owned(),
                    BuiltinValue::List(vec![BuiltinValue::Record(vec![
                        ("name".to_owned(), BuiltinValue::String("host".to_owned())),
                        (
                            "value".to_owned(),
                            BuiltinValue::String("example.test".to_owned())
                        )
                    ])]),
                ),
                ("body".to_owned(), BuiltinValue::Bytes(b"ok".to_vec())),
            ])],
        ),
        Ok(BuiltinValue::ResultOk(Box::new(BuiltinValue::Bytes(
            b"POST /submit HTTP/1.1\r\nhost: example.test\r\ncontent-length: 2\r\n\r\nok".to_vec()
        ))))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::HTTP_DECODE_RESPONSE_HEAD),
            &[BuiltinValue::Bytes(
                b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\nbody".to_vec()
            )],
        ),
        Ok(BuiltinValue::ResultOk(Box::new(BuiltinValue::Record(
            vec![
                (
                    "version".to_owned(),
                    BuiltinValue::String("HTTP/1.1".to_owned())
                ),
                ("status".to_owned(), BuiltinValue::I32(200)),
                ("reason".to_owned(), BuiltinValue::String("OK".to_owned())),
                (
                    "headers".to_owned(),
                    BuiltinValue::List(vec![BuiltinValue::Record(vec![
                        (
                            "name".to_owned(),
                            BuiltinValue::String("content-length".to_owned())
                        ),
                        ("value".to_owned(), BuiltinValue::String("0".to_owned()))
                    ])])
                ),
            ]
        ))))
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::HTTP_DECODE_RESPONSE),
            &[BuiltinValue::Bytes(b"not http\r\n\r\n".to_vec())],
        ),
        Ok(BuiltinValue::ResultErr(Box::new(BuiltinValue::Variant {
            name: "MalformedMessage".to_owned(),
            fields: Vec::new(),
        })))
    );
}

#[test]
fn builtin_values_preserve_collection_shape_tags() {
    assert_eq!(
        BuiltinValue::Array(vec![BuiltinValue::I32(1)]).type_tag(),
        BuiltinTypeTag::Array
    );
    assert_eq!(
        BuiltinValue::List(vec![BuiltinValue::I32(1)]).type_tag(),
        BuiltinTypeTag::List
    );
    assert_eq!(
        BuiltinValue::Slice(vec![BuiltinValue::I32(1)]).type_tag(),
        BuiltinTypeTag::Slice
    );
    assert_eq!(
        BuiltinValue::Record(vec![("field".to_owned(), BuiltinValue::I32(1))]).type_tag(),
        BuiltinTypeTag::Record
    );
    assert_eq!(
        BuiltinValue::Range {
            start: Box::new(BuiltinValue::I32(1)),
            end: Box::new(BuiltinValue::I32(3)),
            bounds: BuiltinRangeBounds::ClosedOpen,
        }
        .type_tag(),
        BuiltinTypeTag::Range
    );
    assert_eq!(
        BuiltinValue::Variant {
            name: "InvalidUtf8".to_owned(),
            fields: Vec::new(),
        }
        .type_tag(),
        BuiltinTypeTag::Variant
    );
}

#[test]
fn returns_structured_errors_without_rendering_diagnostics() {
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::ASSERT),
            &[BuiltinValue::Bool(false)]
        ),
        Err(BuiltinError::AssertionFailed)
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::TEXT_TRIM),
            &[BuiltinValue::Bool(true)],
        ),
        Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: BuiltinTypeTag::Bool,
        })
    );
    assert_eq!(
        call_pure_intrinsic(
            StdIntrinsicId(intrinsic::pure::OPTION_UNWRAP),
            &[BuiltinValue::OptionNone],
        ),
        Err(BuiltinError::Abort {
            message: "unwrap encountered None".to_owned(),
        })
    );
    assert_eq!(
        call_pure_intrinsic(StdIntrinsicId(intrinsic::runtime::APPROVE), &[]),
        Err(BuiltinError::UnsupportedIntrinsic {
            intrinsic: StdIntrinsicId(intrinsic::runtime::APPROVE),
        })
    );
}

#[test]
fn registry_identifies_only_pure_kernel_intrinsics() {
    let registry = PureIntrinsicRegistry;
    assert!(registry.contains(StdIntrinsicId(intrinsic::pure::TEXT_LOWERCASE)));
    assert!(registry.contains(StdIntrinsicId(intrinsic::pure::TEXT_PARSE_I32)));
    assert!(registry.contains(StdIntrinsicId(intrinsic::pure::OPTION_UNWRAP)));
    assert!(registry.contains(StdIntrinsicId(intrinsic::pure::TEXT_UTF8_DECODE)));
    assert!(registry.contains(StdIntrinsicId(intrinsic::pure::CRYPTO_CONSTANT_TIME_EQ)));
    assert!(!registry.contains(StdIntrinsicId(intrinsic::runtime::FS_WRITE_BYTES)));
    assert!(!registry.contains(StdIntrinsicId(intrinsic::runtime::CHECKPOINT)));
}

fn hex_bytes(text: &str) -> Vec<u8> {
    let text = text.split_whitespace().collect::<String>();
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).expect("hex byte"))
        .collect()
}
