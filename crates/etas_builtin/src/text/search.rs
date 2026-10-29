use super::input::TextInput;
use crate::BuiltinError;

enum PatternBytes<'a> {
    Flat(&'a [u8]),
    Chunks(Vec<PatternChunk<'a>>),
}

struct PatternChunk<'a> {
    end: usize,
    bytes: &'a [u8],
}

impl PatternBytes<'_> {
    fn at(&self, index: usize) -> u8 {
        match self {
            Self::Flat(bytes) => bytes[index],
            Self::Chunks(chunks) => {
                let slot = chunks.partition_point(|chunk| chunk.end <= index);
                let chunk = &chunks[slot];
                chunk.bytes[index - (chunk.end - chunk.bytes.len())]
            }
        }
    }
}

/// KMP prefix lengths plus borrowed chunk headers, never flattened pattern bytes.
pub(super) struct PatternIndex<'a> {
    bytes: PatternBytes<'a>,
    prefix: Vec<usize>,
}

impl<'a> PatternIndex<'a> {
    pub(super) fn new<T: TextInput + ?Sized>(pattern: &'a T) -> Result<Self, BuiltinError> {
        let length = pattern.byte_len();
        let bytes = if let Some(text) = pattern.contiguous() {
            if text.len() != length {
                return Err(invalid_projection());
            }
            PatternBytes::Flat(text.as_bytes())
        } else {
            let chunks = pattern.chunks();
            let mut indexed = Vec::new();
            indexed
                .try_reserve_exact(chunks.size_hint().0)
                .map_err(|_| BuiltinError::NumericOverflow)?;
            let mut end = 0usize;
            for chunk in chunks.filter(|chunk| !chunk.is_empty()) {
                end = end
                    .checked_add(chunk.len())
                    .ok_or(BuiltinError::NumericOverflow)?;
                indexed.push(PatternChunk {
                    end,
                    bytes: chunk.as_bytes(),
                });
            }
            if end != length {
                return Err(invalid_projection());
            }
            PatternBytes::Chunks(indexed)
        };
        let mut prefix = Vec::new();
        prefix
            .try_reserve_exact(length)
            .map_err(|_| BuiltinError::NumericOverflow)?;
        prefix.resize(length, 0usize);
        let mut matched = 0;
        for index in 1..length {
            while matched > 0 && bytes.at(matched) != bytes.at(index) {
                matched = prefix[matched - 1];
            }
            if bytes.at(matched) == bytes.at(index) {
                matched += 1;
            }
            prefix[index] = matched;
        }
        Ok(Self { bytes, prefix })
    }

    pub(super) fn len(&self) -> usize {
        self.prefix.len()
    }

    /// Returns each non-overlapping match, as required by str::split.
    pub(super) fn advance(&self, matched: &mut usize, byte: u8) -> bool {
        if self.prefix.is_empty() {
            return true;
        }
        while *matched > 0 && self.bytes.at(*matched) != byte {
            *matched = self.prefix[*matched - 1];
        }
        if self.bytes.at(*matched) == byte {
            *matched += 1;
        }
        if *matched == self.len() {
            *matched = 0;
            true
        } else {
            false
        }
    }
}

fn invalid_projection() -> BuiltinError {
    BuiltinError::Abort {
        message: "invalid text projection length".into(),
    }
}
