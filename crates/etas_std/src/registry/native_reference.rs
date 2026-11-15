use serde::{Deserialize, Serialize};

/// A primitive's reference to an ordinary, source-owned error enum. The host
/// requires these unit variants; it does not define the enum's other variants,
/// exports, declaration identity or language semantics.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, etas_core::RetainedSize)]
pub struct StdNativeEnumReference {
    pub path: Vec<String>,
    pub required_unit_variants: Vec<String>,
}
