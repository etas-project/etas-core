use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque, hash_map::RandomState},
    ffi::OsString,
    hash::BuildHasher,
    marker::PhantomData,
    mem::{align_of, size_of, size_of_val},
    num::*,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use super::{MemoryVisitor, RetainedSize};

macro_rules! inline {
    ($($ty:ty),* $(,)?) => {$(
        impl RetainedSize for $ty {
            fn visit_retained<'a>(&'a self, _: &mut MemoryVisitor<'a>) {}
            fn may_retain_heap() -> bool { false }
        }
    )*};
}

inline!(
    (),
    bool,
    char,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    NonZeroU8,
    NonZeroU16,
    NonZeroU32,
    NonZeroU64,
    NonZeroU128,
    NonZeroUsize,
    NonZeroI8,
    NonZeroI16,
    NonZeroI32,
    NonZeroI64,
    NonZeroI128,
    NonZeroIsize,
    Duration,
    Instant,
    SystemTime,
    RandomState
);

impl<T: ?Sized> RetainedSize for PhantomData<T> {
    fn visit_retained<'a>(&'a self, _: &mut MemoryVisitor<'a>) {}
    fn may_retain_heap() -> bool {
        false
    }
}

// References do not own their referents. Arc/Rc/Box below do.
impl<T: ?Sized> RetainedSize for &T {
    fn visit_retained<'a>(&'a self, _: &mut MemoryVisitor<'a>) {}
    fn may_retain_heap() -> bool {
        false
    }
}

impl RetainedSize for String {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        visitor.allocation(self.as_ptr() as usize, 1, self.capacity());
    }
}

impl RetainedSize for OsString {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        visitor.allocation(
            self.as_encoded_bytes().as_ptr() as usize,
            1,
            self.capacity(),
        );
    }
}

impl RetainedSize for PathBuf {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        visitor.allocation(
            self.as_os_str().as_encoded_bytes().as_ptr() as usize,
            1,
            self.capacity(),
        );
    }
}

impl<T: RetainedSize> RetainedSize for Vec<T> {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        visitor.allocation(
            self.as_ptr() as usize,
            1,
            self.capacity().saturating_mul(size_of::<T>()),
        );
        if T::may_retain_heap() {
            for value in self {
                visitor.enqueue(value);
            }
        }
    }
}

impl<T: RetainedSize> RetainedSize for VecDeque<T> {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        // The public API does not expose the allocation base of a split deque.
        visitor.allocation(
            self as *const Self as usize,
            4,
            self.capacity().saturating_mul(size_of::<T>()),
        );
        if T::may_retain_heap() {
            for value in self {
                visitor.enqueue(value);
            }
        }
    }
}

impl<T: RetainedSize> RetainedSize for Option<T> {
    fn may_retain_heap() -> bool {
        T::may_retain_heap()
    }
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        if let Some(value) = self {
            visitor.enqueue(value);
        }
    }
}

impl<T: RetainedSize, E: RetainedSize> RetainedSize for Result<T, E> {
    fn may_retain_heap() -> bool {
        T::may_retain_heap() || E::may_retain_heap()
    }
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        match self {
            Ok(value) => visitor.enqueue(value),
            Err(error) => visitor.enqueue(error),
        }
    }
}

impl<T: RetainedSize, const N: usize> RetainedSize for [T; N] {
    fn may_retain_heap() -> bool {
        N != 0 && T::may_retain_heap()
    }
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        for value in self {
            visitor.enqueue(value);
        }
    }
}

impl<T: RetainedSize> RetainedSize for Box<T> {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        if visitor.allocation(&**self as *const T as usize, 0, size_of::<T>()) {
            visitor.enqueue(&**self);
        }
    }
}

impl<T: RetainedSize> RetainedSize for Box<[T]> {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        if visitor.allocation(self.as_ptr() as usize, 0, size_of_val(&**self))
            && T::may_retain_heap()
        {
            for value in &**self {
                visitor.enqueue(value);
            }
        }
    }
}

impl RetainedSize for Box<str> {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        visitor.allocation(self.as_ptr() as usize, 0, self.len());
    }
}

macro_rules! shared {
    ($pointer:ident, $kind:literal) => {
        impl<T: RetainedSize> RetainedSize for $pointer<T> {
            fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
                let bytes = size_of::<T>()
                    .saturating_add(2 * size_of::<usize>())
                    .saturating_add(align_of::<T>() - 1);
                if visitor.allocation($pointer::as_ptr(self) as usize, $kind, bytes) {
                    visitor.enqueue(&**self);
                }
            }
        }
        impl<T: RetainedSize> RetainedSize for $pointer<[T]> {
            fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
                let bytes = size_of_val(&**self)
                    .saturating_add(2 * size_of::<usize>())
                    .saturating_add(align_of::<T>() - 1);
                if visitor.allocation(self.as_ptr() as usize, $kind, bytes) && T::may_retain_heap()
                {
                    for value in &**self {
                        visitor.enqueue(value);
                    }
                }
            }
        }
        impl RetainedSize for $pointer<str> {
            fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
                visitor.allocation(
                    self.as_ptr() as usize,
                    $kind,
                    self.len().saturating_add(2 * size_of::<usize>()),
                );
            }
        }
    };
}
shared!(Arc, 2);
shared!(Rc, 3);

// std exposes capacity, not bucket/node layout. Charge a conservative capacity
// envelope instead of pretending that serialized size measures resident memory.
pub fn hash_table_bytes<T>(capacity: usize) -> usize {
    if capacity == 0 {
        return 0;
    }
    capacity
        .saturating_mul(2)
        .saturating_mul(size_of::<T>().saturating_add(1))
        .saturating_add(2 * align_of::<T>())
        .saturating_add(128)
}

fn tree_bytes<T>(len: usize) -> usize {
    // B-tree nodes may retain spare slots after removals; at most one minimally
    // occupied node per entry, each with room for keys/values and child links.
    // An emptied tree can retain its root, so reserve one extra node as well.
    len.saturating_add(1).saturating_mul(
        16_usize
            .saturating_mul(size_of::<T>().saturating_add(size_of::<usize>()))
            .saturating_add(128),
    )
}

impl<K: RetainedSize, V: RetainedSize, S: BuildHasher + RetainedSize> RetainedSize
    for HashMap<K, V, S>
{
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        visitor.allocation(
            self as *const Self as usize,
            4,
            hash_table_bytes::<(K, V)>(self.capacity()),
        );
        visitor.enqueue(self.hasher());
        for (key, value) in self {
            visitor.enqueue(key);
            visitor.enqueue(value);
        }
    }
}

impl<T: RetainedSize, S: BuildHasher + RetainedSize> RetainedSize for HashSet<T, S> {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        visitor.allocation(
            self as *const Self as usize,
            4,
            hash_table_bytes::<T>(self.capacity()),
        );
        visitor.enqueue(self.hasher());
        for value in self {
            visitor.enqueue(value);
        }
    }
}

impl<K: RetainedSize, V: RetainedSize> RetainedSize for BTreeMap<K, V> {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        visitor.allocation(
            self as *const Self as usize,
            5,
            tree_bytes::<(K, V)>(self.len()),
        );
        for (key, value) in self {
            visitor.enqueue(key);
            visitor.enqueue(value);
        }
    }
}

impl<T: RetainedSize> RetainedSize for BTreeSet<T> {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        visitor.allocation(self as *const Self as usize, 5, tree_bytes::<T>(self.len()));
        for value in self {
            visitor.enqueue(value);
        }
    }
}

macro_rules! tuple {
    ($($name:ident:$index:tt),+) => {
        impl<$($name: RetainedSize),+> RetainedSize for ($($name,)+) {
            fn may_retain_heap() -> bool { false $(|| $name::may_retain_heap())+ }
            fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
                $(visitor.enqueue(&self.$index);)+
            }
        }
    };
}
tuple!(A:0);
tuple!(A:0,B:1);
tuple!(A:0,B:1,C:2);
tuple!(A:0,B:1,C:2,D:3);
tuple!(A:0,B:1,C:2,D:3,E:4);
tuple!(A:0,B:1,C:2,D:3,E:4,F:5);
tuple!(A:0,B:1,C:2,D:3,E:4,F:5,G:6);
tuple!(A:0,B:1,C:2,D:3,E:4,F:5,G:6,H:7);

impl RetainedSize for serde_json::Value {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        match self {
            Self::Null | Self::Bool(_) => {}
            Self::Number(number) => visitor.enqueue(number),
            Self::String(value) => visitor.enqueue(value),
            Self::Array(values) => visitor.enqueue(values),
            Self::Object(values) => visitor.enqueue(values),
        }
    }
}

impl RetainedSize for serde_json::Map<String, serde_json::Value> {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        // Works with both serde_json's BTreeMap and preserve_order backend.
        visitor.allocation(
            self as *const Self as usize,
            5,
            tree_bytes::<(String, serde_json::Value)>(self.len()),
        );
        for (key, value) in self {
            visitor.enqueue(key);
            visitor.enqueue(value);
        }
    }
}

impl RetainedSize for serde_json::Number {
    fn visit_retained<'a>(&'a self, visitor: &mut MemoryVisitor<'a>) {
        // Arbitrary-precision serde_json may own decimal storage. The type is
        // opaque; charge its printable capacity envelope on every feature set.
        visitor.allocation(self as *const Self as usize, 6, self.to_string().len());
    }
}
