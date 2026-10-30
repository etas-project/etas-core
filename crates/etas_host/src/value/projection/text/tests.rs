use super::*;
use crate::{
    HostJsonValue, HostValue,
    value::{
        json::project_to_json_string,
        projection::{
            HostJsonProjection, HostJsonVisitor, HostValueProjection, HostValueVisitor,
            project_to_host_value,
        },
    },
};

struct Chunked<'a> {
    bytes: usize,
    chunks: &'a [&'a str],
    json: bool,
}

impl HostValueProjection for Chunked<'_> {
    fn project<V: HostValueVisitor>(&self, visitor: V) -> Result<V::Output, HostError> {
        if self.json {
            visitor.json(self)
        } else {
            visitor.text(self.bytes, self.chunks.iter().copied())
        }
    }
}

impl HostJsonProjection for Chunked<'_> {
    fn project_json<V: HostJsonVisitor>(&self, visitor: V) -> Result<V::Output, HostError> {
        visitor.text(self.bytes, self.chunks.iter().copied())
    }
}

#[test]
fn chunked_text_uses_one_canonical_string_in_both_domains() {
    for chunks in [
        vec![],
        vec!["", ""],
        vec![
            "prefix",
            "",
            "\"\\\u{0}\u{8}\u{c}\n\r\t\u{1f}",
            "中😀",
            "suffix",
        ],
    ] {
        let expected = chunks.concat();
        for json in [false, true] {
            let source = Chunked {
                bytes: expected.len(),
                chunks: &chunks,
                json,
            };
            assert_eq!(
                project_to_json_string(&source).unwrap(),
                serde_json::to_string(&expected).unwrap()
            );
            assert_eq!(
                project_to_host_value(&source).unwrap(),
                if json {
                    HostValue::Json(HostJsonValue::String(expected.clone()))
                } else {
                    HostValue::String(expected.clone())
                }
            );
        }
    }
}

#[test]
fn chunked_json_escaping_matches_serde_for_every_unicode_scalar() {
    for ch in (0..=0x10ffff).filter_map(char::from_u32) {
        let mut buffer = [0; 4];
        let encoded = ch.encode_utf8(&mut buffer);
        let source = Chunked {
            bytes: encoded.len(),
            chunks: &["", encoded, ""],
            json: false,
        };
        assert_eq!(
            project_to_json_string(&source).unwrap(),
            serde_json::to_string(encoded).unwrap(),
            "U+{:04x}",
            ch as u32
        );
    }
}

#[test]
fn inconsistent_chunk_lengths_reject_without_exposing_payloads() {
    for bytes in [0, 1, 6, 8, usize::MAX] {
        for json in [false, true] {
            let source = Chunked {
                bytes,
                chunks: &["pay", "load"],
                json,
            };
            for error in [
                project_to_host_value(&source).unwrap_err(),
                project_to_json_string(&source).unwrap_err(),
            ] {
                assert_eq!(error.code, HostErrorCode::SchemaMismatch);
                assert!(error.message.contains("byte length"));
                assert!(!format!("{error:?}").contains("payload"));
            }
        }
    }
}
