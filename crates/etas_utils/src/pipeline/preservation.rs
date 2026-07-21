use super::{ArtifactKey, ArtifactSet};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PreservedArtifacts {
    #[default]
    All,
    None,
    Some(ArtifactSet),
}

impl PreservedArtifacts {
    pub fn preserves(&self, key: ArtifactKey) -> bool {
        match self {
            Self::All => true,
            Self::None => false,
            Self::Some(keys) => keys.contains(key),
        }
    }

    pub fn preserved_set(&self) -> Option<&ArtifactSet> {
        match self {
            Self::Some(keys) => Some(keys),
            Self::All | Self::None => None,
        }
    }
}
