mod collection;
mod completion;
pub use collection::{StdCollectionIntrinsic, StdCollectionKind, StdCollectionOperation};
pub mod id;
pub use completion::{IntrinsicCompletionSemantics, intrinsic_completion_semantics};
pub mod lowering;
pub mod pure;
pub mod runtime;
pub mod signature;
pub mod static_string;

pub use id::StdIntrinsicId;
pub use lowering::{LoweringHint, StdActionReducer};
pub use signature::{
    IntrinsicDescriptor, IntrinsicDispatch, IntrinsicLatentEffect, IntrinsicMemoryAccess,
    IntrinsicPurity, IntrinsicRuntimeRequirement,
};
pub use static_string::{
    IntrinsicStaticStringSemantics, StaticStringTransform, intrinsic_static_string_semantics,
};
