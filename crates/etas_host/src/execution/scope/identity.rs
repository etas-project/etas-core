/// Unique within an invocation. Never persisted as a cross-run identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeId(pub(crate) u64);

impl ScopeId {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}
