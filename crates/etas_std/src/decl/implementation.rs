use crate::StdIntrinsicId;

#[derive(
    serde::Serialize,
    serde::Deserialize,
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

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
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

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub enum StdImplementationRef {
    Primitive {
        intrinsic: StdIntrinsicId,
    },
    #[serde(skip_deserializing)]
    Source {
        target: StdSourceImplementation,
        optimized_intrinsic: Option<StdIntrinsicId>,
    },
    /// Bundle-local address; its canonical definition is in the registry image.
    Artifact {
        body: u32,
    },
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct StdBodyBinding {
    pub callable: etas_code::CallableId,
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
