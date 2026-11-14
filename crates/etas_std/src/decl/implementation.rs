use crate::StdIntrinsicId;

#[derive(
    serde::Serialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    etas_core::RetainedSize,
)]
pub enum StdLibrarySourceId {
    TextAssembly,
    TextFormat,
}

#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum StdImplementationDeterminism {
    Deterministic,
    RuntimeMediated,
    NonDeterministic,
}

#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct StdSourceImplementation {
    pub source: StdLibrarySourceId,
    pub item: &'static str,
    pub determinism: StdImplementationDeterminism,
}

impl StdSourceImplementation {
    pub const fn new(
        source: StdLibrarySourceId,
        item: &'static str,
        determinism: StdImplementationDeterminism,
    ) -> Self {
        Self {
            source,
            item,
            determinism,
        }
    }
}

#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum StdImplementationRef {
    Primitive {
        intrinsic: StdIntrinsicId,
    },
    Source {
        target: StdSourceImplementation,
        optimized_intrinsic: Option<StdIntrinsicId>,
    },
}

impl StdImplementationRef {
    pub const fn source(
        source: StdLibrarySourceId,
        item: &'static str,
        determinism: StdImplementationDeterminism,
        optimized_intrinsic: Option<StdIntrinsicId>,
    ) -> Self {
        Self::Source {
            target: StdSourceImplementation::new(source, item, determinism),
            optimized_intrinsic,
        }
    }
}
