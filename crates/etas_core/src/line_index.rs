use crate::{TextRange, TextSize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineIndex {
    line_starts: Vec<TextSize>,
    text_len: TextSize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![TextSize::ZERO];
        for (idx, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(TextSize::new(idx + 1));
            }
        }

        Self {
            line_starts,
            text_len: TextSize::new(text.len()),
        }
    }

    pub fn line_col(&self, offset: TextSize) -> LineCol {
        let offset = offset.min(self.text_len);
        let line = match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next) => next.saturating_sub(1),
        };
        let line_start = self.line_starts[line];
        LineCol {
            line: line as u32,
            col: offset.0.saturating_sub(line_start.0),
        }
    }

    pub fn line_range(&self, line: u32) -> Option<TextRange> {
        let line = line as usize;
        let start = *self.line_starts.get(line)?;
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.text_len);
        Some(TextRange::new(start, end))
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}
