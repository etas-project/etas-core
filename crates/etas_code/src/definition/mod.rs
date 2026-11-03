pub mod analysis;
mod diff;
pub use diff::{CHANGE_REPORT_SCHEMA, ChangeReport, DefinitionDiffSnapshot};
mod reference;
mod snapshot;
mod source;
pub use reference::{
    CodeDefinitionOperations, DefinitionLease, DefinitionOperationError, DefinitionRef,
    DefinitionResource,
};
pub use source::{
    DEFINITION_SOURCE_BUNDLE_SCHEMA, DefinitionSourceBundle, SourceBundleError, SourceBundleLimits,
};

pub use analysis::{AnalysisFingerprint, AnalysisUnavailableReason, DefinitionAnalysisState};
pub use snapshot::{
    BaseRevisionInput, DefinitionError, DefinitionSnapshot, FrozenDefinition, FrozenDefinitionInput,
};
