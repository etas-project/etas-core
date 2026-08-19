#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeStatus<T> {
    NeedMore,
    Complete {
        value: T,
        consumed: usize,
    },
    Malformed {
        kind: HttpCodecFailureKind,
        offset: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpCodecFailureKind {
    UnexpectedEof,
    InvalidLineEnding,
    InvalidStatusLine,
    UnsupportedHttpVersion,
    InvalidStatusCode,
    InvalidHeader,
    InvalidContentLength,
    ConflictingContentLength,
    ConflictingMessageFraming,
    UnsupportedTransferEncoding,
    ForbiddenResponseBody,
    InvalidChunkSize,
    InvalidChunkTerminator,
    InvalidTrailer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpWireResponse {
    pub head: HttpWireResponseHead,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpWireResponseHead {
    pub version: String,
    pub status: u16,
    pub reason: String,
    pub headers: Vec<HttpHeader>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

pub fn decode_response_head_incremental(
    bytes: &[u8],
    end_of_stream: bool,
) -> DecodeStatus<HttpWireResponseHead> {
    let head_end = match find_head_end(bytes) {
        ParseStep::NeedMore => return incomplete(end_of_stream, bytes.len()),
        ParseStep::Malformed { kind, offset } => return DecodeStatus::Malformed { kind, offset },
        ParseStep::Complete(end) => end,
    };
    match parse_response_head(&bytes[..head_end]) {
        Ok(head) => DecodeStatus::Complete {
            value: head,
            consumed: head_end,
        },
        Err((kind, offset)) => DecodeStatus::Malformed { kind, offset },
    }
}

pub fn decode_response_incremental(
    bytes: &[u8],
    end_of_stream: bool,
) -> DecodeStatus<HttpWireResponse> {
    let (head, head_end) = match decode_response_head_incremental(bytes, end_of_stream) {
        DecodeStatus::NeedMore => return DecodeStatus::NeedMore,
        DecodeStatus::Malformed { kind, offset } => {
            return DecodeStatus::Malformed { kind, offset };
        }
        DecodeStatus::Complete { value, consumed } => (value, consumed),
    };

    let body_bytes = &bytes[head_end..];
    let no_body = (100..200).contains(&head.status) || matches!(head.status, 204 | 304);
    let transfer_encoding = match transfer_encoding(&head.headers) {
        Ok(value) => value,
        Err(kind) => {
            return DecodeStatus::Malformed {
                kind,
                offset: head_end,
            };
        }
    };
    let content_length = match content_length(&head.headers) {
        Ok(value) => value,
        Err(kind) => {
            return DecodeStatus::Malformed {
                kind,
                offset: head_end,
            };
        }
    };

    if transfer_encoding.is_some() && content_length.is_some() {
        return DecodeStatus::Malformed {
            kind: HttpCodecFailureKind::ConflictingMessageFraming,
            offset: head_end,
        };
    }
    if no_body {
        if transfer_encoding.is_some()
            || (head.status != 304 && content_length.is_some_and(|length| length != 0))
        {
            return DecodeStatus::Malformed {
                kind: HttpCodecFailureKind::ForbiddenResponseBody,
                offset: head_end,
            };
        }
        return DecodeStatus::Complete {
            value: HttpWireResponse {
                head,
                body: Vec::new(),
            },
            consumed: head_end,
        };
    }

    if transfer_encoding.is_some() {
        return match decode_chunked_body(body_bytes, end_of_stream) {
            DecodeStatus::NeedMore => DecodeStatus::NeedMore,
            DecodeStatus::Malformed { kind, offset } => DecodeStatus::Malformed {
                kind,
                offset: head_end + offset,
            },
            DecodeStatus::Complete { value, consumed } => DecodeStatus::Complete {
                value: HttpWireResponse { head, body: value },
                consumed: head_end + consumed,
            },
        };
    }

    if let Some(length) = content_length {
        if body_bytes.len() < length {
            return incomplete(end_of_stream, bytes.len());
        }
        return DecodeStatus::Complete {
            value: HttpWireResponse {
                head,
                body: body_bytes[..length].to_vec(),
            },
            consumed: head_end + length,
        };
    }

    if !end_of_stream {
        return DecodeStatus::NeedMore;
    }
    DecodeStatus::Complete {
        value: HttpWireResponse {
            head,
            body: body_bytes.to_vec(),
        },
        consumed: bytes.len(),
    }
}

enum ParseStep<T> {
    NeedMore,
    Complete(T),
    Malformed {
        kind: HttpCodecFailureKind,
        offset: usize,
    },
}

fn incomplete<T>(end_of_stream: bool, offset: usize) -> DecodeStatus<T> {
    if end_of_stream {
        DecodeStatus::Malformed {
            kind: HttpCodecFailureKind::UnexpectedEof,
            offset,
        }
    } else {
        DecodeStatus::NeedMore
    }
}

fn find_head_end(bytes: &[u8]) -> ParseStep<usize> {
    let mut offset = 0;
    while offset < bytes.len() {
        match bytes[offset] {
            b'\n' => {
                return ParseStep::Malformed {
                    kind: HttpCodecFailureKind::InvalidLineEnding,
                    offset,
                };
            }
            b'\r' => {
                let Some(next) = bytes.get(offset + 1) else {
                    return ParseStep::NeedMore;
                };
                if *next != b'\n' {
                    return ParseStep::Malformed {
                        kind: HttpCodecFailureKind::InvalidLineEnding,
                        offset,
                    };
                }
                if offset >= 2 && bytes[offset - 2..offset] == *b"\r\n" {
                    return ParseStep::Complete(offset + 2);
                }
                offset += 2;
            }
            _ => {
                offset += 1;
            }
        }
    }
    ParseStep::NeedMore
}

fn parse_response_head(
    bytes: &[u8],
) -> Result<HttpWireResponseHead, (HttpCodecFailureKind, usize)> {
    let Some(head_without_terminator) = bytes.strip_suffix(b"\r\n\r\n") else {
        return Err((HttpCodecFailureKind::InvalidLineEnding, bytes.len()));
    };
    let mut lines = head_without_terminator.split(|byte| *byte == b'\n');
    let status_line = lines
        .next()
        .ok_or((HttpCodecFailureKind::InvalidStatusLine, 0))?;
    let status_line = status_line.strip_suffix(b"\r").unwrap_or(status_line);
    let (version, status, reason) = parse_status_line(status_line)?;

    let mut headers = Vec::new();
    let mut offset = status_line.len() + 2;
    for raw_line in lines {
        let line = raw_line.strip_suffix(b"\r").unwrap_or(raw_line);
        let header = parse_header(line).map_err(|kind| (kind, offset))?;
        headers.push(header);
        offset += raw_line.len() + 1;
    }
    Ok(HttpWireResponseHead {
        version,
        status,
        reason,
        headers,
    })
}

fn parse_status_line(line: &[u8]) -> Result<(String, u16, String), (HttpCodecFailureKind, usize)> {
    let Some(first_space) = line.iter().position(|byte| *byte == b' ') else {
        return Err((HttpCodecFailureKind::InvalidStatusLine, 0));
    };
    let version = match &line[..first_space] {
        b"HTTP/1.0" => "HTTP/1.0",
        b"HTTP/1.1" => "HTTP/1.1",
        _ => return Err((HttpCodecFailureKind::UnsupportedHttpVersion, 0)),
    };
    let status_start = first_space + 1;
    let status_end = status_start + 3;
    let Some(status_bytes) = line.get(status_start..status_end) else {
        return Err((HttpCodecFailureKind::InvalidStatusCode, status_start));
    };
    if !status_bytes.iter().all(u8::is_ascii_digit)
        || line.get(status_end).is_some_and(|byte| *byte != b' ')
    {
        return Err((HttpCodecFailureKind::InvalidStatusCode, status_start));
    }
    let status = ((status_bytes[0] - b'0') as u16) * 100
        + ((status_bytes[1] - b'0') as u16) * 10
        + (status_bytes[2] - b'0') as u16;
    if !(100..=999).contains(&status) {
        return Err((HttpCodecFailureKind::InvalidStatusCode, status_start));
    }
    let reason_bytes = if line.len() > status_end {
        &line[status_end + 1..]
    } else {
        &[]
    };
    if reason_bytes
        .iter()
        .any(|byte| byte.is_ascii_control() && *byte != b'\t')
    {
        return Err((HttpCodecFailureKind::InvalidStatusLine, status_end + 1));
    }
    let reason = std::str::from_utf8(reason_bytes).map_err(|error| {
        (
            HttpCodecFailureKind::InvalidStatusLine,
            status_end + 1 + error.valid_up_to(),
        )
    })?;
    Ok((version.to_owned(), status, reason.to_owned()))
}

fn parse_header(line: &[u8]) -> Result<HttpHeader, HttpCodecFailureKind> {
    let Some(colon) = line.iter().position(|byte| *byte == b':') else {
        return Err(HttpCodecFailureKind::InvalidHeader);
    };
    let name = &line[..colon];
    if name.is_empty() || !name.iter().copied().all(is_token_byte) {
        return Err(HttpCodecFailureKind::InvalidHeader);
    }
    let value = trim_optional_whitespace(&line[colon + 1..]);
    if value
        .iter()
        .any(|byte| byte.is_ascii_control() && *byte != b'\t')
    {
        return Err(HttpCodecFailureKind::InvalidHeader);
    }
    Ok(HttpHeader {
        name: std::str::from_utf8(name)
            .map_err(|_| HttpCodecFailureKind::InvalidHeader)?
            .to_owned(),
        value: std::str::from_utf8(value)
            .map_err(|_| HttpCodecFailureKind::InvalidHeader)?
            .to_owned(),
    })
}

fn transfer_encoding(headers: &[HttpHeader]) -> Result<Option<()>, HttpCodecFailureKind> {
    let mut encodings = headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case("transfer-encoding"))
        .flat_map(|header| header.value.split(','))
        .map(str::trim)
        .filter(|encoding| !encoding.is_empty());
    let Some(first) = encodings.next() else {
        return Ok(None);
    };
    if !first.eq_ignore_ascii_case("chunked") || encodings.next().is_some() {
        return Err(HttpCodecFailureKind::UnsupportedTransferEncoding);
    }
    Ok(Some(()))
}

fn content_length(headers: &[HttpHeader]) -> Result<Option<usize>, HttpCodecFailureKind> {
    let mut parsed = None;
    for value in headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case("content-length"))
        .flat_map(|header| header.value.split(','))
    {
        let value = value.trim();
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(HttpCodecFailureKind::InvalidContentLength);
        }
        let length = value
            .parse::<usize>()
            .map_err(|_| HttpCodecFailureKind::InvalidContentLength)?;
        match parsed {
            Some(previous) if previous != length => {
                return Err(HttpCodecFailureKind::ConflictingContentLength);
            }
            _ => parsed = Some(length),
        }
    }
    Ok(parsed)
}

fn decode_chunked_body(bytes: &[u8], end_of_stream: bool) -> DecodeStatus<Vec<u8>> {
    let mut offset = 0;
    let mut decoded = Vec::new();
    loop {
        let (line, next) = match read_crlf_line(bytes, offset) {
            ParseStep::NeedMore => return incomplete(end_of_stream, bytes.len()),
            ParseStep::Malformed { kind, offset } => {
                return DecodeStatus::Malformed { kind, offset };
            }
            ParseStep::Complete(value) => value,
        };
        let size = match parse_chunk_size(line) {
            Ok(size) => size,
            Err(kind) => return DecodeStatus::Malformed { kind, offset },
        };
        offset = next;
        if size == 0 {
            loop {
                let (trailer, next) = match read_crlf_line(bytes, offset) {
                    ParseStep::NeedMore => return incomplete(end_of_stream, bytes.len()),
                    ParseStep::Malformed { kind, offset } => {
                        return DecodeStatus::Malformed { kind, offset };
                    }
                    ParseStep::Complete(value) => value,
                };
                offset = next;
                if trailer.is_empty() {
                    return DecodeStatus::Complete {
                        value: decoded,
                        consumed: offset,
                    };
                }
                let Ok(header) = parse_header(trailer) else {
                    return DecodeStatus::Malformed {
                        kind: HttpCodecFailureKind::InvalidTrailer,
                        offset: offset - trailer.len() - 2,
                    };
                };
                if matches!(
                    header.name.to_ascii_lowercase().as_str(),
                    "content-length" | "transfer-encoding" | "trailer"
                ) {
                    return DecodeStatus::Malformed {
                        kind: HttpCodecFailureKind::InvalidTrailer,
                        offset: offset - trailer.len() - 2,
                    };
                }
            }
        }
        let Some(data_end) = offset.checked_add(size) else {
            return DecodeStatus::Malformed {
                kind: HttpCodecFailureKind::InvalidChunkSize,
                offset,
            };
        };
        if data_end > bytes.len() {
            return incomplete(end_of_stream, bytes.len());
        }
        let terminator_end = data_end + 2;
        let Some(terminator) = bytes.get(data_end..terminator_end) else {
            return incomplete(end_of_stream, bytes.len());
        };
        if terminator != b"\r\n" {
            return DecodeStatus::Malformed {
                kind: HttpCodecFailureKind::InvalidChunkTerminator,
                offset: data_end,
            };
        }
        decoded.extend_from_slice(&bytes[offset..data_end]);
        offset = terminator_end;
    }
}

fn read_crlf_line(bytes: &[u8], start: usize) -> ParseStep<(&[u8], usize)> {
    let mut offset = start;
    while offset < bytes.len() {
        match bytes[offset] {
            b'\n' => {
                return ParseStep::Malformed {
                    kind: HttpCodecFailureKind::InvalidLineEnding,
                    offset,
                };
            }
            b'\r' => match bytes.get(offset + 1) {
                Some(b'\n') => return ParseStep::Complete((&bytes[start..offset], offset + 2)),
                Some(_) => {
                    return ParseStep::Malformed {
                        kind: HttpCodecFailureKind::InvalidLineEnding,
                        offset,
                    };
                }
                None => return ParseStep::NeedMore,
            },
            _ => offset += 1,
        }
    }
    ParseStep::NeedMore
}

fn parse_chunk_size(line: &[u8]) -> Result<usize, HttpCodecFailureKind> {
    let size = line.split(|byte| *byte == b';').next().unwrap_or_default();
    if size.is_empty() || !size.iter().all(u8::is_ascii_hexdigit) {
        return Err(HttpCodecFailureKind::InvalidChunkSize);
    }
    let size = std::str::from_utf8(size).map_err(|_| HttpCodecFailureKind::InvalidChunkSize)?;
    usize::from_str_radix(size, 16).map_err(|_| HttpCodecFailureKind::InvalidChunkSize)
}

fn trim_optional_whitespace(mut value: &[u8]) -> &[u8] {
    while value
        .first()
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        value = &value[1..];
    }
    while value
        .last()
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        value = &value[..value.len() - 1];
    }
    value
}

fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_graphic() && !b"()<>@,;:\\\"/[]?={} \t".contains(&byte)
}
