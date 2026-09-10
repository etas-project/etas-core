use crate::{HostError, HostErrorCode};

#[derive(Clone, PartialEq, Eq)]
pub struct SessionGeneration(String);
impl std::fmt::Debug for SessionGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionGeneration(<redacted>)")
    }
}
impl SessionGeneration {
    pub fn parse(token: &str) -> Result<Self, HostError> {
        let invalid = || {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "invalid session generation token",
            )
        };
        if token.len() != 4 + 64 + 1 + 32 || !token.starts_with("sg1:") {
            return Err(invalid());
        }
        let Some((scope, generation)) = token[4..].split_once(':') else {
            return Err(invalid());
        };
        if scope.len() != 64
            || generation.len() != 32
            || !scope
                .bytes()
                .chain(generation.bytes())
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid());
        }
        Ok(Self(token.to_owned()))
    }
    pub fn as_token(&self) -> &str {
        &self.0
    }
    pub fn belongs_to(&self, session: &str) -> bool {
        self.0[4..68] == crate::storage::version::scope_identity("session", session, &[])
    }
    pub(crate) fn issue(session: &str, generation: &str) -> Result<Self, HostError> {
        let scope = crate::storage::version::scope_identity("session", session, &[]);
        Self::parse(&format!("sg1:{scope}:{generation}"))
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SessionVersion(String);
impl std::fmt::Debug for SessionVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionVersion(<redacted>)")
    }
}
impl SessionVersion {
    pub fn parse(token: &str) -> Result<Self, HostError> {
        let invalid = || {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "invalid session version token",
            )
        };
        if token.len() != 4 + 64 + 1 + 32 + 1 + 16 || !token.starts_with("sv1:") {
            return Err(invalid());
        }
        let parts: Vec<_> = token[4..].split(':').collect();
        if parts.len() != 3
            || parts[0].len() != 64
            || parts[1].len() != 32
            || parts[2].len() != 16
            || parts.iter().any(|part| {
                !part
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        {
            return Err(invalid());
        }
        if u64::from_str_radix(parts[2], 16).map_err(|_| invalid())? > i64::MAX as u64 {
            return Err(invalid());
        }
        Ok(Self(token.to_owned()))
    }
    pub fn as_token(&self) -> &str {
        &self.0
    }
    pub fn belongs_to(&self, session: &str) -> bool {
        self.0[4..68] == crate::storage::version::scope_identity("session", session, &[])
    }
    pub(crate) fn issue(session: &str, generation: &str, ordinal: i64) -> Result<Self, HostError> {
        let scope = crate::storage::version::scope_identity("session", session, &[]);
        Self::parse(&format!("sv1:{scope}:{generation}:{ordinal:016x}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_identity_is_session_bound_and_canonical() {
        let generation = SessionGeneration::issue("one", &"a".repeat(32)).unwrap();
        assert!(generation.belongs_to("one"));
        assert!(!generation.belongs_to("two"));
        assert_eq!(
            SessionGeneration::parse(generation.as_token()).unwrap(),
            generation
        );
        assert!(SessionGeneration::parse(&generation.as_token().replace('a', "A")).is_err());
        assert!(SessionGeneration::parse("sg1:not-a-generation").is_err());
        let append = SessionVersion::issue("one", &"a".repeat(32), 0).unwrap();
        assert!(SessionGeneration::parse(append.as_token()).is_err());
        assert!(SessionVersion::parse(generation.as_token()).is_err());
    }
}
