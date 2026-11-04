use std::ops::Range;

use super::{input::TextInput, search::PatternIndex, unicode};
use crate::{
    BuiltinError,
    work::{WorkControl, WorkPoll},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextTrimKind {
    Both,
    Start,
    End,
}

#[derive(Clone, Debug)]
pub struct TextTrimWork {
    kind: TextTrimKind,
    position: usize,
    first_non_whitespace: Option<usize>,
    last_non_whitespace_end: Option<usize>,
}

impl TextTrimWork {
    pub fn new(kind: TextTrimKind) -> Self {
        Self {
            kind,
            position: 0,
            first_non_whitespace: None,
            last_non_whitespace_end: None,
        }
    }

    pub fn step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> (usize, WorkPoll<Range<usize>>) {
        let mut consumed = 0usize;
        while let Some(ch) = input[consumed..].chars().next() {
            let width = ch.len_utf8();
            if control.consume(width).is_err() {
                return (consumed, WorkPoll::Pending);
            }
            if !unicode::is_whitespace(ch) {
                self.first_non_whitespace.get_or_insert(self.position);
                self.last_non_whitespace_end = Some(self.position + width);
            }
            self.position += width;
            consumed += width;
        }
        if !end_of_input {
            return (consumed, WorkPoll::Pending);
        }
        let range = match self.kind {
            TextTrimKind::Both => {
                let start = self.first_non_whitespace.unwrap_or(self.position);
                start..self.last_non_whitespace_end.unwrap_or(start)
            }
            TextTrimKind::Start => {
                self.first_non_whitespace.unwrap_or(self.position)..self.position
            }
            TextTrimKind::End => 0..self.last_non_whitespace_end.unwrap_or(0),
        };
        (consumed, WorkPoll::Ready(range))
    }
}

/// Byte ranges always delimit complete UTF-8 scalar values in the input.
pub fn trim_range<T: TextInput + ?Sized>(text: &T) -> Range<usize> {
    trim_range_kind(text, TextTrimKind::Both)
}

pub fn trim_range_kind<T: TextInput + ?Sized>(text: &T, kind: TextTrimKind) -> Range<usize> {
    let mut position = 0usize;
    let mut first_non_whitespace = None;
    let mut last_non_whitespace_end = None;
    for scalar in text.chunks().flat_map(str::chars) {
        let width = scalar.len_utf8();
        if !unicode::is_whitespace(scalar) {
            first_non_whitespace.get_or_insert(position);
            last_non_whitespace_end = Some(position + width);
        }
        position += width;
    }
    match kind {
        TextTrimKind::Both => {
            let start = first_non_whitespace.unwrap_or(position);
            start..last_non_whitespace_end.unwrap_or(start)
        }
        TextTrimKind::Start => first_non_whitespace.unwrap_or(position)..position,
        TextTrimKind::End => 0..last_non_whitespace_end.unwrap_or(0),
    }
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
