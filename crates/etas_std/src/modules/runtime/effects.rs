use crate::{StdRegistryBuilder, StdSymbolId};

pub fn register(builder: &mut StdRegistryBuilder, effects: &[(&str, StdSymbolId)]) {
    let module = builder.module(&["std", "runtime", "effects"], "Core effect declarations.");
    for (name, symbol) in effects {
        builder
            .re_export(module, name, *symbol)
            .expect("runtime effect export has one canonical declaration");
    }
}
