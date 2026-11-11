pub mod agent;
pub mod browser;
pub mod codec;
pub mod compiler;
pub mod core;
pub mod crypto;
pub mod effects;
pub mod fs;
pub mod host;
pub mod http;
pub mod io;
pub mod memory;
pub mod net;
pub mod runtime;
pub mod secret;
pub mod security;
pub mod stream;
pub mod task;
pub mod tls;
pub mod tool;

mod registration;

use crate::{StdRegistry, StdRegistryBuilder, StdRegistryVersion};

pub fn standard_registry() -> StdRegistry {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    core::register(&mut builder);
    let (agent_module, tool_ref) = agent::register(&mut builder);
    compiler::register(&mut builder, tool_ref);
    effects::register(&mut builder);
    agent::register_step(&mut builder, agent_module);
    task::register(&mut builder);
    io::register(&mut builder);
    memory::register(&mut builder);
    net::register(&mut builder);
    stream::register(&mut builder);
    tls::register(&mut builder);
    fs::register(&mut builder);
    http::register(&mut builder);
    codec::register(&mut builder);
    secret::register(&mut builder);
    crypto::register(&mut builder);
    browser::register(&mut builder);
    tool::register(&mut builder);
    runtime::register(&mut builder);
    security::register(&mut builder);
    host::register(&mut builder);
    builder.finish()
}
