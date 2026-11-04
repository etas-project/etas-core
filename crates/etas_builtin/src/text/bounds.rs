use std::ops::Range;

use crate::{BuiltinRangeBounds, BuiltinValue, TextFailure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckedPosition {
    Signed(i128),
    Unsigned(u128),
}

impl CheckedPosition {
    pub fn from_value(value: &BuiltinValue) -> Option<Self> {
        Some(match value {
            BuiltinValue::I8(value) => Self::Signed(i128::from(*value)),
            BuiltinValue::I16(value) => Self::Signed(i128::from(*value)),
            BuiltinValue::I32(value) => Self::Signed(i128::from(*value)),
            BuiltinValue::I64(value) => Self::Signed(i128::from(*value)),
            BuiltinValue::I128(value) => Self::Signed(*value),
            BuiltinValue::Isize(value) => Self::Signed(*value as i128),
            BuiltinValue::U8(value) => Self::Unsigned(u128::from(*value)),
            BuiltinValue::U16(value) => Self::Unsigned(u128::from(*value)),
            BuiltinValue::U32(value) => Self::Unsigned(u128::from(*value)),
            BuiltinValue::U64(value) => Self::Unsigned(u128::from(*value)),
            BuiltinValue::U128(value) => Self::Unsigned(*value),
            BuiltinValue::Usize(value) => Self::Unsigned(*value as u128),
            _ => return None,
        })
    }

    pub fn to_usize(self) -> Result<usize, TextFailure> {
        match self {
            Self::Signed(value) if value < 0 => Err(TextFailure::InvalidIndex),
            Self::Signed(value) => usize::try_from(value).map_err(|_| TextFailure::InvalidIndex),
            Self::Unsigned(value) => usize::try_from(value).map_err(|_| TextFailure::InvalidIndex),
        }
    }
}

pub fn position(value: &BuiltinValue) -> Result<usize, TextFailure> {
    CheckedPosition::from_value(value)
        .ok_or(TextFailure::InvalidIndex)?
        .to_usize()
}

pub fn interval(value: &BuiltinValue, limit: usize) -> Result<Range<usize>, TextFailure> {
    let range = requested_interval(value)?;
    if range.end > limit {
        return Err(TextFailure::InvalidRange);
    }
    Ok(range)
}

pub fn requested_interval(value: &BuiltinValue) -> Result<Range<usize>, TextFailure> {
    let BuiltinValue::Range { start, end, bounds } = value else {
        return Err(TextFailure::InvalidRange);
    };
    let start = CheckedPosition::from_value(start).ok_or(TextFailure::InvalidRange)?;
    let end = CheckedPosition::from_value(end).ok_or(TextFailure::InvalidRange)?;
    let mut start = start.to_usize().map_err(|_| TextFailure::InvalidRange)?;
    let mut end = end.to_usize().map_err(|_| TextFailure::InvalidRange)?;
    if matches!(
        bounds,
        BuiltinRangeBounds::OpenOpen | BuiltinRangeBounds::OpenClosed
    ) {
        start = start.checked_add(1).ok_or(TextFailure::InvalidRange)?;
    }
    if matches!(
        bounds,
        BuiltinRangeBounds::ClosedClosed | BuiltinRangeBounds::OpenClosed
    ) {
        end = end.checked_add(1).ok_or(TextFailure::InvalidRange)?;
    }
    if start > end {
        return Err(TextFailure::InvalidRange);
    }
    Ok(start..end)
}

pub fn byte_interval(text: &str, value: &BuiltinValue) -> Result<Range<usize>, TextFailure> {
    let range = interval(value, text.len())?;
    if !text.is_char_boundary(range.start) || !text.is_char_boundary(range.end) {
        return Err(TextFailure::NotCharBoundary);
    }
    Ok(range)
}

pub fn scalar_interval(text: &str, value: &BuiltinValue) -> Result<Range<usize>, TextFailure> {
    let ordinals = requested_interval(value)?;
    let mut start = (ordinals.start == 0).then_some(0);
    let mut end = (ordinals.end == 0).then_some(0);
    let mut count = 0usize;
    for (byte, scalar) in text.char_indices() {
        if count == ordinals.start {
            start = Some(byte);
        }
        if count == ordinals.end {
            end = Some(byte);
        }
        count += 1;
        let next = byte + scalar.len_utf8();
        if count == ordinals.start {
            start = Some(next);
        }
        if count == ordinals.end {
            end = Some(next);
        }
        if start.is_some() && end.is_some() {
            break;
        }
    }
    if ordinals.start > count || ordinals.end > count {
        return Err(TextFailure::InvalidRange);
    }
    if ordinals.start == count {
        start = Some(text.len());
    }
    if ordinals.end == count {
        end = Some(text.len());
    }
    Ok(start.ok_or(TextFailure::InvalidRange)?..end.ok_or(TextFailure::InvalidRange)?)
}
