mod chunk_line;
#[cfg(test)]
mod tests;

use super::incremental::{
    ParseStep, is_token_byte, parse_header, parse_status_line, read_crlf_line,
};
use super::{DecodeStatus, HttpCodecFailureKind, HttpWireResponseHead};

pub fn decode_response_head_prefix(
    bytes: &[u8],
    limit: usize,
) -> DecodeStatus<HttpWireResponseHead> {
    let bounded = &bytes[..bytes.len().min(limit)];
    let (line, mut offset) = match read_crlf_line(bounded, 0) {
        ParseStep::Complete(value) => value,
        ParseStep::Malformed { kind, offset } => return DecodeStatus::Malformed { kind, offset },
        ParseStep::NeedMore => {
            if let Some((kind, offset)) = invalid_status_prefix(partial_line(bounded), false) {
                return DecodeStatus::Malformed { kind, offset };
            }
            return need_more_or_limit(bytes, limit);
        }
    };
    if let Some((kind, offset)) = invalid_status_prefix(line, true) {
        return DecodeStatus::Malformed { kind, offset };
    }
    let (version, status, reason) = match parse_status_line(line) {
        Ok(value) => value,
        Err((kind, offset)) => return DecodeStatus::Malformed { kind, offset },
    };
    let mut headers = Vec::new();
    loop {
        match read_crlf_line(bounded, offset) {
            ParseStep::Complete((line, next)) => {
                if line.is_empty() {
                    return DecodeStatus::Complete {
                        value: HttpWireResponseHead {
                            version,
                            status,
                            reason,
                            headers,
                        },
                        consumed: next,
                    };
                }
                match parse_header(line) {
                    Ok(header) => headers.push(header),
                    Err(kind) => return DecodeStatus::Malformed { kind, offset },
                }
                offset = next;
            }
            ParseStep::Malformed { kind, offset } => {
                return DecodeStatus::Malformed { kind, offset };
            }
            ParseStep::NeedMore => {
                if let Some(index) = invalid_header_prefix(partial_line(&bounded[offset..])) {
                    return DecodeStatus::Malformed {
                        kind: HttpCodecFailureKind::InvalidHeader,
                        offset: offset + index,
                    };
                }
                return need_more_or_limit(bytes, limit);
            }
        }
    }
}

pub fn decode_chunk_size_line_prefix(bytes: &[u8], limit: usize) -> DecodeStatus<u64> {
    let bounded = &bytes[..bytes.len().min(limit)];
    let (line, consumed) = match read_crlf_line(bounded, 0) {
        ParseStep::Complete((line, consumed)) => (line, Some(consumed)),
        ParseStep::Malformed { kind, offset } => return DecodeStatus::Malformed { kind, offset },
        ParseStep::NeedMore => (partial_line(bounded), None),
    };
    match chunk_line::parse(line, consumed.is_some()) {
        ParseStep::Complete(value) => match consumed {
            Some(consumed) => DecodeStatus::Complete { value, consumed },
            None => need_more_or_limit(bytes, limit),
        },
        ParseStep::NeedMore => need_more_or_limit(bytes, limit),
        ParseStep::Malformed { kind, offset } => DecodeStatus::Malformed { kind, offset },
    }
}

fn need_more_or_limit<T>(bytes: &[u8], limit: usize) -> DecodeStatus<T> {
    if bytes.len() >= limit {
        DecodeStatus::Malformed {
            kind: HttpCodecFailureKind::LimitExceeded,
            offset: limit,
        }
    } else {
        DecodeStatus::NeedMore
    }
}

fn partial_line(bytes: &[u8]) -> &[u8] {
    bytes.strip_suffix(b"\r").unwrap_or(bytes)
}

fn invalid_status_prefix(line: &[u8], complete: bool) -> Option<(HttpCodecFailureKind, usize)> {
    let version_len = line.len().min(9);
    if ![b"HTTP/1.0 ", b"HTTP/1.1 "]
        .iter()
        .any(|version| version[..version_len] == line[..version_len])
    {
        return Some((HttpCodecFailureKind::UnsupportedHttpVersion, 0));
    }
    for (index, byte) in line.iter().enumerate().skip(9).take(3) {
        if !byte.is_ascii_digit() || (index == 9 && *byte == b'0') {
            return Some((HttpCodecFailureKind::InvalidStatusCode, index));
        }
    }
    if line.get(12).is_some_and(|byte| *byte != b' ') || (complete && line.len() < 13) {
        return Some((HttpCodecFailureKind::InvalidStatusLine, line.len().min(12)));
    }
    line.iter()
        .enumerate()
        .skip(13)
        .find(|(_, byte)| byte.is_ascii_control() && **byte != b'\t')
        .map(|(offset, _)| (HttpCodecFailureKind::InvalidStatusLine, offset))
}

fn invalid_header_prefix(line: &[u8]) -> Option<usize> {
    let mut value = false;
    for (offset, byte) in line.iter().copied().enumerate() {
        if !value && byte == b':' {
            if offset == 0 {
                return Some(offset);
            }
            value = true;
        } else if (!value && !is_token_byte(byte))
            || (value && byte.is_ascii_control() && byte != b'\t')
        {
            return Some(offset);
        }
    }
    None
}
