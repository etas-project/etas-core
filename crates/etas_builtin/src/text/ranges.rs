use std::ops::Range;

use super::{input::TextInput, search::PatternIndex};
use crate::BuiltinError;

/// Byte ranges always delimit complete UTF-8 scalar values in the input.
pub fn trim_range<T: TextInput + ?Sized>(text: &T) -> Range<usize> {
    if let Some(text) = text.contiguous() {
        let remaining = text.trim_start();
        let start = text.len() - remaining.len();
        return start..start + remaining.trim_end().len();
    }
    let mut chars = text.chunks().flat_map(str::chars);
    let mut start = 0;
    for ch in chars.by_ref() {
        if !ch.is_whitespace() {
            break;
        }
        start += ch.len_utf8();
    }
    if start == text.byte_len() {
        return start..start;
    }
    let mut end = text.byte_len();
    for ch in chars.rev() {
        if !ch.is_whitespace() {
            break;
        }
        end -= ch.len_utf8();
    }
    start..end
}

struct Bytes<'a, C> {
    chunks: C,
    current: std::slice::Iter<'a, u8>,
}

impl<'a, C: Iterator<Item = &'a str>> Bytes<'a, C> {
    fn new(chunks: C) -> Self {
        Self {
            chunks,
            current: [].iter(),
        }
    }
}

impl<'a, C: Iterator<Item = &'a str>> Iterator for Bytes<'a, C> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        loop {
            if let Some(byte) = self.current.next() {
                return Some(*byte);
            }
            self.current = self.chunks.next()?.as_bytes().iter();
        }
    }
}

pub struct LineRanges<'a, C> {
    bytes: Bytes<'a, C>,
    flat: Option<(&'a str, std::str::Lines<'a>)>,
    start: usize,
    position: usize,
    finished: bool,
}

pub fn line_ranges<T: TextInput + ?Sized>(text: &T) -> LineRanges<'_, T::Chunks<'_>> {
    LineRanges {
        bytes: Bytes::new(text.chunks()),
        flat: text.contiguous().map(|text| (text, text.lines())),
        start: 0,
        position: 0,
        finished: false,
    }
}

impl<'a, C: Iterator<Item = &'a str>> Iterator for LineRanges<'a, C> {
    type Item = Range<usize>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        if let Some((text, lines)) = &mut self.flat {
            let line = lines.next()?;
            let end = self.start + line.len();
            let remaining = &text[end..];
            let delimiter = if remaining.starts_with("\r\n") {
                2
            } else {
                usize::from(remaining.starts_with('\n'))
            };
            let range = self.start..end;
            self.start = end + delimiter;
            return Some(range);
        }
        let mut previous = None;
        for byte in self.bytes.by_ref() {
            let position = self.position;
            self.position += 1;
            if byte == b'\n' {
                let end = position - usize::from(previous == Some(b'\r'));
                let range = self.start..end;
                self.start = self.position;
                return Some(range);
            }
            previous = Some(byte);
        }
        self.finished = true;
        (self.start < self.position).then_some(self.start..self.position)
    }
}

type Chars<'a, C> = std::iter::FlatMap<C, std::str::Chars<'a>, fn(&'a str) -> std::str::Chars<'a>>;

pub struct SplitRanges<'a, C> {
    source: SplitSource<'a, C>,
    start: usize,
    position: usize,
    finished: bool,
}

enum SplitSource<'a, C> {
    Flat {
        parts: std::str::Split<'a, &'a str>,
        separator_bytes: usize,
    },
    Empty {
        chars: Chars<'a, C>,
        leading: bool,
    },
    Pattern {
        bytes: Bytes<'a, C>,
        pattern: PatternIndex<'a>,
        matched: usize,
    },
}

pub fn split_ranges<'a, T: TextInput + ?Sized>(
    text: &'a T,
    pattern: &'a T,
) -> Result<SplitRanges<'a, T::Chunks<'a>>, BuiltinError> {
    let source = if pattern.byte_len() == 0 {
        SplitSource::Empty {
            chars: text
                .chunks()
                .flat_map(str::chars as fn(&'a str) -> std::str::Chars<'a>),
            leading: true,
        }
    } else if let (Some(text), Some(pattern)) = (text.contiguous(), pattern.contiguous()) {
        SplitSource::Flat {
            parts: text.split(pattern),
            separator_bytes: pattern.len(),
        }
    } else {
        SplitSource::Pattern {
            bytes: Bytes::new(text.chunks()),
            pattern: PatternIndex::new(pattern)?,
            matched: 0,
        }
    };
    Ok(SplitRanges {
        source,
        start: 0,
        position: 0,
        finished: false,
    })
}

impl<'a, C: Iterator<Item = &'a str>> Iterator for SplitRanges<'a, C> {
    type Item = Range<usize>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        match &mut self.source {
            SplitSource::Flat {
                parts,
                separator_bytes,
            } => {
                let part = parts.next()?;
                let end = self.start + part.len();
                let range = self.start..end;
                self.start = end + *separator_bytes;
                Some(range)
            }
            SplitSource::Empty { chars, leading } => {
                if std::mem::take(leading) {
                    return Some(0..0);
                }
                if let Some(ch) = chars.next() {
                    let start = self.position;
                    self.position += ch.len_utf8();
                    return Some(start..self.position);
                }
                self.finished = true;
                Some(self.position..self.position)
            }
            SplitSource::Pattern {
                bytes,
                pattern,
                matched,
            } => {
                for byte in bytes.by_ref() {
                    self.position += 1;
                    if pattern.advance(matched, byte) {
                        let range = self.start..self.position - pattern.len();
                        self.start = self.position;
                        return Some(range);
                    }
                }
                self.finished = true;
                Some(self.start..self.position)
            }
        }
    }
}

#[cfg(test)]
mod tests;
