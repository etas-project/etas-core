use std::collections::BTreeMap;

use super::StdSymbolId;

#[derive(Clone, Debug, Default, PartialEq, Eq, etas_core::RetainedSize)]
pub struct StdPrelude {
    symbols: BTreeMap<String, Vec<StdSymbolRef>>,
}

impl StdPrelude {
    pub fn insert(&mut self, name: impl Into<String>, id: StdSymbolId) {
        let name = name.into();
        let candidates = self.symbols.entry(name.clone()).or_default();
        if !candidates.iter().any(|binding| binding.id == id) {
            candidates.push(StdSymbolRef { name, id });
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&StdSymbolRef> {
        let [symbol] = self.symbols.get(name)?.as_slice() else {
            return None;
        };
        Some(symbol)
    }

    pub fn candidates(&self, name: &str) -> impl Iterator<Item = &StdSymbolRef> {
        self.symbols.get(name).into_iter().flatten()
    }

    pub fn symbols(&self) -> impl Iterator<Item = &StdSymbolRef> {
        self.symbols.values().flatten()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct StdSymbolRef {
    pub name: String,
    pub id: StdSymbolId,
}
