#[derive(Clone, Debug, Default, PartialEq, Eq, etas_core::RetainedSize)]
pub struct DocsMetadata {
    pub summary: String,
}

impl DocsMetadata {
    pub fn summary(summary: &str) -> Self {
        Self {
            summary: summary.to_owned(),
        }
    }
}
