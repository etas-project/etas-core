pub mod command;
pub mod path;
pub mod sandbox;
pub mod url;

use crate::StdRegistryBuilder;

pub fn register(builder: &mut StdRegistryBuilder) {
    sandbox::register(builder);
    command::register(builder);
    path::register(builder);
    url::register(builder);
}
