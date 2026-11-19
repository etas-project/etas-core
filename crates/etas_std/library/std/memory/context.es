module std.memory;

// Readonly execution limits/clock snapshot. Private fields keep construction
// inside this module; actual enforcement and replay remain runtime mechanisms.
public type StorageContext = {
    private max_value_bytes: usize,
    private max_result_bytes: usize,
    private max_nodes: usize,
    private max_depth: usize,
    private max_scan_rows: usize,
    private max_page_entries: usize,
    private receipt_retention_seconds: u64,
    private now_seconds: u64,
};

impl StorageContext {
    @std.role("std.memory.StorageContext.current")
    public flow current() -> StorageContext ![];

    // Actual OS entropy only; source constructs operation keys/fingerprints.
    @std.role("std.memory.StorageContext.nonce")
    public flow nonce() -> Result<u128, StorageError> ![];
}
