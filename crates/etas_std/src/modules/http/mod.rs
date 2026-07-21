pub mod codec;

use crate::StdRegistryBuilder;

pub fn register(builder: &mut StdRegistryBuilder) {
    codec::register(builder);
}
