pub mod effect_decl;
pub mod flow_decl;
pub mod impl_decl;
pub mod requirement;
pub mod std_type;
pub mod tool_decl;
pub mod type_decl;
pub mod value_decl;

pub use effect_decl::{EffectActionArgKind, EffectActionDecl, EffectDecl, StdRuntimeRequirement};
pub use flow_decl::{FlowDecl, FlowSourceMethod, FlowSourceMethodKind};
pub use impl_decl::ImplDecl;
pub use requirement::{RequirementDecl, RequirementKind, RequirementSemantics, StdLimitKind};
pub use std_type::{
    StdPrimitiveType, StdRecordField, StdSupportConstraint, StdTrustWrapper, StdType,
};
pub use tool_decl::ToolDecl;
pub use type_decl::{TypeDecl, TypeDeclKind, TypeParam};
pub use value_decl::ValueDecl;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StdDecl {
    Type(TypeDecl),
    Effect(EffectDecl),
    EffectAction(EffectActionDecl),
    Flow(FlowDecl),
    Tool(ToolDecl),
    Impl(ImplDecl),
    Requirement(RequirementDecl),
    Value(ValueDecl),
}
