pub mod agent;
pub mod browser;
pub mod codec;
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
pub mod tls;

use crate::{StdRegistry, StdRegistryBuilder, StdRegistryVersion};

pub fn standard_registry() -> StdRegistry {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    core::register(&mut builder);
    effects::register(&mut builder);
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
    agent::register(&mut builder);
    runtime::register(&mut builder);
    security::register(&mut builder);
    host::register(&mut builder);
    builder.finish()
}
