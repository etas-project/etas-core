use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

mod incremental;

pub use incremental::{
    DecodeStatus, HttpCodecFailureKind, HttpHeader, HttpWireResponse, HttpWireResponseHead,
    decode_response_head_incremental, decode_response_incremental,
};

pub fn encode_request(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    let request = HttpWireRequest::from_value(&args[0])?;
    let Ok(bytes) = request.encode() else {
        return Ok(codec_error());
    };
    Ok(BuiltinValue::ResultOk(Box::new(BuiltinValue::Bytes(bytes))))
}

pub fn decode_response_head(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    let BuiltinValue::Bytes(bytes) = &args[0] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Bytes,
            actual: args[0].type_tag(),
        });
    };
    match decode_response_head_incremental(bytes, true) {
        DecodeStatus::Complete { value, .. } => {
            Ok(BuiltinValue::ResultOk(Box::new(value.into_value())))
        }
        DecodeStatus::NeedMore | DecodeStatus::Malformed { .. } => Ok(codec_error()),
    }
}

pub fn decode_response(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    let BuiltinValue::Bytes(bytes) = &args[0] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Bytes,
            actual: args[0].type_tag(),
        });
    };
    match decode_response_incremental(bytes, true) {
        DecodeStatus::Complete { value, .. } => Ok(BuiltinValue::ResultOk(Box::new(
            BuiltinValue::Record(vec![
                ("head".to_owned(), value.head.into_value()),
                ("body".to_owned(), BuiltinValue::Bytes(value.body)),
            ]),
        ))),
        DecodeStatus::NeedMore | DecodeStatus::Malformed { .. } => Ok(codec_error()),
    }
}

#[derive(Debug)]
struct HttpWireRequest {
    method: String,
    target: String,
    version: String,
    headers: Vec<HttpHeader>,
    body: Vec<u8>,
}

impl HttpWireRequest {
    fn from_value(value: &BuiltinValue) -> Result<Self, BuiltinError> {
        let BuiltinValue::Record(fields) = value else {
            return Err(BuiltinError::TypeMismatch {
                expected: BuiltinTypeTag::Record,
                actual: value.type_tag(),
            });
        };
        let target = match record_field(fields, "target") {
            Some(_) => string_field(fields, "target")?,
            None => string_field(fields, "path")?,
        };
        Ok(Self {
            method: string_field(fields, "method")?.to_owned(),
            target: target.to_owned(),
            version: optional_string_field(fields, "version")?
                .unwrap_or("HTTP/1.1")
                .to_owned(),
            headers: optional_headers_field(fields, "headers")?.unwrap_or_default(),
            body: optional_bytes_field(fields, "body")?
                .unwrap_or_default()
                .to_vec(),
        })
    }

    fn encode(&self) -> Result<Vec<u8>, ()> {
        validate_token(&self.method)?;
        validate_version(&self.version)?;
        validate_no_newline(&self.target)?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.method.as_bytes());
        bytes.push(b' ');
        bytes.extend_from_slice(self.target.as_bytes());
        bytes.push(b' ');
        bytes.extend_from_slice(self.version.as_bytes());
        bytes.extend_from_slice(b"\r\n");
        let mut has_content_length = false;
        for header in &self.headers {
            header.validate()?;
            if header.name.eq_ignore_ascii_case("content-length") {
                has_content_length = true;
            }
            bytes.extend_from_slice(header.name.as_bytes());
            bytes.extend_from_slice(b": ");
            bytes.extend_from_slice(header.value.as_bytes());
            bytes.extend_from_slice(b"\r\n");
        }
        if !self.body.is_empty() && !has_content_length {
            bytes.extend_from_slice(b"content-length: ");
            bytes.extend_from_slice(self.body.len().to_string().as_bytes());
            bytes.extend_from_slice(b"\r\n");
        }
        bytes.extend_from_slice(b"\r\n");
        bytes.extend_from_slice(&self.body);
        Ok(bytes)
    }
}

impl HttpWireResponseHead {
    fn into_value(self) -> BuiltinValue {
        BuiltinValue::Record(vec![
            ("version".to_owned(), BuiltinValue::String(self.version)),
            (
                "status".to_owned(),
                BuiltinValue::I32(i32::from(self.status)),
            ),
            ("reason".to_owned(), BuiltinValue::String(self.reason)),
            (
                "headers".to_owned(),
                BuiltinValue::List(
                    self.headers
                        .into_iter()
                        .map(HttpHeader::into_value)
                        .collect(),
                ),
            ),
        ])
    }
}

impl HttpHeader {
    fn validate(&self) -> Result<(), ()> {
        validate_token(&self.name)?;
        validate_no_newline(&self.value)
    }

    fn from_value(value: &BuiltinValue) -> Result<Self, BuiltinError> {
        let BuiltinValue::Record(fields) = value else {
            return Err(BuiltinError::TypeMismatch {
                expected: BuiltinTypeTag::Record,
                actual: value.type_tag(),
            });
        };
        Ok(Self {
            name: string_field(fields, "name")?.to_owned(),
            value: string_field(fields, "value")?.to_owned(),
        })
    }

    fn into_value(self) -> BuiltinValue {
        BuiltinValue::Record(vec![
            ("name".to_owned(), BuiltinValue::String(self.name)),
            ("value".to_owned(), BuiltinValue::String(self.value)),
        ])
    }
}

fn string_field<'a>(
    fields: &'a [(String, BuiltinValue)],
    name: &str,
) -> Result<&'a str, BuiltinError> {
    match record_field(fields, name) {
        Some(BuiltinValue::String(value)) => Ok(value),
        Some(other) => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: other.type_tag(),
        }),
        None => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: BuiltinTypeTag::Unit,
        }),
    }
}

fn optional_string_field<'a>(
    fields: &'a [(String, BuiltinValue)],
    name: &str,
) -> Result<Option<&'a str>, BuiltinError> {
    match record_field(fields, name) {
        Some(BuiltinValue::String(value)) => Ok(Some(value)),
        Some(other) => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: other.type_tag(),
        }),
        None => Ok(None),
    }
}

fn optional_bytes_field<'a>(
    fields: &'a [(String, BuiltinValue)],
    name: &str,
) -> Result<Option<&'a [u8]>, BuiltinError> {
    match record_field(fields, name) {
        Some(BuiltinValue::Bytes(value)) => Ok(Some(value)),
        Some(other) => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Bytes,
            actual: other.type_tag(),
        }),
        None => Ok(None),
    }
}

fn optional_headers_field(
    fields: &[(String, BuiltinValue)],
    name: &str,
) -> Result<Option<Vec<HttpHeader>>, BuiltinError> {
    match record_field(fields, name) {
        Some(BuiltinValue::List(headers)) | Some(BuiltinValue::Array(headers)) => headers
            .iter()
            .map(HttpHeader::from_value)
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(other) => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::List,
            actual: other.type_tag(),
        }),
        None => Ok(None),
    }
}

fn record_field<'a>(fields: &'a [(String, BuiltinValue)], name: &str) -> Option<&'a BuiltinValue> {
    fields
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value))
}

fn validate_token(value: &str) -> Result<(), ()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && !b"()<>@,;:\\\"/[]?={} \t".contains(&byte))
    {
        return Err(());
    }
    Ok(())
}

fn validate_version(value: &str) -> Result<(), ()> {
    if value == "HTTP/1.0" || value == "HTTP/1.1" {
        Ok(())
    } else {
        Err(())
    }
}

fn validate_no_newline(value: &str) -> Result<(), ()> {
    if value.bytes().any(|byte| matches!(byte, b'\r' | b'\n')) {
        Err(())
    } else {
        Ok(())
    }
}

fn codec_error() -> BuiltinValue {
    BuiltinValue::ResultErr(Box::new(BuiltinValue::Variant {
        name: "MalformedMessage".to_owned(),
        fields: Vec::new(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_request_accepts_structured_http_wire_request() {
        let request = BuiltinValue::Record(vec![
            ("method".to_owned(), BuiltinValue::String("POST".to_owned())),
            (
                "target".to_owned(),
                BuiltinValue::String("/submit".to_owned()),
            ),
            (
                "headers".to_owned(),
                BuiltinValue::List(vec![BuiltinValue::Record(vec![
                    ("name".to_owned(), BuiltinValue::String("host".to_owned())),
                    (
                        "value".to_owned(),
                        BuiltinValue::String("example.test".to_owned()),
                    ),
                ])]),
            ),
            ("body".to_owned(), BuiltinValue::Bytes(b"body".to_vec())),
        ]);

        let encoded = encode_request(&[request]).expect("request should encode");

        let BuiltinValue::ResultOk(value) = encoded else {
            panic!("expected successful HTTP request encoding");
        };
        let BuiltinValue::Bytes(bytes) = *value else {
            panic!("encoded request should be bytes");
        };
        let text = String::from_utf8(bytes).expect("request should be UTF-8 test data");
        assert!(text.starts_with("POST /submit HTTP/1.1\r\n"));
        assert!(text.contains("host: example.test\r\n"));
        assert!(text.contains("content-length: 4\r\n\r\nbody"));
    }

    #[test]
    fn decode_response_head_returns_structured_http_wire_head() {
        let bytes = b"HTTP/1.1 204 No Content\r\nserver: fixture\r\n\r\nignored body".to_vec();

        let decoded = decode_response_head(&[BuiltinValue::Bytes(bytes)])
            .expect("response head should decode");

        let BuiltinValue::ResultOk(value) = decoded else {
            panic!("expected successful HTTP response head decode");
        };
        let BuiltinValue::Record(fields) = *value else {
            panic!("decoded response head should be a record");
        };
        assert_eq!(
            record_field(&fields, "version"),
            Some(&BuiltinValue::String("HTTP/1.1".to_owned()))
        );
        assert_eq!(
            record_field(&fields, "status"),
            Some(&BuiltinValue::I32(204))
        );
        assert_eq!(
            record_field(&fields, "reason"),
            Some(&BuiltinValue::String("No Content".to_owned()))
        );
    }

    #[test]
    fn decode_response_returns_structured_http_wire_response() {
        let bytes = b"HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\n\r\nhello".to_vec();

        let decoded =
            decode_response(&[BuiltinValue::Bytes(bytes)]).expect("response should decode");

        let BuiltinValue::ResultOk(value) = decoded else {
            panic!("expected successful HTTP response decode");
        };
        let BuiltinValue::Record(fields) = *value else {
            panic!("decoded response should be a record");
        };
        let Some(BuiltinValue::Record(head)) = record_field(&fields, "head") else {
            panic!("decoded response should contain a structured head");
        };
        assert_eq!(record_field(head, "status"), Some(&BuiltinValue::I32(200)));
        assert_eq!(
            record_field(&fields, "body"),
            Some(&BuiltinValue::Bytes(b"hello".to_vec()))
        );
    }

    #[test]
    fn decode_response_decodes_chunked_transfer_body() {
        let bytes = b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n5\r\nhello\r\n6;ext=value\r\n world\r\n0\r\nx-trailer: ignored\r\n\r\n".to_vec();

        let fields = decoded_response_fields(bytes);

        assert_eq!(
            record_field(&fields, "body"),
            Some(&BuiltinValue::Bytes(b"hello world".to_vec()))
        );
    }

    #[test]
    fn decode_response_rejects_invalid_chunk_size() {
        assert_decode_response_codec_error(
            b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\nnot-hex\r\nhello\r\n0\r\n\r\n"
                .to_vec(),
        );
    }

    #[test]
    fn decode_response_rejects_missing_chunk_terminator() {
        assert_decode_response_codec_error(
            b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n".to_vec(),
        );
    }

    #[test]
    fn incremental_decoder_preserves_pipelined_bytes_after_chunked_response() {
        let first = b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n";
        let second = b"HTTP/1.1 204 No Content\r\n\r\n";
        let bytes = [first.as_slice(), second.as_slice()].concat();

        let DecodeStatus::Complete { value, consumed } = decode_response_incremental(&bytes, false)
        else {
            panic!("first framed response should be complete");
        };

        assert_eq!(value.body, b"hello");
        assert_eq!(consumed, first.len());
        assert_eq!(&bytes[consumed..], second);
    }

    #[test]
    fn decode_response_rejects_oversized_declared_chunk() {
        assert_decode_response_codec_error(
            b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\nA\r\nshort\r\n0\r\n\r\n"
                .to_vec(),
        );
    }

    #[test]
    fn decode_response_rejects_unsupported_transfer_encoding() {
        assert_decode_response_codec_error(
            b"HTTP/1.1 200 OK\r\ntransfer-encoding: gzip, chunked\r\n\r\n0\r\n\r\n".to_vec(),
        );
    }

    #[test]
    fn incremental_decoder_distinguishes_need_more_from_invalid_line_endings() {
        assert_eq!(
            decode_response_incremental(b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\n", false),
            DecodeStatus::NeedMore
        );
        assert!(matches!(
            decode_response_incremental(b"HTTP/1.1 200 OK\n\n", false),
            DecodeStatus::Malformed {
                kind: HttpCodecFailureKind::InvalidLineEnding,
                ..
            }
        ));
    }

    #[test]
    fn incremental_content_length_response_consumes_exact_frame() {
        let first = b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\n\r\nhello";
        let second = b"HTTP/1.1 204 No Content\r\n\r\n";
        let bytes = [first.as_slice(), second.as_slice()].concat();

        let DecodeStatus::Complete { value, consumed } = decode_response_incremental(&bytes, false)
        else {
            panic!("content-length framed response should be complete");
        };

        assert_eq!(value.body, b"hello");
        assert_eq!(consumed, first.len());
        assert_eq!(&bytes[consumed..], second);
    }

    #[test]
    fn incremental_decoder_waits_for_complete_content_length_body() {
        assert_eq!(
            decode_response_incremental(b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\n\r\nhell", false,),
            DecodeStatus::NeedMore
        );
        assert!(matches!(
            decode_response_incremental(b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\n\r\nhell", true,),
            DecodeStatus::Malformed {
                kind: HttpCodecFailureKind::UnexpectedEof,
                ..
            }
        ));
    }

    #[test]
    fn incremental_decoder_rejects_ambiguous_message_framing() {
        assert!(matches!(
            decode_response_incremental(
                b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\ncontent-length: 6\r\n\r\nhello!",
                true,
            ),
            DecodeStatus::Malformed {
                kind: HttpCodecFailureKind::ConflictingContentLength,
                ..
            }
        ));
        assert!(matches!(
            decode_response_incremental(
                b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\ncontent-length: 5\r\n\r\n0\r\n\r\n",
                true,
            ),
            DecodeStatus::Malformed {
                kind: HttpCodecFailureKind::ConflictingMessageFraming,
                ..
            }
        ));
    }

    #[test]
    fn incremental_decoder_rejects_http_2_and_http_3_text_framing() {
        for version in ["HTTP/2", "HTTP/3"] {
            let message = format!("{version} 200 OK\r\n\r\n");
            assert!(matches!(
                decode_response_incremental(message.as_bytes(), true),
                DecodeStatus::Malformed {
                    kind: HttpCodecFailureKind::UnsupportedHttpVersion,
                    ..
                }
            ));
        }
    }

    #[test]
    fn close_delimited_body_only_completes_at_end_of_stream() {
        let bytes = b"HTTP/1.1 200 OK\r\n\r\nhello";
        assert_eq!(
            decode_response_incremental(bytes, false),
            DecodeStatus::NeedMore
        );
        let DecodeStatus::Complete { value, consumed } = decode_response_incremental(bytes, true)
        else {
            panic!("EOF should complete a close-delimited response");
        };
        assert_eq!(value.body, b"hello");
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn bodyless_status_finishes_at_header_boundary() {
        let first = b"HTTP/1.1 204 No Content\r\n\r\n";
        let second = b"HTTP/1.1 204 No Content\r\n\r\n";
        let bytes = [first.as_slice(), second.as_slice()].concat();

        let DecodeStatus::Complete { value, consumed } = decode_response_incremental(&bytes, false)
        else {
            panic!("204 response should complete at the header boundary");
        };

        assert!(value.body.is_empty());
        assert_eq!(consumed, first.len());
    }

    fn decoded_response_fields(bytes: Vec<u8>) -> Vec<(String, BuiltinValue)> {
        let decoded =
            decode_response(&[BuiltinValue::Bytes(bytes)]).expect("response should decode");
        let BuiltinValue::ResultOk(value) = decoded else {
            panic!("expected successful HTTP response decode");
        };
        let BuiltinValue::Record(fields) = *value else {
            panic!("decoded response should be a record");
        };
        fields
    }

    fn assert_decode_response_codec_error(bytes: Vec<u8>) {
        let decoded = decode_response(&[BuiltinValue::Bytes(bytes)])
            .expect("codec errors are represented as Result.Err");
        let BuiltinValue::ResultErr(value) = decoded else {
            panic!("expected HTTP codec error");
        };
        let BuiltinValue::Variant { name, fields } = *value else {
            panic!("expected HttpCodecError variant");
        };
        assert_eq!(name, "MalformedMessage");
        assert!(fields.is_empty());
    }
}
