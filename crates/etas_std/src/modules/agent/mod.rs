mod conversation;
pub mod group;
pub mod message;
pub mod prompt;
mod protocol;
pub mod schema;
pub mod session;
mod step;

use crate::StdRegistryBuilder;

pub fn register(builder: &mut StdRegistryBuilder) -> (crate::StdModuleId, crate::StdSymbolId) {
    let (module, tool_ref) = protocol::register(builder);
    prompt::register(builder);
    schema::register(builder);
    message::register(builder);
    session::register(builder);
    group::register(builder);
    conversation::register(builder);
    (module, tool_ref)
}

pub(super) fn register_step(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    step::register(builder, module);
}
