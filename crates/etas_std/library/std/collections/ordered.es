module std.collections;

enum OrderedTree<K, V> { Empty, Branch(OrderedNode<K, V>) }
type OrderedNode<K, V> = { key: K, value: V, left: OrderedTree<K, V>, right: OrderedTree<K, V>, height: usize, count: usize };

impl OrderedTree<K, V> {
    flow height(self) -> usize ![] {
        return match self { OrderedTree.Empty => 0, OrderedTree.Branch(node) => node.height };
    }
    flow count(self) -> usize ![] {
        return match self { OrderedTree.Empty => 0, OrderedTree.Branch(node) => node.count };
    }
    flow branch(key: K, value: V, left: OrderedTree<K, V>, right: OrderedTree<K, V>) -> OrderedTree<K, V> ![] {
        let lh = left.height(); let rh = right.height();
        return OrderedTree.Branch(OrderedNode { key= key, value= value, left= left, right= right,
            height= 1 + if lh > rh { lh } else { rh }, count= 1 + left.count() + right.count() });
    }
    flow rotate_left(self) -> OrderedTree<K, V> ![] {
        let root = match self { OrderedTree.Branch(node) => node, OrderedTree.Empty => std.core.abort("AVL rotation invariant") };
        let pivot = match root.right { OrderedTree.Branch(node) => node, OrderedTree.Empty => std.core.abort("AVL rotation invariant") };
        return OrderedTree.branch(pivot.key, pivot.value, OrderedTree.branch(root.key, root.value, root.left, pivot.left), pivot.right);
    }
    flow rotate_right(self) -> OrderedTree<K, V> ![] {
        let root = match self { OrderedTree.Branch(node) => node, OrderedTree.Empty => std.core.abort("AVL rotation invariant") };
        let pivot = match root.left { OrderedTree.Branch(node) => node, OrderedTree.Empty => std.core.abort("AVL rotation invariant") };
        return OrderedTree.branch(pivot.key, pivot.value, pivot.left, OrderedTree.branch(root.key, root.value, pivot.right, root.right));
    }
    flow balance(self) -> OrderedTree<K, V> ![] {
        return match self {
            OrderedTree.Empty => self,
            OrderedTree.Branch(node) => {
                if node.left.height() > node.right.height() + 1 {
                    let left = match node.left {
                        OrderedTree.Empty => std.core.abort("AVL balance invariant"),
                        OrderedTree.Branch(child) => if child.right.height() > child.left.height() { node.left.rotate_left() } else { node.left },
                    };
                    return OrderedTree.branch(node.key, node.value, left, node.right).rotate_right();
                }
                if node.right.height() > node.left.height() + 1 {
                    let right = match node.right {
                        OrderedTree.Empty => std.core.abort("AVL balance invariant"),
                        OrderedTree.Branch(child) => if child.left.height() > child.right.height() { node.right.rotate_right() } else { node.right },
                    };
                    return OrderedTree.branch(node.key, node.value, node.left, right).rotate_left();
                }
                return self;
            },
        };
    }
    flow insert(self, key: K, value: V, compare: (K, K) -> i32 ![]) -> (OrderedTree<K, V>, bool) ![] {
        return match self {
            OrderedTree.Empty => (OrderedTree.branch(key, value, OrderedTree.Empty, OrderedTree.Empty), true),
            OrderedTree.Branch(node) => {
                let order = compare(key, node.key);
                if order == 0 { return (OrderedTree.branch(node.key, value, node.left, node.right), false); }
                if order < 0 {
                    let (left, added) = node.left.insert(key, value, compare);
                    return (OrderedTree.branch(node.key, node.value, left, node.right).balance(), added);
                }
                let (right, added) = node.right.insert(key, value, compare);
                return (OrderedTree.branch(node.key, node.value, node.left, right).balance(), added);
            },
        };
    }
    flow joined(key: K, value: V, left: OrderedTree<K, V>, right: OrderedTree<K, V>) -> OrderedTree<K, V> ![] {
        if left.height() > right.height() + 1 {
            let node = match left { OrderedTree.Branch(node) => node, OrderedTree.Empty => std.core.abort("AVL join invariant") };
            let tail = OrderedTree.joined(key, value, node.right, right);
            return OrderedTree.branch(node.key, node.value, node.left, tail).balance();
        }
        if right.height() > left.height() + 1 {
            let node = match right { OrderedTree.Branch(node) => node, OrderedTree.Empty => std.core.abort("AVL join invariant") };
            let head = OrderedTree.joined(key, value, left, node.left);
            return OrderedTree.branch(node.key, node.value, head, node.right).balance();
        }
        return OrderedTree.branch(key, value, left, right);
    }
    flow pop_first(self) -> (OrderedTree<K, V>, K, V) ![] {
        let node = match self { OrderedTree.Branch(node) => node, OrderedTree.Empty => std.core.abort("nonempty AVL invariant") };
        return match node.left {
            OrderedTree.Empty => (node.right, node.key, node.value),
            OrderedTree.Branch(_) => {
                let (left, key, value) = node.left.pop_first();
                (OrderedTree.branch(node.key, node.value, left, node.right).balance(), key, value)
            },
        };
    }
    flow removed(self, key: K, compare: (K, K) -> i32 ![]) -> (OrderedTree<K, V>, Option<V>) ![] {
        return match self {
            OrderedTree.Empty => (self, None),
            OrderedTree.Branch(node) => {
                let order = compare(key, node.key);
                if order < 0 {
                    let (left, previous) = node.left.removed(key, compare);
                    if previous.is_none() { return (self, previous); }
                    return (OrderedTree.branch(node.key, node.value, left, node.right).balance(), previous);
                }
                if order > 0 {
                    let (right, previous) = node.right.removed(key, compare);
                    if previous.is_none() { return (self, previous); }
                    return (OrderedTree.branch(node.key, node.value, node.left, right).balance(), previous);
                }
                return match node.right {
                    OrderedTree.Empty => (node.left, Some(node.value)),
                    OrderedTree.Branch(_) => {
                        let (right, successor, value) = node.right.pop_first();
                        (OrderedTree.branch(successor, value, node.left, right).balance(), Some(node.value))
                    },
                };
            },
        };
    }
    flow split(self, key: K, compare: (K, K) -> i32 ![]) -> (OrderedTree<K, V>, OrderedTree<K, V>) ![] {
        return match self {
            OrderedTree.Empty => (self, self),
            OrderedTree.Branch(node) => {
                if compare(node.key, key) < 0 {
                    let (left, right) = node.right.split(key, compare);
                    return (OrderedTree.joined(node.key, node.value, node.left, left), right);
                }
                let (left, right) = node.left.split(key, compare);
                return (left, OrderedTree.joined(node.key, node.value, right, node.right));
            },
        };
    }
    flow first(self) -> Option<(K, V)> ![] {
        var tree = self;
        var first: Option<(K, V)> = None;
        while true {
            match tree {
                OrderedTree.Empty => { return first; },
                OrderedTree.Branch(node) => { first = Some((node.key, node.value)); tree = node.left; },
            };
        }
    }
    flow last(self) -> Option<(K, V)> ![] {
        var tree = self;
        var last: Option<(K, V)> = None;
        while true {
            match tree {
                OrderedTree.Empty => { return last; },
                OrderedTree.Branch(node) => { last = Some((node.key, node.value)); tree = node.right; },
            };
        }
    }
    flow bound(self, key: K, strict: bool, compare: (K, K) -> i32 ![]) -> Option<(K, V)> ![] {
        var tree = self;
        var best: Option<(K, V)> = None;
        while true {
            match tree {
                OrderedTree.Empty => { return best; },
                OrderedTree.Branch(node) => {
                    let order = compare(node.key, key);
                    if order > 0 || (!strict && order == 0) { best = Some((node.key, node.value)); tree = node.left; }
                    else { tree = node.right; }
                },
            };
        }
    }
    flow range_cursor(self, pending: List<OrderedNode<K, V>>, lower: Option<K>, upper: Option<K>, include_lower: bool, include_upper: bool, compare: (K, K) -> i32 ![]) -> Iter<(K, V)> ![] {
        return Iter { next= () => {
            var tree = self;
            var stack = pending;
            while true {
                match tree {
                    OrderedTree.Empty => { break; },
                    OrderedTree.Branch(node) => {
                        let before = match lower { None => false, Some(key) => { let order = compare(node.key, key); order < 0 || (order == 0 && !include_lower) } };
                        if before { tree = node.right; }
                        else { stack = node :: stack; tree = node.left; }
                    },
                };
            }
            return match stack {
                [] => None,
                node :: rest => {
                    let after = match upper { None => false, Some(key) => { let order = compare(node.key, key); order > 0 || (order == 0 && !include_upper) } };
                    if after { return None; }
                    Some(((node.key, node.value), node.right.range_cursor(rest, None, upper, true, include_upper, compare)))
                },
            };
        } };
    }
    flow from_sorted(values: Array<(K, V)>, start: usize, end: usize) -> OrderedTree<K, V> ![] {
        if start == end { return OrderedTree.Empty; }
        let middle = start + (end - start) / 2;
        let (key, value) = values.get(middle).unwrap();
        return OrderedTree.branch(key, value, OrderedTree.from_sorted(values, start, middle), OrderedTree.from_sorted(values, middle + 1, end));
    }
}

type ComparatorIdentity;
impl ComparatorIdentity {
    @std.role("std.collections.ComparatorIdentity.capture")
    flow capture<T>(compare: (T, T) -> i32 ![]) -> ComparatorIdentity ![];
    @std.role("std.collections.ComparatorIdentity.same")
    flow same(self, other: ComparatorIdentity) -> bool ![];
}
enum ComparatorProfile { Standard, Custom(ComparatorIdentity) }
impl ComparatorProfile {
    flow matches(self, other: ComparatorProfile) -> bool ![] {
        return match (self, other) {
            (ComparatorProfile.Standard, ComparatorProfile.Standard) => true,
            (ComparatorProfile.Custom(left), ComparatorProfile.Custom(right)) => left.same(right),
            _ => false,
        };
    }
}

public type OrderedMap<K, V> = {
    private tree: OrderedTree<K, V>, private compare: (K, K) -> i32 ![], private profile: ComparatorProfile,
};
impl OrderedMap<K ~ Order, V> {
    public flow new() -> OrderedMap<K, V> ![] {
        return OrderedMap { tree= OrderedTree.Empty, compare= (a: K, b: K) => a.compare(b), profile= ComparatorProfile.Standard };
    }
    public flow from_entries(items: Iter<(K, V)>) -> OrderedMap<K, V> ![] {
        var tree: OrderedTree<K, V> = OrderedTree.Empty;
        let compare = (a: K, b: K) => a.compare(b);
        for (key, value) in items { let (next, _) = tree.insert(key, value, compare); tree = next; }
        return OrderedMap { tree= tree, compare= compare, profile= ComparatorProfile.Standard };
    }
}
impl OrderedMap<K, V> {
    public flow new(compare: (K, K) -> i32 ![]) -> OrderedMap<K, V> ![] {
        return OrderedMap { tree= OrderedTree.Empty, compare= compare, profile= ComparatorProfile.Custom(ComparatorIdentity.capture(compare)) };
    }
    public flow from_entries(items: Iter<(K, V)>, compare: (K, K) -> i32 ![]) -> OrderedMap<K, V> ![] {
        var tree: OrderedTree<K, V> = OrderedTree.Empty;
        for (key, value) in items { let (next, _) = tree.insert(key, value, compare); tree = next; }
        return OrderedMap { tree= tree, compare= compare, profile= ComparatorProfile.Custom(ComparatorIdentity.capture(compare)) };
    }
    public flow get(self, key: K) -> Option<V> ![] {
        var tree = self.tree;
        while true {
            match tree {
                OrderedTree.Empty => { return None; },
                OrderedTree.Branch(node) => {
                    let order = (self.compare)(key, node.key);
                    if order == 0 { return Some(node.value); }
                    tree = if order < 0 { node.left } else { node.right };
                },
            };
        }
    }
    public flow contains_key(self, key: K) -> bool ![] { return self.get(key).is_some(); }
    public flow insert(self, key: K, value: V) -> OrderedMap<K, V> ![] {
        let (tree, _) = self.tree.insert(key, value, self.compare);
        return OrderedMap { tree= tree, compare= self.compare, profile= self.profile };
    }
    public flow remove(self, key: K) -> OrderedMap<K, V> ![] {
        let (tree, _) = self.tree.removed(key, self.compare);
        return OrderedMap { tree= tree, compare= self.compare, profile= self.profile };
    }
    public flow take(self, key: K) -> (OrderedMap<K, V>, Option<V>) ![] {
        let (tree, old) = self.tree.removed(key, self.compare);
        return (OrderedMap { tree= tree, compare= self.compare, profile= self.profile }, old);
    }
    public flow len(self) -> usize ![] { return self.tree.count(); }
    public flow is_empty(self) -> bool ![] { return self.tree.count() == 0; }
    public flow first(self) -> Option<(K, V)> ![] { return self.tree.first(); }
    public flow last(self) -> Option<(K, V)> ![] { return self.tree.last(); }
    public flow lower_bound(self, key: K) -> Option<(K, V)> ![] { return self.tree.bound(key, false, self.compare); }
    public flow upper_bound(self, key: K) -> Option<(K, V)> ![] { return self.tree.bound(key, true, self.compare); }
    public flow keys(self) -> Iter<K> ![] { return self.iter().map((entry: (K, V)) => { let (key, _) = entry; return key; }); }
    public flow values(self) -> Iter<V> ![] { return self.iter().map((entry: (K, V)) => { let (_, value) = entry; return value; }); }
    public flow range(self, lower: Option<K>, upper: Option<K>, include_lower: bool = true, include_upper: bool = false) -> Iter<(K, V)> ![] {
        return self.tree.range_cursor([], lower, upper, include_lower, include_upper, self.compare);
    }
    public flow split_at_key(self, key: K) -> (OrderedMap<K, V>, OrderedMap<K, V>) ![] {
        let (left, right) = self.tree.split(key, self.compare);
        return (OrderedMap { tree= left, compare= self.compare, profile= self.profile }, OrderedMap { tree= right, compare= self.compare, profile= self.profile });
    }
    public flow clear(self) -> OrderedMap<K, V> ![] { return OrderedMap { tree= OrderedTree.Empty, compare= self.compare, profile= self.profile }; }
}
impl OrderedMap<K, V> ~ Iterable<(K, V)> {
    public flow iter(self) -> Iter<(K, V)> ![] { return self.tree.range_cursor([], None, None, true, false, self.compare); }
}

public type OrderedSet<T> = { private tree: OrderedTree<T, unit>, private compare: (T, T) -> i32 ![], private profile: ComparatorProfile };
impl OrderedSet<T ~ Order> {
    public flow new() -> OrderedSet<T> ![] {
        return OrderedSet { tree= OrderedTree.Empty, compare= (a: T, b: T) => a.compare(b), profile= ComparatorProfile.Standard };
    }
    public flow from_iter(items: Iter<T>) -> OrderedSet<T> ![] {
        var tree: OrderedTree<T, unit> = OrderedTree.Empty;
        let compare = (a: T, b: T) => a.compare(b);
        for value in items { let (next, _) = tree.insert(value, (), compare); tree = next; }
        return OrderedSet { tree= tree, compare= compare, profile= ComparatorProfile.Standard };
    }
}
impl OrderedSet<T> {
    public flow new(compare: (T, T) -> i32 ![]) -> OrderedSet<T> ![] {
        return OrderedSet { tree= OrderedTree.Empty, compare= compare, profile= ComparatorProfile.Custom(ComparatorIdentity.capture(compare)) };
    }
    public flow from_iter(items: Iter<T>, compare: (T, T) -> i32 ![]) -> OrderedSet<T> ![] {
        var tree: OrderedTree<T, unit> = OrderedTree.Empty;
        for value in items { let (next, _) = tree.insert(value, (), compare); tree = next; }
        return OrderedSet { tree= tree, compare= compare, profile= ComparatorProfile.Custom(ComparatorIdentity.capture(compare)) };
    }
    public flow contains(self, value: T) -> bool ![] {
        var tree = self.tree;
        while true {
            match tree {
                OrderedTree.Empty => { return false; },
                OrderedTree.Branch(node) => {
                    let order = (self.compare)(value, node.key);
                    if order == 0 { return true; }
                    tree = if order < 0 { node.left } else { node.right };
                },
            };
        }
    }
    public flow insert(self, value: T) -> OrderedSet<T> ![] {
        let (tree, _) = self.tree.insert(value, (), self.compare);
        return OrderedSet { tree= tree, compare= self.compare, profile= self.profile };
    }
    public flow remove(self, value: T) -> OrderedSet<T> ![] {
        let (tree, _) = self.tree.removed(value, self.compare);
        return OrderedSet { tree= tree, compare= self.compare, profile= self.profile };
    }
    public flow len(self) -> usize ![] { return self.tree.count(); }
    public flow is_empty(self) -> bool ![] { return self.tree.count() == 0; }
    public flow first(self) -> Option<T> ![] { return match self.tree.first() { None => None, Some((value, _)) => Some(value) }; }
    public flow last(self) -> Option<T> ![] { return match self.tree.last() { None => None, Some((value, _)) => Some(value) }; }
    public flow lower_bound(self, value: T) -> Option<T> ![] { return match self.tree.bound(value, false, self.compare) { None => None, Some((key, _)) => Some(key) }; }
    public flow upper_bound(self, value: T) -> Option<T> ![] { return match self.tree.bound(value, true, self.compare) { None => None, Some((key, _)) => Some(key) }; }
    public flow range(self, lower: Option<T>, upper: Option<T>, include_lower: bool = true, include_upper: bool = false) -> Iter<T> ![] {
        return self.tree.range_cursor([], lower, upper, include_lower, include_upper, self.compare).map((entry: (T, unit)) => { let (key, _) = entry; return key; });
    }
    public flow split_at_value(self, value: T) -> (OrderedSet<T>, OrderedSet<T>) ![] {
        let (left, right) = self.tree.split(value, self.compare);
        return (OrderedSet { tree= left, compare= self.compare, profile= self.profile }, OrderedSet { tree= right, compare= self.compare, profile= self.profile });
    }
    public flow clear(self) -> OrderedSet<T> ![] { return OrderedSet { tree= OrderedTree.Empty, compare= self.compare, profile= self.profile }; }
}
impl OrderedSet<T> ~ Iterable<T> {
    public flow iter(self) -> Iter<T> ![] {
        return self.tree.range_cursor([], None, None, true, false, self.compare).map((entry: (T, unit)) => { let (key, _) = entry; return key; });
    }
}
