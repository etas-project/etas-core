pub mod text;

use crate::StdRegistryBuilder;

pub fn register(builder: &mut StdRegistryBuilder) {
    text::register(builder);
}
