use etas_code::descriptor::ParameterDefaultInfo;

/// Calling information generated from a checked declaration signature. Type
/// nodes stay in the bundle descriptor graph; this view owns no graph indices.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct StdParameterInterface {
    pub name: Option<String>,
    pub default: Option<ParameterDefaultInfo>,
}
