use crate::StdRegistryVersion;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdManifest {
    pub version: StdRegistryVersion,
    pub module_count: usize,
    pub symbol_count: usize,
}
