pub(crate) mod canonical;
mod definition;
mod fingerprint;
mod header;
mod method;
mod origins;
mod revision;

pub use definition::{DefinitionKey, DefinitionKind, IdentityError, PackageKey};
pub use fingerprint::CodeFingerprint;
pub use header::{ImplementationHeader, ImplementationSpecIdentity};
pub use method::{CallableId, CallableIdentityRef, ImplementationKey, MethodId};
pub use origins::{
    DefinitionBindingOrigin, DefinitionModuleOrigin, DefinitionOriginsError,
    PackageDefinitionOrigins,
};
pub use revision::{RevisionInfo, RevisionInputs, RevisionKey};

pub const IDENTITY_SCHEMA_VERSION: u32 = 1;
