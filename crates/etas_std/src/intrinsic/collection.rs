use super::StdIntrinsicId;

// These explicit codes are part of stable intrinsic identity, not arena indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
#[repr(u32)]
pub enum StdCollectionKind {
    Array = 0,
    List = 1,
    Slice = 2,
    Map = 3,
    Range = 4,
    Deque = 5,
    Queue = 6,
    Stack = 7,
    PriorityQueue = 8,
    OrderedMap = 9,
    OrderedSet = 10,
}

impl StdCollectionKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Array => "Array",
            Self::List => "List",
            Self::Slice => "Slice",
            Self::Map => "Map",
            Self::Range => "Range",
            Self::Deque => "Deque",
            Self::Queue => "Queue",
            Self::Stack => "Stack",
            Self::PriorityQueue => "PriorityQueue",
            Self::OrderedMap => "OrderedMap",
            Self::OrderedSet => "OrderedSet",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
#[repr(u32)]
pub enum StdCollectionOperation {
    Get = 0,
    At = 1,
    Head = 2,
    Tail = 3,
    Push = 4,
    Pop = 5,
    Extend = 6,
    ToArray = 7,
    New = 8,
    WithCompare = 9,
    PushFront = 10,
    PushBack = 11,
    PopFront = 12,
    PopBack = 13,
    Insert = 14,
    Contains = 15,
    ContainsKey = 16,
    Closed = 17,
    Open = 18,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct StdCollectionIntrinsic {
    pub kind: StdCollectionKind,
    pub operation: StdCollectionOperation,
}

impl StdCollectionIntrinsic {
    pub const fn id(self) -> StdIntrinsicId {
        StdIntrinsicId(3_000 + self.kind as u32 * 32 + self.operation as u32)
    }

    pub fn is_supported(self) -> bool {
        use StdCollectionKind as K;
        use StdCollectionOperation as O;
        matches!(
            (self.kind, self.operation),
            (K::Array, O::Get | O::At | O::Push | O::Pop | O::Extend)
                | (
                    K::List,
                    O::Get | O::At | O::Head | O::Tail | O::Push | O::Pop | O::Extend
                )
                | (K::Slice, O::Get | O::At | O::ToArray)
                | (K::Map, O::Get)
                | (K::Range, O::Closed | O::Open)
                | (
                    K::Deque,
                    O::New | O::PushFront | O::PushBack | O::PopFront | O::PopBack
                )
                | (K::Queue | K::Stack, O::New | O::Push | O::Pop)
                | (K::PriorityQueue, O::New | O::WithCompare | O::Push | O::Pop)
                | (
                    K::OrderedMap,
                    O::New | O::WithCompare | O::Get | O::Insert | O::ContainsKey
                )
                | (
                    K::OrderedSet,
                    O::New | O::WithCompare | O::Insert | O::Contains
                )
        )
    }

    pub fn method_name(self) -> &'static str {
        use StdCollectionOperation as O;
        match self.operation {
            O::Get => "get",
            O::At => "at",
            O::Head => "head",
            O::Tail => "tail",
            O::Push => "push",
            O::Pop => "pop",
            O::Extend => "extend",
            O::ToArray => "to_array",
            O::New | O::WithCompare => "new",
            O::PushFront => "push_front",
            O::PushBack => "push_back",
            O::PopFront => "pop_front",
            O::PopBack => "pop_back",
            O::Insert => "insert",
            O::Contains => "contains",
            O::ContainsKey => "contains_key",
            O::Closed => "closed",
            O::Open => "open",
        }
    }
}
