//! Source-owned logical storage representations. These names identify checked
//! private Etas methods, with no native collection algorithm or default body.
pub const SOURCE_STORAGE_VIEW: &str = "__storage_view";
pub const SOURCE_STORAGE_BUILD: &str = "__storage_build";

pub fn source_storage_owner(module: &[String], declaration: &[String]) -> bool {
    module == ["std", "collections"]
        && matches!(declaration, [name] if matches!(name.as_str(),
            "Map" | "Set" | "Slice" | "Stack" | "Queue" | "Deque" | "BoundedDeque"))
}

pub fn source_storage_method(module: &[String], declaration: &[String], method: &str) -> bool {
    matches!(method, SOURCE_STORAGE_VIEW | SOURCE_STORAGE_BUILD)
        && source_storage_owner(module, declaration)
}
