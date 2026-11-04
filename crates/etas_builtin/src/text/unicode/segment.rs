use icu_properties::{
    CodePointMapData, CodePointSetData,
    props::{ExtendedPictographic, GraphemeClusterBreak, IndicConjunctBreak},
};

use crate::{
    TextFailure,
    work::{WorkControl, WorkPoll, WorkStopReason},
};

use super::super::iteration::{
    TextCursorState, TextSequenceElement, TextSequenceKind, TextSequencePlan, TextSequenceStep,
};

pub fn step_grapheme(
    plan: &TextSequencePlan,
    cursor: &mut TextCursorState,
    source: &str,
) -> Result<TextSequenceStep, TextFailure> {
    if cursor.byte_offset == source.len() {
        cursor.terminal = true;
        return Ok(TextSequenceStep::End);
    }
    let start = cursor.byte_offset;
    let end = next_grapheme_boundary(source, start);
    cursor.byte_offset = end;
    let range = start..end;
    Ok(TextSequenceStep::Item(match plan.kind {
        TextSequenceKind::Graphemes => TextSequenceElement::Segment(range),
        TextSequenceKind::GraphemeIndices => {
            TextSequenceElement::IndexedSegment { byte: start, range }
        }
        _ => unreachable!("grapheme step requires a grapheme plan"),
    }))
}

pub fn next_grapheme_boundary(source: &str, start: usize) -> usize {
    let gcb = CodePointMapData::<GraphemeClusterBreak>::new();
    let incb = CodePointMapData::<IndicConjunctBreak>::new();
    let pictographic = CodePointSetData::new::<ExtendedPictographic>();
    let mut scalars = source[start..].char_indices();
    let Some((_, first)) = scalars.next() else {
        return source.len();
    };
    let mut previous = gcb.get(first);
    let mut regional_count = usize::from(previous == GraphemeClusterBreak::RegionalIndicator);
    let mut pictographic_extend = pictographic.contains(first);
    let mut zwj_after_pictographic = false;
    let mut conjunct = incb.get(first) == IndicConjunctBreak::Consonant;
    let mut linker_seen = false;

    for (relative, scalar) in scalars {
        let current = gcb.get(scalar);
        let current_incb = incb.get(scalar);
        let current_pictographic = pictographic.contains(scalar);
        if grapheme_break(
            previous,
            current,
            current_pictographic,
            zwj_after_pictographic,
            conjunct && linker_seen && current_incb == IndicConjunctBreak::Consonant,
            regional_count,
        ) {
            return start + relative;
        }

        if current == GraphemeClusterBreak::RegionalIndicator {
            regional_count += 1;
        } else {
            regional_count = 0;
        }

        zwj_after_pictographic = current == GraphemeClusterBreak::ZWJ && pictographic_extend;
        pictographic_extend = if current_pictographic {
            true
        } else if current == GraphemeClusterBreak::Extend {
            pictographic_extend
        } else {
            false
        };

        match current_incb {
            IndicConjunctBreak::Consonant => {
                conjunct = true;
                linker_seen = false;
            }
            IndicConjunctBreak::Extend if conjunct => {}
            IndicConjunctBreak::Linker if conjunct => linker_seen = true,
            _ => {
                conjunct = false;
                linker_seen = false;
            }
        }
        previous = current;
    }
    source.len()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphemeScanStep {
    Boundary(usize),
    End,
}

#[derive(Clone, Debug, Default)]
pub struct GraphemeScanner {
    position: usize,
    previous: Option<GraphemeClusterBreak>,
    regional_count: usize,
    pictographic_extend: bool,
    zwj_after_pictographic: bool,
    conjunct: bool,
    linker_seen: bool,
    final_boundary_emitted: bool,
}

impl GraphemeScanner {
    pub fn at(position: usize) -> Self {
        Self {
            position,
            ..Self::default()
        }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn step(
        &mut self,
        input: &str,
        end_of_input: bool,
        control: &mut WorkControl,
    ) -> (usize, WorkPoll<GraphemeScanStep>) {
        let gcb = CodePointMapData::<GraphemeClusterBreak>::new();
        let incb = CodePointMapData::<IndicConjunctBreak>::new();
        let pictographic = CodePointSetData::new::<ExtendedPictographic>();
        let mut consumed = 0usize;
        while let Some(scalar) = input[consumed..].chars().next() {
            let width = scalar.len_utf8();
            if control.consume(width) == Err(WorkStopReason::Yield) {
                return (consumed, WorkPoll::Pending);
            }
            let current = gcb.get(scalar);
            let current_incb = incb.get(scalar);
            let current_pictographic = pictographic.contains(scalar);
            let boundary = self.previous.is_some_and(|previous| {
                grapheme_break(
                    previous,
                    current,
                    current_pictographic,
                    self.zwj_after_pictographic,
                    self.conjunct
                        && self.linker_seen
                        && current_incb == IndicConjunctBreak::Consonant,
                    self.regional_count,
                )
            });
            let at = self.position;
            self.advance_context(current, current_incb, current_pictographic);
            self.position += width;
            consumed += width;
            if boundary {
                return (consumed, WorkPoll::Ready(GraphemeScanStep::Boundary(at)));
            }
        }
        if !end_of_input {
            return (consumed, WorkPoll::Pending);
        }
        if !self.final_boundary_emitted {
            self.final_boundary_emitted = true;
            return (
                consumed,
                WorkPoll::Ready(GraphemeScanStep::Boundary(self.position)),
            );
        }
        (consumed, WorkPoll::Ready(GraphemeScanStep::End))
    }

    fn advance_context(
        &mut self,
        current: GraphemeClusterBreak,
        current_incb: IndicConjunctBreak,
        current_pictographic: bool,
    ) {
        if current == GraphemeClusterBreak::RegionalIndicator {
            self.regional_count += 1;
        } else {
            self.regional_count = 0;
        }
        self.zwj_after_pictographic =
            current == GraphemeClusterBreak::ZWJ && self.pictographic_extend;
        self.pictographic_extend = if current_pictographic {
            true
        } else if current == GraphemeClusterBreak::Extend {
            self.pictographic_extend
        } else {
            false
        };
        match current_incb {
            IndicConjunctBreak::Consonant => {
                self.conjunct = true;
                self.linker_seen = false;
            }
            IndicConjunctBreak::Extend if self.conjunct => {}
            IndicConjunctBreak::Linker if self.conjunct => self.linker_seen = true,
            _ => {
                self.conjunct = false;
                self.linker_seen = false;
            }
        }
        self.previous = Some(current);
    }
}

fn grapheme_break(
    previous: GraphemeClusterBreak,
    current: GraphemeClusterBreak,
    current_pictographic: bool,
    zwj_after_pictographic: bool,
    indic_conjunct: bool,
    regional_count: usize,
) -> bool {
    use GraphemeClusterBreak as G;
    if previous == G::CR && current == G::LF {
        return false;
    }
    if matches!(previous, G::Control | G::CR | G::LF)
        || matches!(current, G::Control | G::CR | G::LF)
    {
        return true;
    }
    if previous == G::L && matches!(current, G::L | G::V | G::LV | G::LVT) {
        return false;
    }
    if matches!(previous, G::LV | G::V) && matches!(current, G::V | G::T) {
        return false;
    }
    if matches!(previous, G::LVT | G::T) && current == G::T {
        return false;
    }
    if matches!(current, G::Extend | G::ZWJ | G::SpacingMark) || previous == G::Prepend {
        return false;
    }
    if indic_conjunct || (zwj_after_pictographic && current_pictographic) {
        return false;
    }
    !(previous == G::RegionalIndicator
        && current == G::RegionalIndicator
        && regional_count % 2 == 1)
}
