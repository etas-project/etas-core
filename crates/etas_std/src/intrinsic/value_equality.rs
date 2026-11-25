//! Fixed language equality contracts whose algorithms are owned by Etas.
//! This identifies private source methods; it provides no native implementation.
pub const SOURCE_VALUE_EQUALITY_METHOD: &str = "__value_eq";

pub fn source_value_equality_owner(module: &[String], declaration: &[String]) -> bool {
    module == ["std", "collections"]
        && matches!(declaration, [name] if matches!(name.as_str(),
            "Map" | "Set" | "Slice" | "Stack" | "Queue" | "Deque" | "BoundedDeque"))
}
