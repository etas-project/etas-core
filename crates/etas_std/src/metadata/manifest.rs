use crate::StdRegistryVersion;

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct StdManifest {
    pub version: StdRegistryVersion,
    pub module_count: usize,
    pub symbol_count: usize,
}
