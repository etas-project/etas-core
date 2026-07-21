use std::{fmt, path::PathBuf};

#[derive(Debug)]
pub enum MetadataArtifactError {
    Io { path: PathBuf, message: String },
    Invalid { path: PathBuf, message: String },
    Compression { path: PathBuf, message: String },
    SizeOverflow,
    HeaderStringTooLong,
}

impl MetadataArtifactError {
    pub fn invalid(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Invalid {
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn compression(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Compression {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for MetadataArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message }
            | Self::Invalid { path, message }
            | Self::Compression { path, message } => {
                write!(formatter, "{}: {message}", path.display())
            }
            Self::SizeOverflow => formatter.write_str("metadata artifact size overflow"),
            Self::HeaderStringTooLong => formatter.write_str("metadata header string is too long"),
        }
    }
}

impl std::error::Error for MetadataArtifactError {}
