mod reports;
mod service;
mod syntax;
mod types;

pub(super) fn register_actions(builder: &mut crate::StdRegistryBuilder) {
    service::register_actions(builder);
}

pub fn register(builder: &mut crate::StdRegistryBuilder) {
    let module = builder.module(
        &["std", "compiler"],
        "Retained code definitions and checked compilation.",
    );
    types::register(builder, module);
    reports::register(builder, module);
    syntax::register(builder, module);
    service::register(builder, module);
}
