use super::{CodeFingerprint, PackageKey, canonical::CanonicalEncoder};

/// The caller supplies a pre-analysis origin, never the artifact being built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StableSourceOrigin {
    Project(CodeFingerprint),
    Package(PackageKey),
    Generated(CodeFingerprint),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceIdentityError {
    EmptyPath,
    InvalidPathSegment,
    InvalidSourceRange,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub struct StableSourceKey(CodeFingerprint);

impl StableSourceKey {
    /// A generated body is anchored to the original snapshot and declaration site.
    pub fn generated_body(
        parent: StableSourceKey,
        parent_bytes: &[u8],
        declaration_start: u64,
        declaration_end: u64,
    ) -> Result<Self, SourceIdentityError> {
        if declaration_start > declaration_end || declaration_end > parent_bytes.len() as u64 {
            return Err(SourceIdentityError::InvalidSourceRange);
        }
        let mut encoder = CanonicalEncoder::new("etas.code.generated-body-origin.v1");
        encoder.fingerprint(SourceRevisionKey::new(parent, parent_bytes).fingerprint());
        encoder.number(declaration_start);
        encoder.number(declaration_end);
        Self::new(
            StableSourceOrigin::Generated(encoder.finish()),
            &["body".into()],
        )
    }
    /// `logical_path` is origin-relative, not an absolute checkout path.
    pub fn new(
        origin: StableSourceOrigin,
        logical_path: &[String],
    ) -> Result<Self, SourceIdentityError> {
        if logical_path.is_empty() {
            return Err(SourceIdentityError::EmptyPath);
        }
        if logical_path.iter().any(|part| {
            part.is_empty() || part == "." || part == ".." || part.contains(['/', '\\', '\0'])
        }) {
            return Err(SourceIdentityError::InvalidPathSegment);
        }
        let mut encoder = CanonicalEncoder::new("etas.code.stable-source.v1");
        match origin {
            StableSourceOrigin::Project(key) => {
                encoder.tag(1);
                encoder.fingerprint(key);
            }
            StableSourceOrigin::Package(key) => {
                encoder.tag(2);
                encoder.fingerprint(key.fingerprint());
            }
            StableSourceOrigin::Generated(key) => {
                encoder.tag(3);
                encoder.fingerprint(key);
            }
        }
        encoder.path(logical_path);
        Ok(Self(encoder.finish()))
    }

    pub const fn fingerprint(self) -> CodeFingerprint {
        self.0
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub struct SourceRevisionKey(CodeFingerprint);

impl SourceRevisionKey {
    /// Hashes exactly the supplied source bytes without newline or Unicode normalization.
    pub fn new(source: StableSourceKey, bytes: &[u8]) -> Self {
        let content = CodeFingerprint::of_bytes("etas.code.source-bytes.v1", bytes);
        let mut encoder = CanonicalEncoder::new("etas.code.source-revision.v1");
        encoder.fingerprint(source.fingerprint());
        encoder.fingerprint(content);
        Self(encoder.finish())
    }

    pub const fn fingerprint(self) -> CodeFingerprint {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_revision_is_origin_and_exact_bytes_sensitive() {
        let origin = StableSourceOrigin::Project(CodeFingerprint::from_bytes([1; 32]));
        let first = StableSourceKey::new(origin, &["src".into(), "first.es".into()]).unwrap();
        let second = StableSourceKey::new(origin, &["src".into(), "second.es".into()]).unwrap();
        assert_ne!(first, second);
        assert_eq!(
            SourceRevisionKey::new(first, b"a\n"),
            SourceRevisionKey::new(first, b"a\n")
        );
        assert_ne!(
            SourceRevisionKey::new(first, b"a\n"),
            SourceRevisionKey::new(first, b"a\r\n")
        );
        assert_ne!(
            SourceRevisionKey::new(first, b"a\n"),
            SourceRevisionKey::new(second, b"a\n")
        );
        assert_ne!(
            SourceRevisionKey::new(first, "\u{e9}".as_bytes()),
            SourceRevisionKey::new(first, "e\u{301}".as_bytes())
        );
        assert_eq!(
            StableSourceKey::new(origin, &["..".into()]),
            Err(SourceIdentityError::InvalidPathSegment)
        );
        assert_eq!(
            StableSourceKey::new(origin, &["a/b".into()]),
            Err(SourceIdentityError::InvalidPathSegment)
        );
        assert_eq!(
            StableSourceKey::new(origin, &["a\0b".into()]),
            Err(SourceIdentityError::InvalidPathSegment)
        );
    }

    #[test]
    fn generated_body_origin_uses_source_snapshot_and_declaration_not_checkout() {
        let origin = StableSourceOrigin::Project(CodeFingerprint::from_bytes([9; 32]));
        let source = StableSourceKey::new(origin, &["src".into(), "main.es".into()]).unwrap();
        let first = StableSourceKey::generated_body(source, b"flow a; flow b", 0, 6).unwrap();
        assert_eq!(
            first,
            StableSourceKey::generated_body(source, b"flow a; flow b", 0, 6).unwrap()
        );
        assert_ne!(
            first,
            StableSourceKey::generated_body(source, b"flow a; flow b", 8, 14).unwrap()
        );
        assert_ne!(
            first,
            StableSourceKey::generated_body(source, b"flow a; flow c", 0, 6).unwrap()
        );
        assert_eq!(
            StableSourceKey::generated_body(source, b"a", 0, 2),
            Err(SourceIdentityError::InvalidSourceRange)
        );
        assert_ne!(
            SourceRevisionKey::new(first, b"return 1;"),
            SourceRevisionKey::new(first, b"return 2;")
        );
    }
}
