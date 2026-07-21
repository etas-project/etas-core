pub mod declassify;
pub mod trust;

use crate::StdRegistryBuilder;

pub fn register(builder: &mut StdRegistryBuilder) {
    trust::register(builder);
    declassify::register(builder);
}
