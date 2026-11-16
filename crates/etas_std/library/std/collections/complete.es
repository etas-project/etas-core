module std.collections;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Standard generic collection type.
@std.role("std.collections.Deque")
public type Deque<T>;

// Compiler-known support constraint accepted by std.collections.is_empty.
@std.role("std.collections.EmptinessInput")
public type EmptinessInput;

// Compiler-known support constraint accepted by std.collections.len.
@std.role("std.collections.LengthInput")
public type LengthInput;

// Standard generic collection type.
@std.role("std.collections.OrderedMap")
public type OrderedMap<K, V>;

// Standard generic collection type.
@std.role("std.collections.OrderedSet")
public type OrderedSet<T>;

// Standard generic collection type.
@std.role("std.collections.PriorityQueue")
public type PriorityQueue<T, P>;

// Standard generic collection type.
@std.role("std.collections.Queue")
public type Queue<T>;

// Standard generic collection type.
@std.role("std.collections.Stack")
public type Stack<T>;

// Return an array element or raise Error[IndexError] when out of range.
@std.binding("std.collections.array_at")
public flow array_at<T, I ~ std.core.Index>(arg0: std.collections.Array<T>, arg1: I) -> T ![Error<std.runtime.error.IndexError>] {}

// Construct a closed range.
@std.binding("std.collections.closed")
public flow closed<T ~ std.core.Index>(arg0: T, arg1: T) -> std.collections.Range<T> ![] {}

// Return whether a map contains the requested key.
@std.binding("std.collections.contains_key")
public flow contains_key<K, V>(arg0: std.collections.Map<K, V>, arg1: K) -> bool ![] {}

// Construct an empty deque with explicit type arguments.
@std.binding("std.collections.deque_new")
public flow deque_new<T>() -> std.collections.Deque<T> ![] {}

// Return a deque without the last value and the removed value.
@std.binding("std.collections.deque_pop_back")
public flow deque_pop_back<T>(arg0: std.collections.Deque<T>) -> (std.collections.Deque<T>, std.option.Option<T>) ![] {}

// Return a deque without the first value and the removed value.
@std.binding("std.collections.deque_pop_front")
public flow deque_pop_front<T>(arg0: std.collections.Deque<T>) -> (std.collections.Deque<T>, std.option.Option<T>) ![] {}

// Return a deque with a value appended at the back.
@std.binding("std.collections.deque_push_back")
public flow deque_push_back<T>(arg0: std.collections.Deque<T>, arg1: T) -> std.collections.Deque<T> ![] {}

// Return a deque with a value inserted at the front.
@std.binding("std.collections.deque_push_front")
public flow deque_push_front<T>(arg0: std.collections.Deque<T>, arg1: T) -> std.collections.Deque<T> ![] {}

// Return whether an Array, List, Slice, Map, string, or bytes value is empty.
@std.binding("std.collections.is_empty")
public flow is_empty(arg0: std.collections.EmptinessInput) -> bool ![] {}

// Return the length of an Array, List, Slice, Map, string, or bytes value.
@std.binding("std.collections.len")
public flow len(arg0: std.collections.LengthInput) -> usize ![] {}

// Return a list element or raise Error[IndexError] when out of range.
@std.binding("std.collections.list_at")
public flow list_at<T, I ~ std.core.Index>(arg0: std.collections.List<T>, arg1: I) -> T ![Error<std.runtime.error.IndexError>] {}

// Return a map value when the key exists.
@std.binding("std.collections.map_get")
public flow map_get<K, V>(arg0: std.collections.Map<K, V>, arg1: K) -> std.option.Option<V> ![] {}

// Construct an open range.
@std.binding("std.collections.open")
public flow open<T ~ std.core.Index>(arg0: T, arg1: T) -> std.collections.Range<T> ![] {}

// Return whether an ordered map contains the requested key.
@std.binding("std.collections.ordered_map_contains_key")
public flow ordered_map_contains_key<K, V>(arg0: std.collections.OrderedMap<K, V>, arg1: K) -> bool ![] {}

// Return an ordered-map value when the key exists.
@std.binding("std.collections.ordered_map_get")
public flow ordered_map_get<K, V>(arg0: std.collections.OrderedMap<K, V>, arg1: K) -> std.option.Option<V> ![] {}

// Return an ordered map with a key/value inserted or replaced.
@std.binding("std.collections.ordered_map_insert")
public flow ordered_map_insert<K, V>(arg0: std.collections.OrderedMap<K, V>, arg1: K, arg2: V) -> std.collections.OrderedMap<K, V> ![] {}

// Construct an empty ordered map with explicit type arguments.
@std.binding("std.collections.ordered_map_new")
public flow ordered_map_new<K, V>() -> std.collections.OrderedMap<K, V> ![] {}

// Construct an ordered collection with a pure key comparison flow.
@std.binding("std.collections.ordered_map_with_compare")
public flow ordered_map_with_compare<K, V>(arg0: (K, K) -> i32 ![]) -> std.collections.OrderedMap<K, V> ![] {}

// Return whether an ordered set contains the value.
@std.binding("std.collections.ordered_set_contains")
public flow ordered_set_contains<T>(arg0: std.collections.OrderedSet<T>, arg1: T) -> bool ![] {}

// Return an ordered set containing the value.
@std.binding("std.collections.ordered_set_insert")
public flow ordered_set_insert<T>(arg0: std.collections.OrderedSet<T>, arg1: T) -> std.collections.OrderedSet<T> ![] {}

// Construct an empty ordered set with explicit type arguments.
@std.binding("std.collections.ordered_set_new")
public flow ordered_set_new<T>() -> std.collections.OrderedSet<T> ![] {}

// Construct an ordered collection with a pure key comparison flow.
@std.binding("std.collections.ordered_set_with_compare")
public flow ordered_set_with_compare<T>(arg0: (T, T) -> i32 ![]) -> std.collections.OrderedSet<T> ![] {}

// Construct an empty priority queue with explicit type arguments.
@std.binding("std.collections.priority_queue_new")
public flow priority_queue_new<T, P>() -> std.collections.PriorityQueue<T, P> ![] {}

// Return a priority queue without the next value and the removed value.
@std.binding("std.collections.priority_queue_pop")
public flow priority_queue_pop<T, P>(arg0: std.collections.PriorityQueue<T, P>) -> (std.collections.PriorityQueue<T, P>, std.option.Option<T>) ![] {}

// Return a priority queue with a prioritized value.
@std.binding("std.collections.priority_queue_push")
public flow priority_queue_push<T, P>(arg0: std.collections.PriorityQueue<T, P>, arg1: T, arg2: P) -> std.collections.PriorityQueue<T, P> ![] {}

// Construct a stable priority queue with a pure priority comparison flow.
@std.binding("std.collections.priority_queue_with_compare")
public flow priority_queue_with_compare<T, P>(arg0: (P, P) -> i32 ![]) -> std.collections.PriorityQueue<T, P> ![] {}

// Construct an empty queue with explicit type arguments.
@std.binding("std.collections.queue_new")
public flow queue_new<T>() -> std.collections.Queue<T> ![] {}

// Return a queue without the next value and the removed value.
@std.binding("std.collections.queue_pop")
public flow queue_pop<T>(arg0: std.collections.Queue<T>) -> (std.collections.Queue<T>, std.option.Option<T>) ![] {}

// Return a queue with a value enqueued.
@std.binding("std.collections.queue_push")
public flow queue_push<T>(arg0: std.collections.Queue<T>, arg1: T) -> std.collections.Queue<T> ![] {}

// Return a slice element or raise Error[IndexError] when out of range.
@std.binding("std.collections.slice_at")
public flow slice_at<T, I ~ std.core.Index>(arg0: std.collections.Slice<T>, arg1: I) -> T ![Error<std.runtime.error.IndexError>] {}

// Return a slice element when the checked index is in range.
@std.binding("std.collections.slice_get")
public flow slice_get<T, I ~ std.core.Index>(arg0: std.collections.Slice<T>, arg1: I) -> std.option.Option<T> ![] {}

// Construct an empty stack with explicit type arguments.
@std.binding("std.collections.stack_new")
public flow stack_new<T>() -> std.collections.Stack<T> ![] {}

// Return a stack without the top value and the removed value.
@std.binding("std.collections.stack_pop")
public flow stack_pop<T>(arg0: std.collections.Stack<T>) -> (std.collections.Stack<T>, std.option.Option<T>) ![] {}

// Return a stack with a value pushed on top.
@std.binding("std.collections.stack_push")
public flow stack_push<T>(arg0: std.collections.Stack<T>, arg1: T) -> std.collections.Stack<T> ![] {}

// Materialize a slice as an array.
@std.binding("std.collections.to_array")
public flow to_array<T>(arg0: std.collections.Slice<T>) -> std.collections.Array<T> ![] {}

impl std.collections.Array<T> {
    public flow at<I ~ std.core.Index>(self, arg0: I) -> T ![Error<std.runtime.error.IndexError>] { return std.collections.array_at(self, arg0); }
}

impl std.collections.Deque<T> {
    public flow new() -> std.collections.Deque<T> ![] { return std.collections.deque_new(); }
    public flow pop_back(self) -> (std.collections.Deque<T>, std.option.Option<T>) ![] { return std.collections.deque_pop_back(self); }
    public flow pop_front(self) -> (std.collections.Deque<T>, std.option.Option<T>) ![] { return std.collections.deque_pop_front(self); }
    public flow push_back(self, arg0: T) -> std.collections.Deque<T> ![] { return std.collections.deque_push_back(self, arg0); }
    public flow push_front(self, arg0: T) -> std.collections.Deque<T> ![] { return std.collections.deque_push_front(self, arg0); }
}

impl std.collections.EmptinessInput {
    public flow is_empty(self) -> bool ![] { return std.collections.is_empty(self); }
}

impl std.collections.LengthInput {
    public flow len(self) -> usize ![] { return std.collections.len(self); }
}

impl std.collections.List<T> {
    public flow at<I ~ std.core.Index>(self, arg0: I) -> T ![Error<std.runtime.error.IndexError>] { return std.collections.list_at(self, arg0); }
}

impl std.collections.Map<K, V> {
    public flow contains_key(self, arg0: K) -> bool ![] { return std.collections.contains_key(self, arg0); }
    public flow get(self, arg0: K) -> std.option.Option<V> ![] { return std.collections.map_get(self, arg0); }
}

impl std.collections.OrderedMap<K, V> {
    public flow contains_key(self, arg0: K) -> bool ![] { return std.collections.ordered_map_contains_key(self, arg0); }
    public flow get(self, arg0: K) -> std.option.Option<V> ![] { return std.collections.ordered_map_get(self, arg0); }
    public flow insert(self, arg0: K, arg1: V) -> std.collections.OrderedMap<K, V> ![] { return std.collections.ordered_map_insert(self, arg0, arg1); }
    public flow new() -> std.collections.OrderedMap<K, V> ![] { return std.collections.ordered_map_new(); }
    public flow new(arg0: (K, K) -> i32 ![]) -> std.collections.OrderedMap<K, V> ![] { return std.collections.ordered_map_with_compare(arg0); }
}

impl std.collections.OrderedSet<T> {
    public flow contains(self, arg0: T) -> bool ![] { return std.collections.ordered_set_contains(self, arg0); }
    public flow insert(self, arg0: T) -> std.collections.OrderedSet<T> ![] { return std.collections.ordered_set_insert(self, arg0); }
    public flow new() -> std.collections.OrderedSet<T> ![] { return std.collections.ordered_set_new(); }
    public flow new(arg0: (T, T) -> i32 ![]) -> std.collections.OrderedSet<T> ![] { return std.collections.ordered_set_with_compare(arg0); }
}

impl std.collections.PriorityQueue<T, P> {
    public flow new() -> std.collections.PriorityQueue<T, P> ![] { return std.collections.priority_queue_new(); }
    public flow pop(self) -> (std.collections.PriorityQueue<T, P>, std.option.Option<T>) ![] { return std.collections.priority_queue_pop(self); }
    public flow push(self, arg0: T, arg1: P) -> std.collections.PriorityQueue<T, P> ![] { return std.collections.priority_queue_push(self, arg0, arg1); }
    public flow new(arg0: (P, P) -> i32 ![]) -> std.collections.PriorityQueue<T, P> ![] { return std.collections.priority_queue_with_compare(arg0); }
}

impl std.collections.Queue<T> {
    public flow new() -> std.collections.Queue<T> ![] { return std.collections.queue_new(); }
    public flow pop(self) -> (std.collections.Queue<T>, std.option.Option<T>) ![] { return std.collections.queue_pop(self); }
    public flow push(self, arg0: T) -> std.collections.Queue<T> ![] { return std.collections.queue_push(self, arg0); }
}

impl std.collections.Range<T> {
    public flow closed(arg0: T, arg1: T) -> std.collections.Range<T> ![] { return std.collections.closed(arg0, arg1); }
    public flow open(arg0: T, arg1: T) -> std.collections.Range<T> ![] { return std.collections.open(arg0, arg1); }
}

impl std.collections.Slice<T> {
    public flow at<I ~ std.core.Index>(self, arg0: I) -> T ![Error<std.runtime.error.IndexError>] { return std.collections.slice_at(self, arg0); }
    public flow get<I ~ std.core.Index>(self, arg0: I) -> std.option.Option<T> ![] { return std.collections.slice_get(self, arg0); }
    public flow to_array(self) -> std.collections.Array<T> ![] { return std.collections.to_array(self); }
}

impl std.collections.Stack<T> {
    public flow new() -> std.collections.Stack<T> ![] { return std.collections.stack_new(); }
    public flow pop(self) -> (std.collections.Stack<T>, std.option.Option<T>) ![] { return std.collections.stack_pop(self); }
    public flow push(self, arg0: T) -> std.collections.Stack<T> ![] { return std.collections.stack_push(self, arg0); }
}
