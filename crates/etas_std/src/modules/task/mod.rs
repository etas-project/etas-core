mod effects;
mod flows;
mod types;

pub fn register(builder: &mut crate::StdRegistryBuilder) {
    let module = builder.module(
        &["std", "task"],
        "Structured concurrency tasks and scheduler-scoped operations.",
    );
    types::register(builder, module);
    effects::register(builder, module);
    flows::register(builder, module);
}
