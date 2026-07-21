#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct TextSize(pub u32);

impl TextSize {
    pub const ZERO: Self = Self(0);

    pub fn new(value: usize) -> Self {
        Self(value.min(u32::MAX as usize) as u32)
    }

    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct TextRange {
    pub start: TextSize,
    pub end: TextSize,
}

impl TextRange {
    pub fn new(start: TextSize, end: TextSize) -> Self {
        Self { start, end }
    }

    pub fn empty(offset: TextSize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    pub fn cover(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Span {
    pub source: crate::SourceId,
    pub range: TextRange,
}

impl Span {
    pub fn new(source: crate::SourceId, range: TextRange) -> Self {
        Self { source, range }
    }

    pub fn empty(source: crate::SourceId, offset: TextSize) -> Self {
        Self {
            source,
            range: TextRange::empty(offset),
        }
    }

    pub fn cover(self, other: Self) -> Self {
        Self {
            source: self.source,
            range: self.range.cover(other.range),
        }
    }
}
