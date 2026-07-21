pub mod protocol;

use crate::StdRegistryBuilder;

pub fn register(builder: &mut StdRegistryBuilder) {
    protocol::register(builder);
}
