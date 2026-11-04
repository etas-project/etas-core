use std::ops::Range;

use crate::{
    BuiltinError, TextFailure,
    text::unicode::{GraphemeScanStep, GraphemeScanner, is_whitespace},
    work::{WorkControl, WorkPoll, WorkStopReason},
};

use super::{TextCursorState, TextSequenceKind, TextSequencePlan};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextSequenceElement {
    Scalar(char),
    IndexedScalar { byte: usize, scalar: char },
    Segment(Range<usize>),
    IndexedSegment { byte: usize, range: Range<usize> },
    Match(Range<usize>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextSequenceStep {
    Item(TextSequenceElement),
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WhitespacePhase {
    Skip,
    Token,
}

#[derive(Clone, Debug)]
enum ScannerState {
    Scalar,
    Line {
        start: usize,
        previous_was_cr: bool,
    },
    Whitespace {
        phase: WhitespacePhase,
        start: usize,
    },
    Matcher(MatcherState),
    Grapheme {
        start: usize,
        scanner: GraphemeScanner,
    },
}

#[derive(Clone, Debug)]
struct MatcherState {
    prefix: Vec<usize>,
    prefix_index: usize,
    prefix_matched: usize,
    matched: usize,
    item_start: usize,
    prepared: bool,
}

impl MatcherState {
    fn new(item_start: usize) -> Self {
        Self {
            prefix: Vec::new(),
            prefix_index: 1,
            prefix_matched: 0,
            matched: 0,
            item_start,
            prepared: false,
        }
    }

    fn prepare(
        &mut self,
        pattern: &[u8],
        control: &mut WorkControl,
    ) -> Result<WorkPoll<()>, BuiltinError> {
        if self.prepared {
            return Ok(WorkPoll::Ready(()));
        }
        if self.prefix.is_empty() && !pattern.is_empty() {
            let bytes = pattern
                .len()
                .checked_mul(std::mem::size_of::<usize>())
                .ok_or(BuiltinError::NumericOverflow)?;
            control.reserve_scratch(bytes).map_err(work_limit_error)?;
            self.prefix
                .try_reserve_exact(pattern.len())
                .map_err(|_| BuiltinError::ResourceExhausted)?;
            self.prefix.resize(pattern.len(), 0);
        }
        while self.prefix_index < pattern.len() {
            if control.consume(1) == Err(WorkStopReason::Yield) {
                return Ok(WorkPoll::Pending);
            }
            if self.prefix_matched > 0 && pattern[self.prefix_matched] != pattern[self.prefix_index]
            {
                self.prefix_matched = self.prefix[self.prefix_matched - 1];
                continue;
            }
            if pattern[self.prefix_matched] == pattern[self.prefix_index] {
                self.prefix_matched += 1;
            }
            self.prefix[self.prefix_index] = self.prefix_matched;
            self.prefix_index += 1;
        }
        self.prepared = true;
        Ok(WorkPoll::Ready(()))
    }
}

/// One operation-tagged, resumable cursor. Input slices start at `scan_offset`;
/// callers retain source ownership and recreate the borrow for every poll.
#[derive(Clone, Debug)]
pub struct TextSequenceWork {
    plan: TextSequencePlan,
    cursor: TextCursorState,
    scan_offset: usize,
    scanner: ScannerState,
}

struct MatcherProgress<'a> {
    cursor: &'a mut TextCursorState,
    scan_offset: &'a mut usize,
    scanner: &'a mut ScannerState,
}

impl TextSequenceWork {
    pub fn new(plan: TextSequencePlan, cursor: TextCursorState) -> Result<Self, TextFailure> {
        plan.validate().map_err(|_| TextFailure::InvalidIndex)?;
        if cursor.terminal {
            return Ok(Self {
                plan,
                scan_offset: cursor.byte_offset,
                scanner: ScannerState::Scalar,
                cursor,
            });
        }
        let offset = cursor.byte_offset;
        let scanner = match &plan.kind {
            TextSequenceKind::Chars | TextSequenceKind::CharIndices => ScannerState::Scalar,
            TextSequenceKind::Lines => ScannerState::Line {
                start: offset,
                previous_was_cr: false,
            },
            TextSequenceKind::Whitespace => ScannerState::Whitespace {
                phase: WhitespacePhase::Skip,
                start: offset,
            },
            TextSequenceKind::Split { .. } | TextSequenceKind::Matches { .. } => {
                ScannerState::Matcher(MatcherState::new(offset))
            }
            TextSequenceKind::Graphemes | TextSequenceKind::GraphemeIndices => {
                ScannerState::Grapheme {
                    start: offset,
                    scanner: GraphemeScanner::at(offset),
                }
            }
        };
        Ok(Self {
            plan,
            cursor,
            scan_offset: offset,
            scanner,
        })
    }

    pub fn cursor(&self) -> &TextCursorState {
        &self.cursor
    }

    pub fn scan_offset(&self) -> usize {
        self.scan_offset
    }

    pub fn reserved_scratch_bytes(&self) -> usize {
        match &self.scanner {
            ScannerState::Matcher(state) => state
                .prefix
                .capacity()
                .saturating_mul(std::mem::size_of::<usize>()),
            _ => 0,
        }
    }

    pub fn step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<TextSequenceStep>), BuiltinError> {
        if self.cursor.terminal {
            return Ok((0, WorkPoll::Ready(TextSequenceStep::End)));
        }
        match &self.plan.kind {
            TextSequenceKind::Chars | TextSequenceKind::CharIndices => {
                self.scalar(input, end_of_input, control)
            }
            TextSequenceKind::Lines => self.line(input, end_of_input, control),
            TextSequenceKind::Whitespace => self.whitespace(input, end_of_input, control),
            TextSequenceKind::Split { separator } => {
                if separator.is_empty() {
                    self.empty_pattern(input, end_of_input, false, control)
                } else {
                    Self::matcher(
                        MatcherProgress {
                            cursor: &mut self.cursor,
                            scan_offset: &mut self.scan_offset,
                            scanner: &mut self.scanner,
                        },
                        input,
                        end_of_input,
                        separator.as_bytes(),
                        false,
                        control,
                    )
                }
            }
            TextSequenceKind::Matches { pattern } => {
                if pattern.is_empty() {
                    self.empty_pattern(input, end_of_input, true, control)
                } else {
                    Self::matcher(
                        MatcherProgress {
                            cursor: &mut self.cursor,
                            scan_offset: &mut self.scan_offset,
                            scanner: &mut self.scanner,
                        },
                        input,
                        end_of_input,
                        pattern.as_bytes(),
                        true,
                        control,
                    )
                }
            }
            TextSequenceKind::Graphemes | TextSequenceKind::GraphemeIndices => {
                self.grapheme(input, end_of_input, control)
            }
        }
    }

    fn scalar(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<TextSequenceStep>), BuiltinError> {
        let Some(scalar) = input.chars().next() else {
            if end_of_input {
                self.cursor.terminal = true;
                return Ok((0, WorkPoll::Ready(TextSequenceStep::End)));
            }
            return Ok((0, WorkPoll::Pending));
        };
        let width = scalar.len_utf8();
        if control.consume(width) == Err(WorkStopReason::Yield) {
            return Ok((0, WorkPoll::Pending));
        }
        let start = self.scan_offset;
        self.scan_offset += width;
        self.cursor.byte_offset = self.scan_offset;
        let item = if matches!(self.plan.kind, TextSequenceKind::CharIndices) {
            TextSequenceElement::IndexedScalar {
                byte: start,
                scalar,
            }
        } else {
            TextSequenceElement::Scalar(scalar)
        };
        Ok((width, WorkPoll::Ready(TextSequenceStep::Item(item))))
    }

    fn line(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<TextSequenceStep>), BuiltinError> {
        let ScannerState::Line {
            start,
            previous_was_cr,
        } = &mut self.scanner
        else {
            return Err(invalid_state("line"));
        };
        let mut consumed = 0usize;
        while let Some(scalar) = input[consumed..].chars().next() {
            let width = scalar.len_utf8();
            if control.consume(width) == Err(WorkStopReason::Yield) {
                return Ok((consumed, WorkPoll::Pending));
            }
            let at = self.scan_offset;
            consumed += width;
            self.scan_offset += width;
            if scalar == '\n' {
                let end = at - usize::from(*previous_was_cr);
                let range = *start..end;
                *start = self.scan_offset;
                *previous_was_cr = false;
                self.cursor.byte_offset = self.scan_offset;
                return Ok((
                    consumed,
                    WorkPoll::Ready(TextSequenceStep::Item(TextSequenceElement::Segment(range))),
                ));
            }
            *previous_was_cr = scalar == '\r';
        }
        if !end_of_input {
            return Ok((consumed, WorkPoll::Pending));
        }
        if *start == self.scan_offset {
            self.cursor.terminal = true;
            self.cursor.byte_offset = self.scan_offset;
            return Ok((consumed, WorkPoll::Ready(TextSequenceStep::End)));
        }
        let range = *start..self.scan_offset;
        self.cursor.byte_offset = self.scan_offset;
        self.cursor.terminal = true;
        Ok((
            consumed,
            WorkPoll::Ready(TextSequenceStep::Item(TextSequenceElement::Segment(range))),
        ))
    }

    fn whitespace(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<TextSequenceStep>), BuiltinError> {
        let ScannerState::Whitespace { phase, start } = &mut self.scanner else {
            return Err(invalid_state("whitespace"));
        };
        let mut consumed = 0usize;
        while let Some(scalar) = input[consumed..].chars().next() {
            let width = scalar.len_utf8();
            if control.consume(width) == Err(WorkStopReason::Yield) {
                return Ok((consumed, WorkPoll::Pending));
            }
            let at = self.scan_offset;
            consumed += width;
            self.scan_offset += width;
            match (*phase, is_whitespace(scalar)) {
                (WhitespacePhase::Skip, true) => {}
                (WhitespacePhase::Skip, false) => {
                    *start = at;
                    *phase = WhitespacePhase::Token;
                }
                (WhitespacePhase::Token, false) => {}
                (WhitespacePhase::Token, true) => {
                    let range = *start..at;
                    *phase = WhitespacePhase::Skip;
                    *start = self.scan_offset;
                    self.cursor.byte_offset = self.scan_offset;
                    return Ok((
                        consumed,
                        WorkPoll::Ready(TextSequenceStep::Item(TextSequenceElement::Segment(
                            range,
                        ))),
                    ));
                }
            }
        }
        if !end_of_input {
            return Ok((consumed, WorkPoll::Pending));
        }
        self.cursor.byte_offset = self.scan_offset;
        self.cursor.terminal = true;
        if *phase == WhitespacePhase::Token {
            Ok((
                consumed,
                WorkPoll::Ready(TextSequenceStep::Item(TextSequenceElement::Segment(
                    *start..self.scan_offset,
                ))),
            ))
        } else {
            Ok((consumed, WorkPoll::Ready(TextSequenceStep::End)))
        }
    }

    fn empty_pattern(
        &mut self,
        input: &str,
        end_of_input: bool,
        matches: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<TextSequenceStep>), BuiltinError> {
        if !self.cursor.leading_empty_emitted {
            self.cursor.leading_empty_emitted = true;
            return Ok((
                0,
                WorkPoll::Ready(TextSequenceStep::Item(if matches {
                    TextSequenceElement::Match(0..0)
                } else {
                    TextSequenceElement::Segment(0..0)
                })),
            ));
        }
        if let Some(scalar) = input.chars().next() {
            let width = scalar.len_utf8();
            if control.consume(width) == Err(WorkStopReason::Yield) {
                return Ok((0, WorkPoll::Pending));
            }
            let start = self.scan_offset;
            self.scan_offset += width;
            self.cursor.byte_offset = self.scan_offset;
            let element = if matches {
                TextSequenceElement::Match(self.scan_offset..self.scan_offset)
            } else {
                TextSequenceElement::Segment(start..self.scan_offset)
            };
            return Ok((width, WorkPoll::Ready(TextSequenceStep::Item(element))));
        }
        if !end_of_input {
            return Ok((0, WorkPoll::Pending));
        }
        if !matches && !self.cursor.trailing_empty_emitted {
            self.cursor.trailing_empty_emitted = true;
            self.cursor.terminal = true;
            return Ok((
                0,
                WorkPoll::Ready(TextSequenceStep::Item(TextSequenceElement::Segment(
                    self.scan_offset..self.scan_offset,
                ))),
            ));
        }
        self.cursor.terminal = true;
        Ok((0, WorkPoll::Ready(TextSequenceStep::End)))
    }

    fn matcher(
        progress: MatcherProgress<'_>,
        input: &str,
        end_of_input: bool,
        pattern: &[u8],
        matches: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<TextSequenceStep>), BuiltinError> {
        let MatcherProgress {
            cursor,
            scan_offset,
            scanner,
        } = progress;
        let ScannerState::Matcher(state) = scanner else {
            return Err(invalid_state("matcher"));
        };
        if state.prepare(pattern, control)? == WorkPoll::Pending {
            return Ok((0, WorkPoll::Pending));
        }
        let mut consumed = 0usize;
        while let Some(&byte) = input.as_bytes().get(consumed) {
            if state.matched > 0 && pattern[state.matched] != byte {
                if control.consume(1) == Err(WorkStopReason::Yield) {
                    return Ok((consumed, WorkPoll::Pending));
                }
                state.matched = state.prefix[state.matched - 1];
                continue;
            }
            if control.consume(1) == Err(WorkStopReason::Yield) {
                return Ok((consumed, WorkPoll::Pending));
            }
            if pattern[state.matched] == byte {
                state.matched += 1;
            }
            consumed += 1;
            *scan_offset += 1;
            if state.matched == pattern.len() {
                let start = *scan_offset - pattern.len();
                let range = if matches {
                    start..*scan_offset
                } else {
                    state.item_start..start
                };
                state.matched = 0;
                state.item_start = *scan_offset;
                cursor.byte_offset = *scan_offset;
                return Ok((
                    consumed,
                    WorkPoll::Ready(TextSequenceStep::Item(if matches {
                        TextSequenceElement::Match(range)
                    } else {
                        TextSequenceElement::Segment(range)
                    })),
                ));
            }
        }
        if !end_of_input {
            return Ok((consumed, WorkPoll::Pending));
        }
        cursor.byte_offset = *scan_offset;
        cursor.terminal = true;
        if matches {
            Ok((consumed, WorkPoll::Ready(TextSequenceStep::End)))
        } else {
            Ok((
                consumed,
                WorkPoll::Ready(TextSequenceStep::Item(TextSequenceElement::Segment(
                    state.item_start..*scan_offset,
                ))),
            ))
        }
    }

    fn grapheme(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> Result<(usize, WorkPoll<TextSequenceStep>), BuiltinError> {
        let ScannerState::Grapheme { start, scanner } = &mut self.scanner else {
            return Err(invalid_state("grapheme"));
        };
        let (consumed, poll) = scanner.step(input, end_of_input, control);
        self.scan_offset += consumed;
        match poll {
            WorkPoll::Pending => Ok((consumed, WorkPoll::Pending)),
            WorkPoll::Ready(GraphemeScanStep::Boundary(end)) if end > *start => {
                let range = *start..end;
                let byte = *start;
                *start = end;
                self.cursor.byte_offset = end;
                Ok((
                    consumed,
                    WorkPoll::Ready(TextSequenceStep::Item(
                        if matches!(self.plan.kind, TextSequenceKind::GraphemeIndices) {
                            TextSequenceElement::IndexedSegment { byte, range }
                        } else {
                            TextSequenceElement::Segment(range)
                        },
                    )),
                ))
            }
            WorkPoll::Ready(GraphemeScanStep::Boundary(_)) => {
                if end_of_input {
                    self.cursor.terminal = true;
                    Ok((consumed, WorkPoll::Ready(TextSequenceStep::End)))
                } else {
                    Ok((consumed, WorkPoll::Pending))
                }
            }
            WorkPoll::Ready(GraphemeScanStep::End) => {
                self.cursor.terminal = true;
                Ok((consumed, WorkPoll::Ready(TextSequenceStep::End)))
            }
        }
    }
}

pub fn step(
    plan: &TextSequencePlan,
    cursor: &mut TextCursorState,
    source: &str,
) -> Result<TextSequenceStep, TextFailure> {
    if cursor.byte_offset > source.len() || !source.is_char_boundary(cursor.byte_offset) {
        return Err(TextFailure::InvalidIndex);
    }
    let mut work = TextSequenceWork::new(plan.clone(), cursor.clone())?;
    let mut offset = work.scan_offset();
    loop {
        let mut control = WorkControl::new(usize::MAX, usize::MAX, usize::MAX);
        let (consumed, poll) =
            work.step(&source[offset..], true, &mut control)
                .map_err(|error| match error {
                    BuiltinError::Text(failure) => failure,
                    _ => TextFailure::LengthOverflow,
                })?;
        offset += consumed;
        if let WorkPoll::Ready(result) = poll {
            *cursor = work.cursor().clone();
            return Ok(result);
        }
    }
}

fn invalid_state(operation: &str) -> BuiltinError {
    BuiltinError::Abort {
        message: format!("text sequence work state does not match {operation} plan"),
    }
}

fn work_limit_error(reason: WorkStopReason) -> BuiltinError {
    match reason {
        WorkStopReason::Yield => BuiltinError::Abort {
            message: "text sequence scratch reservation exceeded its work quantum".into(),
        },
        WorkStopReason::OutputLimit | WorkStopReason::ScratchLimit => {
            BuiltinError::ResourceExhausted
        }
    }
}
