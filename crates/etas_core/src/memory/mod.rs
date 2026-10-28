mod impls;
mod visitor;

#[cfg(test)]
mod tests;

pub use impls::hash_table_bytes;
pub use visitor::{AllocationId, MemoryAllocation, MemoryFootprint, MemoryVisitor};

/// Visits owned storage only. Capacity-based accounting excludes allocator
/// overhead; opaque standard-container layouts use conservative estimates.
pub trait RetainedSize {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>);

    fn may_retain_heap() -> bool
    where
        Self: Sized,
    {
        true
    }
}
