use std::{path::PathBuf, sync::Arc};

use crate::LineIndex;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct SourceId(pub u32);

#[derive(Clone, Debug)]
pub struct SourceFile {
    pub id: SourceId,
    pub path: Option<PathBuf>,
    pub text: Arc<str>,
    pub line_index: LineIndex,
}

impl SourceFile {
    pub fn new(id: SourceId, path: Option<PathBuf>, text: impl Into<Arc<str>>) -> Self {
        let text = text.into();
        let line_index = LineIndex::new(&text);
        Self {
            id,
            path,
            text,
            line_index,
        }
    }

    pub fn anonymous(text: impl Into<Arc<str>>) -> Self {
        Self::new(SourceId(0), None, text)
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}
