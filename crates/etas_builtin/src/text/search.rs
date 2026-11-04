use super::input::TextInput;
use crate::{
    BuiltinError, BuiltinValue,
    work::{WorkControl, WorkPoll, WorkStopReason},
};

use super::iteration::{
    TextCursorState, TextSequenceElement, TextSequenceKind, TextSequencePlan, TextSequenceStep,
    step,
};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextSearchKind {
    Contains,
    StartsWith,
    EndsWith,
    Find,
    Rfind,
    FindFrom { start: usize },
    Count,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextSearchPhase {
    Pattern,
    Prefix,
    Source,
    Complete,
}

#[derive(Clone, Debug)]
pub struct TextSearchWork {
    kind: TextSearchKind,
    phase: TextSearchPhase,
    expected_pattern_bytes: usize,
    pattern: Vec<u8>,
    prefix: Vec<usize>,
    prefix_index: usize,
    prefix_matched: usize,
    source_offset: usize,
    matched: usize,
    pending_byte: Option<u8>,
    last_match: Option<usize>,
    count: usize,
}

impl TextSearchWork {
    pub fn new(kind: TextSearchKind, pattern_bytes: usize) -> Self {
        Self {
            kind,
            phase: TextSearchPhase::Pattern,
            expected_pattern_bytes: pattern_bytes,
            pattern: Vec::new(),
            prefix: Vec::new(),
            prefix_index: 1,
            prefix_matched: 0,
            source_offset: 0,
            matched: 0,
            pending_byte: None,
            last_match: None,
            count: 0,
        }
    }

    pub fn reserved_scratch_bytes(&self) -> usize {
        self.pattern.capacity().saturating_add(
            self.prefix
                .capacity()
                .saturating_mul(std::mem::size_of::<usize>()),
        )
    }

    pub fn pattern_step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<()>), BuiltinError> {
        if self.phase == TextSearchPhase::Prefix {
            return self.prefix_step(control).map(|poll| (0, poll));
        }
        if self.phase != TextSearchPhase::Pattern {
            return Ok((0, WorkPoll::Ready(())));
        }
        let missing = self
            .expected_pattern_bytes
            .checked_sub(self.pattern.len())
            .ok_or_else(invalid_projection)?;
        let consumed = input.len().min(missing).min(control.work_remaining());
        if consumed != 0 {
            let scratch = consumed
                .checked_mul(1 + std::mem::size_of::<usize>())
                .ok_or(BuiltinError::NumericOverflow)?;
            control.reserve_scratch(scratch).map_err(work_limit_error)?;
            self.pattern
                .try_reserve_exact(consumed)
                .map_err(|_| BuiltinError::ResourceExhausted)?;
            self.prefix
                .try_reserve_exact(consumed)
                .map_err(|_| BuiltinError::ResourceExhausted)?;
            self.pattern
                .extend_from_slice(&input.as_bytes()[..consumed]);
            self.prefix.resize(self.prefix.len() + consumed, 0);
            control
                .consume(consumed)
                .expect("pattern collection is bounded by remaining work");
        }
        if end_of_input && consumed == input.len() {
            if self.pattern.len() != self.expected_pattern_bytes {
                return Err(invalid_projection());
            }
            self.phase = TextSearchPhase::Prefix;
            let poll = self.prefix_step(control)?;
            return Ok((consumed, poll));
        }
        Ok((consumed, WorkPoll::Pending))
    }

    fn prefix_step(&mut self, control: &mut WorkControl) -> Result<WorkPoll<()>, BuiltinError> {
        while self.prefix_index < self.pattern.len() {
            if control.consume(1) == Err(WorkStopReason::Yield) {
                return Ok(WorkPoll::Pending);
            }
            if self.prefix_matched > 0
                && self.pattern[self.prefix_matched] != self.pattern[self.prefix_index]
            {
                self.prefix_matched = self.prefix[self.prefix_matched - 1];
                continue;
            }
            if self.pattern[self.prefix_matched] == self.pattern[self.prefix_index] {
                self.prefix_matched += 1;
            }
            self.prefix[self.prefix_index] = self.prefix_matched;
            self.prefix_index += 1;
        }
        self.phase = TextSearchPhase::Source;
        Ok(WorkPoll::Ready(()))
    }

    pub fn source_step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<BuiltinValue>), BuiltinError> {
        if self.phase != TextSearchPhase::Source {
            return Err(BuiltinError::Abort {
                message: "text search source was polled before its pattern was prepared".into(),
            });
        }
        let mut consumed = 0usize;
        if let TextSearchKind::FindFrom { start } = self.kind {
            if self.source_offset < start {
                let skipped = (start - self.source_offset)
                    .min(input.len())
                    .min(control.work_remaining());
                control
                    .consume(skipped)
                    .expect("source skip is bounded by remaining work");
                consumed += skipped;
                self.source_offset += skipped;
                if self.source_offset < start {
                    if end_of_input && consumed == input.len() {
                        return Err(BuiltinError::Text(crate::TextFailure::InvalidIndex));
                    }
                    return Ok((consumed, WorkPoll::Pending));
                }
                if !input.is_char_boundary(consumed) {
                    return Err(BuiltinError::Text(crate::TextFailure::NotCharBoundary));
                }
            }
        }
        if self.pattern.is_empty() {
            return self.empty_pattern_step(input, consumed, end_of_input, control);
        }
        if self.kind == TextSearchKind::StartsWith {
            return self.starts_with_step(input, consumed, end_of_input, control);
        }
        loop {
            let (byte, newly_consumed) = if let Some(byte) = self.pending_byte.take() {
                (byte, false)
            } else if let Some(byte) = input.as_bytes().get(consumed).copied() {
                (byte, true)
            } else {
                break;
            };
            if newly_consumed {
                consumed += 1;
                self.source_offset += 1;
            }
            if control.consume(1) == Err(WorkStopReason::Yield) {
                self.pending_byte = Some(byte);
                break;
            }
            if self.matched > 0 && self.pattern[self.matched] != byte {
                self.matched = self.prefix[self.matched - 1];
                self.pending_byte = Some(byte);
                continue;
            }
            if self.pattern[self.matched] == byte {
                self.matched += 1;
            }
            if self.matched == self.pattern.len() {
                let start = self.source_offset - self.pattern.len();
                match self.kind {
                    TextSearchKind::Contains => {
                        return self.complete(consumed, BuiltinValue::Bool(true));
                    }
                    TextSearchKind::StartsWith => unreachable!("handled before KMP scan"),
                    TextSearchKind::Find | TextSearchKind::FindFrom { .. } => {
                        return self.complete(consumed, option_usize(Some(start)));
                    }
                    TextSearchKind::Rfind | TextSearchKind::EndsWith => {
                        self.last_match = Some(start)
                    }
                    TextSearchKind::Count => {
                        self.count = self
                            .count
                            .checked_add(1)
                            .ok_or(BuiltinError::NumericOverflow)?;
                    }
                }
                self.matched =
                    if matches!(self.kind, TextSearchKind::Rfind | TextSearchKind::EndsWith) {
                        self.prefix[self.matched - 1]
                    } else {
                        0
                    };
            }
            if consumed == input.len() && self.pending_byte.is_none() {
                break;
            }
        }
        if end_of_input && consumed == input.len() && self.pending_byte.is_none() {
            let value = match self.kind {
                TextSearchKind::Contains => BuiltinValue::Bool(false),
                TextSearchKind::StartsWith => unreachable!("handled before KMP scan"),
                TextSearchKind::EndsWith => BuiltinValue::Bool(
                    self.last_match
                        .is_some_and(|start| start + self.pattern.len() == self.source_offset),
                ),
                TextSearchKind::Find | TextSearchKind::FindFrom { .. } => option_usize(None),
                TextSearchKind::Rfind => option_usize(self.last_match),
                TextSearchKind::Count => BuiltinValue::Usize(self.count),
            };
            return self.complete(consumed, value);
        }
        Ok((consumed, WorkPoll::Pending))
    }

    fn empty_pattern_step(
        &mut self,
        input: &str,
        mut consumed: usize,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<BuiltinValue>), BuiltinError> {
        match self.kind {
            TextSearchKind::Contains | TextSearchKind::StartsWith | TextSearchKind::EndsWith => {
                self.complete(consumed, BuiltinValue::Bool(true))
            }
            TextSearchKind::Find => self.complete(consumed, option_usize(Some(0))),
            TextSearchKind::FindFrom { start } => {
                self.complete(consumed, option_usize(Some(start)))
            }
            TextSearchKind::Rfind => {
                while consumed < input.len() {
                    let ch = input[consumed..].chars().next().expect("remaining scalar");
                    if control.consume(ch.len_utf8()) == Err(WorkStopReason::Yield) {
                        return Ok((consumed, WorkPoll::Pending));
                    }
                    consumed += ch.len_utf8();
                    self.source_offset += ch.len_utf8();
                }
                if end_of_input {
                    self.complete(consumed, option_usize(Some(self.source_offset)))
                } else {
                    Ok((consumed, WorkPoll::Pending))
                }
            }
            TextSearchKind::Count => {
                while consumed < input.len() {
                    let ch = input[consumed..].chars().next().expect("remaining scalar");
                    if control.consume(ch.len_utf8()) == Err(WorkStopReason::Yield) {
                        return Ok((consumed, WorkPoll::Pending));
                    }
                    consumed += ch.len_utf8();
                    self.source_offset += ch.len_utf8();
                    self.count = self
                        .count
                        .checked_add(1)
                        .ok_or(BuiltinError::NumericOverflow)?;
                }
                if end_of_input {
                    let count = self
                        .count
                        .checked_add(1)
                        .ok_or(BuiltinError::NumericOverflow)?;
                    self.complete(consumed, BuiltinValue::Usize(count))
                } else {
                    Ok((consumed, WorkPoll::Pending))
                }
            }
        }
    }

    fn starts_with_step(
        &mut self,
        input: &str,
        mut consumed: usize,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<BuiltinValue>), BuiltinError> {
        while consumed < input.len() && self.source_offset < self.pattern.len() {
            if control.consume(1) == Err(WorkStopReason::Yield) {
                return Ok((consumed, WorkPoll::Pending));
            }
            if input.as_bytes()[consumed] != self.pattern[self.source_offset] {
                consumed += 1;
                self.source_offset += 1;
                return self.complete(consumed, BuiltinValue::Bool(false));
            }
            consumed += 1;
            self.source_offset += 1;
        }
        if self.source_offset == self.pattern.len() {
            self.complete(consumed, BuiltinValue::Bool(true))
        } else if end_of_input && consumed == input.len() {
            self.complete(consumed, BuiltinValue::Bool(false))
        } else {
            Ok((consumed, WorkPoll::Pending))
        }
    }

    fn complete(
        &mut self,
        consumed: usize,
        value: BuiltinValue,
    ) -> Result<(usize, WorkPoll<BuiltinValue>), BuiltinError> {
        self.phase = TextSearchPhase::Complete;
        Ok((consumed, WorkPoll::Ready(value)))
    }
}

fn work_limit_error(reason: WorkStopReason) -> BuiltinError {
    match reason {
        WorkStopReason::Yield => BuiltinError::Abort {
            message: "text work attempted to reserve scratch after its quantum was exhausted"
                .into(),
        },
        WorkStopReason::OutputLimit | WorkStopReason::ScratchLimit => {
            BuiltinError::ResourceExhausted
        }
    }
}

fn option_usize(value: Option<usize>) -> BuiltinValue {
    value
        .map(BuiltinValue::Usize)
        .map(Box::new)
        .map(BuiltinValue::OptionSome)
        .unwrap_or(BuiltinValue::OptionNone)
}

pub fn matches<'a>(source: &'a str, pattern: &str) -> MatchRanges<'a> {
    MatchRanges {
        source,
        plan: TextSequencePlan::new(TextSequenceKind::Matches {
            pattern: pattern.to_owned(),
        }),
        cursor: TextCursorState::default(),
    }
}

#[derive(Clone)]
pub struct MatchRanges<'a> {
    source: &'a str,
    plan: TextSequencePlan,
    cursor: TextCursorState,
}

impl Iterator for MatchRanges<'_> {
    type Item = std::ops::Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        match step(&self.plan, &mut self.cursor, self.source).expect("valid text match cursor") {
            TextSequenceStep::Item(TextSequenceElement::Match(range)) => Some(range),
            TextSequenceStep::End => None,
            _ => unreachable!("matches plan emits match ranges"),
        }
    }
}

pub fn find(source: &str, pattern: &str) -> Option<usize> {
    matches(source, pattern).next().map(|range| range.start)
}

pub fn rfind(source: &str, pattern: &str) -> Option<usize> {
    matches(source, pattern).last().map(|range| range.start)
}

pub fn find_from(
    source: &str,
    pattern: &str,
    start: usize,
) -> Result<Option<usize>, crate::TextFailure> {
    if start > source.len() {
        return Err(crate::TextFailure::InvalidIndex);
    }
    if !source.is_char_boundary(start) {
        return Err(crate::TextFailure::NotCharBoundary);
    }
    Ok(matches(&source[start..], pattern)
        .next()
        .map(|range| start + range.start))
}

pub fn count(source: &str, pattern: &str) -> usize {
    matches(source, pattern).count()
}
