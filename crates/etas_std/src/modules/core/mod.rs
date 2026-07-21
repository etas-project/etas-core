pub mod bytes;
pub mod collections;
pub mod json;
pub mod math;
pub mod option_result;
pub mod primitives;
pub mod text;

use crate::StdRegistryBuilder;

pub fn register(builder: &mut StdRegistryBuilder) {
    let core = builder.module(
        &["std", "core"],
        "Core primitive and universal declarations.",
    );
    primitives::register(builder, core);
    collections::register(builder);
    option_result::register(builder);
    text::register(builder);
    bytes::register(builder);
    json::register(builder);
    math::register(builder);
}
