pub mod tcp;

use crate::StdRegistryBuilder;

pub fn register(builder: &mut StdRegistryBuilder) {
    tcp::register(builder);
}
