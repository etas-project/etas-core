#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
