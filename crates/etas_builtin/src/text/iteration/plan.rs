pub const TEXT_SEQUENCE_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextSequencePlan {
    pub version: u32,
    pub kind: TextSequenceKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextSequenceKind {
    Chars,
    CharIndices,
    Lines,
    Split { separator: String },
    Whitespace,
    Matches { pattern: String },
    Graphemes,
    GraphemeIndices,
}

impl TextSequencePlan {
    pub fn new(kind: TextSequenceKind) -> Self {
        Self {
            version: TEXT_SEQUENCE_FORMAT_VERSION,
            kind,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != TEXT_SEQUENCE_FORMAT_VERSION {
            return Err("unsupported text sequence plan version");
        }
        Ok(())
    }
}
