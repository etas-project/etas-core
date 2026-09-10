use super::MemoryVersion;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteCondition {
    Any,
    Missing,
    Exists,
    Match(MemoryVersion),
}

impl WriteCondition {
    pub fn is_satisfied_by(&self, current: Option<&MemoryVersion>) -> bool {
        match self {
            Self::Any => true,
            Self::Missing => current.is_none(),
            Self::Exists => current.is_some(),
            Self::Match(expected) => current == Some(expected),
        }
    }

    pub fn expected_version(&self) -> Option<&MemoryVersion> {
        match self {
            Self::Match(version) => Some(version),
            _ => None,
        }
    }
}
