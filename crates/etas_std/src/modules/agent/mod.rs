pub mod group;
pub mod message;
pub mod prompt;
pub mod schema;
pub mod session;

use crate::StdRegistryBuilder;

pub fn register(builder: &mut StdRegistryBuilder) {
    prompt::register(builder);
    schema::register(builder);
    message::register(builder);
    session::register(builder);
    group::register(builder);
}
