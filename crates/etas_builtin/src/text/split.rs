use crate::{BuiltinError, BuiltinValue};

use super::iteration::{
    TextCursorState, TextSequenceElement, TextSequenceKind, TextSequencePlan, TextSequenceStep,
    step,
};

pub fn split(source: &str, separator: &str) -> Vec<BuiltinValue> {
    split_ranges(source, separator)
        .map(|range| BuiltinValue::String(source[range].to_owned()))
        .collect()
}

pub fn splitn(source: &str, separator: &str, count: usize) -> Vec<BuiltinValue> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![BuiltinValue::String(source.to_owned())];
    }
    let mut ranges = split_ranges(source, separator);
    let mut output = Vec::with_capacity(count.min(8));
    for _ in 0..count - 1 {
        let Some(range) = ranges.next() else {
            return output;
        };
        output.push(BuiltinValue::String(source[range].to_owned()));
    }
    let remainder = ranges.cursor.byte_offset.min(source.len());
    output.push(BuiltinValue::String(source[remainder..].to_owned()));
    output
}

pub fn split_once(source: &str, separator: &str) -> Option<BuiltinValue> {
    let range = super::search::matches(source, separator).next()?;
    Some(tuple(
        source[..range.start].to_owned(),
        source[range.end..].to_owned(),
    ))
}

pub fn rsplit_once(source: &str, separator: &str) -> Option<BuiltinValue> {
    let range = super::search::matches(source, separator).last()?;
    Some(tuple(
        source[..range.start].to_owned(),
        source[range.end..].to_owned(),
    ))
}

fn tuple(first: String, second: String) -> BuiltinValue {
    BuiltinValue::Variant {
        name: "Tuple".into(),
        fields: vec![BuiltinValue::String(first), BuiltinValue::String(second)],
    }
}

pub struct SplitRanges<'a> {
    source: &'a str,
    plan: TextSequencePlan,
    pub(crate) cursor: TextCursorState,
}

pub fn split_ranges<'a>(source: &'a str, separator: &str) -> SplitRanges<'a> {
    SplitRanges {
        source,
        plan: TextSequencePlan::new(TextSequenceKind::Split {
            separator: separator.to_owned(),
        }),
        cursor: TextCursorState::default(),
    }
}

impl Iterator for SplitRanges<'_> {
    type Item = std::ops::Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        match step(&self.plan, &mut self.cursor, self.source)
            .map_err(BuiltinError::Text)
            .expect("valid split cursor")
        {
            TextSequenceStep::Item(TextSequenceElement::Segment(range)) => Some(range),
            TextSequenceStep::End => None,
            _ => unreachable!("split plan emits text segments"),
        }
    }
}
