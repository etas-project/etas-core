#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextCursorState {
    pub byte_offset: usize,
    pub terminal: bool,
    pub leading_empty_emitted: bool,
    pub trailing_empty_emitted: bool,
}
