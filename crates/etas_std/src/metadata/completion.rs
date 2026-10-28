#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct CompletionMetadata {
    pub label: String,
    pub detail: String,
}

impl CompletionMetadata {
    pub fn new(label: &str, detail: &str) -> Self {
        Self {
            label: label.to_owned(),
            detail: detail.to_owned(),
        }
    }
}
