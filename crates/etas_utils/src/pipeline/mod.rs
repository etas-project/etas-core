pub mod adapter;
pub mod analysis;
pub mod artifact;
pub mod config;
pub mod definition;
pub mod instrumentation;
pub mod manager;
pub mod pass;
pub mod preservation;
pub mod result;
pub mod timing;
pub mod unit;

pub use adapter::ForEachAdapter;
pub use analysis::{Analysis, AnalysisCache};
pub use artifact::{ArtifactKey, ArtifactRef, ArtifactScope, ArtifactSet};
pub use config::PipelineConfig;
pub use definition::{Pipeline, PipelineStep};
pub use instrumentation::PassInstrumentation;
pub use manager::PassManager;
pub use pass::{Pass, PassDescriptor, PassKind};
pub use preservation::PreservedArtifacts;
pub use result::{PassControl, PassFailure, PassResult, PipelineRunResult};
pub use timing::{PassRunRecord, PassTiming, PipelineStats};
pub use unit::{
    PassContext, PassScope, UnitFilterKey, UnitKey, UnitKindKey, UnitOrder, UnitProvider,
    UnitSelector,
};
