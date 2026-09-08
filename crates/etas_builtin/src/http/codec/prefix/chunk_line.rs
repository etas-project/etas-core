use super::{HttpCodecFailureKind, ParseStep, is_token_byte};

// RFC 9112 section 7.1.1: BWS is allowed around extension delimiters,
// and extension values are token or quoted-string, not arbitrary suffixes.
pub(super) fn parse(line: &[u8], complete: bool) -> ParseStep<u64> {
    let mut offset = 0;
    let mut size = 0u64;
    while let Some(digit) = line
        .get(offset)
        .and_then(|byte| (*byte as char).to_digit(16))
    {
        let Some(next) = size
            .checked_mul(16)
            .and_then(|value| value.checked_add(u64::from(digit)))
        else {
            return malformed(HttpCodecFailureKind::InvalidChunkSize, offset);
        };
        size = next;
        offset += 1;
    }
    if offset == 0 {
        return missing_or_invalid(
            line.is_empty() && !complete,
            HttpCodecFailureKind::InvalidChunkSize,
            0,
        );
    }
    while offset < line.len() {
        skip_bws(line, &mut offset);
        if offset == line.len() {
            return missing_or_invalid(
                !complete,
                HttpCodecFailureKind::InvalidChunkExtension,
                offset,
            );
        }
        if line[offset] != b';' {
            return malformed(HttpCodecFailureKind::InvalidChunkExtension, offset);
        }
        offset += 1;
        skip_bws(line, &mut offset);
        let start = offset;
        while line.get(offset).is_some_and(|byte| is_token_byte(*byte)) {
            offset += 1;
        }
        if start == offset {
            return missing_or_invalid(
                offset == line.len() && !complete,
                HttpCodecFailureKind::InvalidChunkExtension,
                offset,
            );
        }
        let before_bws = offset;
        skip_bws(line, &mut offset);
        if line.get(offset) == Some(&b'=') {
            offset += 1;
            skip_bws(line, &mut offset);
            if line.get(offset) == Some(&b'"') {
                offset += 1;
                loop {
                    let Some(byte) = line.get(offset).copied() else {
                        return missing_or_invalid(
                            !complete,
                            HttpCodecFailureKind::InvalidChunkExtension,
                            offset,
                        );
                    };
                    offset += 1;
                    match byte {
                        b'"' => break,
                        b'\\' => {
                            let Some(escaped) = line.get(offset).copied() else {
                                return missing_or_invalid(
                                    !complete,
                                    HttpCodecFailureKind::InvalidChunkExtension,
                                    offset,
                                );
                            };
                            if !matches!(escaped, b'\t' | b' '..=b'~' | 0x80..=0xff) {
                                return malformed(
                                    HttpCodecFailureKind::InvalidChunkExtension,
                                    offset,
                                );
                            }
                            offset += 1;
                        }
                        b'\t' | b' ' | b'!' | b'#'..=b'[' | b']'..=b'~' | 0x80..=0xff => {}
                        _ => {
                            return malformed(
                                HttpCodecFailureKind::InvalidChunkExtension,
                                offset - 1,
                            );
                        }
                    }
                }
            } else {
                let start = offset;
                while line.get(offset).is_some_and(|byte| is_token_byte(*byte)) {
                    offset += 1;
                }
                if start == offset {
                    return missing_or_invalid(
                        offset == line.len() && !complete,
                        HttpCodecFailureKind::InvalidChunkExtension,
                        offset,
                    );
                }
            }
        } else {
            // Preserve BWS for the next semicolon. It is not legal trailing data.
            offset = before_bws;
        }
    }
    if complete {
        ParseStep::Complete(size)
    } else {
        ParseStep::NeedMore
    }
}

fn skip_bws(line: &[u8], offset: &mut usize) {
    while line
        .get(*offset)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        *offset += 1;
    }
}

fn malformed(kind: HttpCodecFailureKind, offset: usize) -> ParseStep<u64> {
    ParseStep::Malformed { kind, offset }
}

fn missing_or_invalid(missing: bool, kind: HttpCodecFailureKind, offset: usize) -> ParseStep<u64> {
    if missing {
        ParseStep::NeedMore
    } else {
        malformed(kind, offset)
    }
}
