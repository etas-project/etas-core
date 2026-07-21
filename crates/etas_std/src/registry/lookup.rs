use crate::{StdRegistry, StdSymbol};

pub fn lookup_qualified<'a>(
    registry: &'a StdRegistry,
    path: &[impl AsRef<str>],
) -> Option<&'a StdSymbol> {
    registry.lookup_qualified(path)
}
