use super::*;

#[test]
fn head_prefix_preserves_body_and_pipeline_at_every_split() {
    let head = b"HTTP/1.1 200 OK\r\ncontent-length: 4\r\nx-test: value\r\n\r\n";
    for split in 0..head.len() {
        assert_eq!(
            decode_response_head_prefix(&head[..split], head.len()),
            DecodeStatus::NeedMore,
            "split {split}"
        );
    }
    let all = [head.as_slice(), b"bodyHTTP/1.1 204 No Content\r\n\r\n"].concat();
    let DecodeStatus::Complete { value, consumed } = decode_response_head_prefix(&all, head.len())
    else {
        panic!("complete head");
    };
    assert_eq!(value.status, 200);
    assert_eq!(value.headers.len(), 2);
    assert_eq!(consumed, head.len());
    assert_eq!(&all[consumed..], b"bodyHTTP/1.1 204 No Content\r\n\r\n");
    assert!(matches!(
        decode_response_head_prefix(&all, head.len() - 1),
        DecodeStatus::Malformed {
            kind: HttpCodecFailureKind::LimitExceeded,
            ..
        }
    ));
}

#[test]
fn head_prefix_rejects_invalid_complete_and_partial_lines() {
    for bytes in [
        b"WRONG".as_slice(),
        b"HTTP/1.1 x",
        b"HTTP/1.1 200\r\n\r\n",
        b"HTTP/1.1 200 OK\r\ninvalid header",
        b"HTTP/1.1 200 OK\n",
        b"HTTP/1.1 200 OK\r\n folded: no\r\n\r\n",
    ] {
        assert!(
            matches!(
                decode_response_head_prefix(bytes, 1024),
                DecodeStatus::Malformed { .. }
            ),
            "{bytes:?}"
        );
    }
    assert!(matches!(
        decode_response_head_prefix(b"", 0),
        DecodeStatus::Malformed {
            kind: HttpCodecFailureKind::LimitExceeded,
            ..
        }
    ));
}

#[test]
fn chunk_prefix_consumes_only_line_and_validates_extensions_at_every_split() {
    for line in [
        b"a\r\n".as_slice(),
        b"0;last\r\n",
        b"a ; name = token ; quoted=\"a;\\\"b\"\r\n",
    ] {
        for split in 0..line.len() {
            assert_eq!(
                decode_chunk_size_line_prefix(&line[..split], line.len()),
                DecodeStatus::NeedMore,
                "{line:?} split {split}"
            );
        }
        let all = [line, b"DATA\r\n0\r\n\r\n"].concat();
        let DecodeStatus::Complete { value, consumed } =
            decode_chunk_size_line_prefix(&all, line.len())
        else {
            panic!("complete chunk line: {line:?}");
        };
        assert_eq!(value, if line[0] == b'0' { 0 } else { 10 });
        assert_eq!(consumed, line.len());
        assert_eq!(&all[consumed..], b"DATA\r\n0\r\n\r\n");
        assert!(matches!(
            decode_chunk_size_line_prefix(&all, line.len() - 1),
            DecodeStatus::Malformed {
                kind: HttpCodecFailureKind::LimitExceeded,
                ..
            }
        ));
    }
}

#[test]
fn chunk_size_uses_fixed_u64_range_not_platform_usize() {
    assert_eq!(
        decode_chunk_size_line_prefix(b"ffffffffffffffff\r\n", 18),
        DecodeStatus::Complete {
            value: u64::MAX,
            consumed: 18
        }
    );
    assert_eq!(
        decode_chunk_size_line_prefix(b"100000000\r\n", 64),
        DecodeStatus::Complete {
            value: 4_294_967_296,
            consumed: 11
        }
    );
    assert!(matches!(
        decode_chunk_size_line_prefix(b"10000000000000000", 64),
        DecodeStatus::Malformed {
            kind: HttpCodecFailureKind::InvalidChunkSize,
            ..
        }
    ));
}

#[test]
fn chunk_prefix_rejects_malformed_extensions_and_size() {
    for line in [
        b"\r\n".as_slice(),
        b"g",
        b"-1\r\n",
        b"1;\r\n",
        b"1;x=\r\n",
        b"1;x=\"unterminated\r\n",
        b"1;x=\"bad\x00\"\r\n",
        b"1;x=token junk\r\n",
        b"1;=no\r\n",
        b"1 ; \r\n",
        b"1\n",
        b"1;x=\"a\"junk\r\n",
        b"1;x=\"bad\\\x01\"\r\n",
    ] {
        assert!(
            matches!(
                decode_chunk_size_line_prefix(line, 1024),
                DecodeStatus::Malformed { .. }
            ),
            "{line:?}"
        );
    }
    assert!(matches!(
        decode_chunk_size_line_prefix(b"", 0),
        DecodeStatus::Malformed {
            kind: HttpCodecFailureKind::LimitExceeded,
            ..
        }
    ));
}
