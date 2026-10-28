use std::{
    collections::HashMap,
    mem::{align_of_val, size_of, size_of_val},
    sync::Arc,
};

use super::RetainedSize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AllocationId {
    address: usize,
    kind: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryAllocation {
    pub id: AllocationId,
    pub estimated_bytes: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryFootprint {
    pub allocations: Vec<MemoryAllocation>,
}

impl MemoryFootprint {
    pub fn of<T: RetainedSize>(value: &T) -> Self {
        let mut visitor = MemoryVisitor::default();
        visitor.enqueue(value);
        visitor.finish()
    }

    pub fn for_boxed<T: RetainedSize>(value: &T) -> Self {
        let mut visitor = MemoryVisitor::default();
        visitor.allocation(value as *const T as usize, 0, size_of::<T>());
        visitor.enqueue(value);
        visitor.finish()
    }

    pub fn estimated_bytes(&self) -> u64 {
        self.allocations.iter().fold(0_u64, |total, allocation| {
            total.saturating_add(allocation.estimated_bytes)
        })
    }

    pub fn accounting_bytes(&self) -> u64 {
        (self.allocations.capacity() as u64).saturating_mul(size_of::<MemoryAllocation>() as u64)
    }
}

#[derive(Default)]
pub struct MemoryVisitor<'a> {
    pending: Vec<&'a dyn RetainedSize>,
    allocations: HashMap<AllocationId, u64>,
}

impl<'a> MemoryVisitor<'a> {
    /// Accounts for an opaque shared backing without downcasting or counting
    /// the same allocation again through another trait-object reference.
    pub fn visit_shared_arc<T: RetainedSize + ?Sized>(&mut self, value: &'a Arc<T>) {
        let bytes = size_of_val(&**value)
            .saturating_add(2 * size_of::<usize>())
            .saturating_add(align_of_val(&**value) - 1);
        if self.allocation(Arc::as_ptr(value).cast::<()>() as usize, 2, bytes) {
            (**value).visit_retained(self);
        }
    }

    pub fn enqueue<T: RetainedSize>(&mut self, value: &'a T) {
        if T::may_retain_heap() {
            self.pending.push(value);
        }
    }

    pub(crate) fn allocation(&mut self, address: usize, kind: u8, bytes: usize) -> bool {
        if bytes == 0 {
            return false;
        }
        let id = AllocationId { address, kind };
        match self.allocations.entry(id) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                *entry.get_mut() = (*entry.get()).max(bytes as u64);
                false
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(bytes as u64);
                true
            }
        }
    }

    pub fn finish(mut self) -> MemoryFootprint {
        while let Some(value) = self.pending.pop() {
            value.visit_retained(&mut self);
        }
        let mut allocations: Vec<_> = self
            .allocations
            .into_iter()
            .map(|(id, estimated_bytes)| MemoryAllocation {
                id,
                estimated_bytes,
            })
            .collect();
        allocations.sort_unstable_by_key(|allocation| allocation.id);
        MemoryFootprint { allocations }
    }
}
