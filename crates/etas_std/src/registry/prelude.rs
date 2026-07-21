use std::collections::BTreeMap;

use super::StdSymbolId;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StdPrelude {
    symbols: BTreeMap<String, StdSymbolRef>,
}

impl StdPrelude {
    pub fn insert(&mut self, name: impl Into<String>, id: StdSymbolId) {
        let name = name.into();
        self.symbols.insert(name.clone(), StdSymbolRef { name, id });
    }

    pub fn lookup(&self, name: &str) -> Option<&StdSymbolRef> {
        self.symbols.get(name)
    }

    pub fn symbols(&self) -> impl Iterator<Item = &StdSymbolRef> {
        self.symbols.values()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdSymbolRef {
    pub name: String,
    pub id: StdSymbolId,
}
