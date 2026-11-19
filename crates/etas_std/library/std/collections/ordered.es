module std.collections;

enum OrderedTree<K, V> {
    Empty,
    Branch(OrderedNode<K, V>),
}

type OrderedNode<K, V> = {
    key: K,
    value: V,
    left: OrderedTree<K, V>,
    right: OrderedTree<K, V>,
    height: usize,
};

impl OrderedTree<K, V> {
    flow height(self) -> usize ![] {
        return match self {
            OrderedTree.Empty => 0,
            OrderedTree.Branch(node) => node.height,
        };
    }

    flow insert(self, key: K, value: V, compare: (K, K) -> i32 ![])
        -> (OrderedTree<K, V>, bool) ![] {
        return match self {
            OrderedTree.Empty => (
                OrderedTree.Branch(OrderedNode {
                    key= key, value= value,
                    left= OrderedTree.Empty, right= OrderedTree.Empty, height= 1,
                }),
                true,
            ),
            OrderedTree.Branch(node) => {
                let order = compare(key, node.key);
                if order == 0 {
                    return (OrderedTree.Branch(OrderedNode {
                        key= node.key, value= value, left= node.left, right= node.right, height= node.height,
                    }), false);
                }
                var left = node.left;
                var right = node.right;
                var added = false;
                if order < 0 {
                    let (updated, inserted) = left.insert(key, value, compare);
                    left = updated;
                    added = inserted;
                } else {
                    let (updated, inserted) = right.insert(key, value, compare);
                    right = updated;
                    added = inserted;
                }
                let left_height = left.height();
                let right_height = right.height();
                let height = 1 + if left_height > right_height { left_height } else { right_height };
                let updated = OrderedTree.Branch(OrderedNode {
                    key= node.key, value= node.value, left= left, right= right, height= height,
                });
                return (updated.balance(), added);
            },
        };
    }

    flow balance(self) -> OrderedTree<K, V> ![] {
        return match self {
            OrderedTree.Empty => self,
            OrderedTree.Branch(node) => {
                let left_height = node.left.height();
                let right_height = node.right.height();
                if left_height > right_height + 1 {
                    let left = match node.left {
                        OrderedTree.Branch(child) => {
                            if child.right.height() > child.left.height() {
                                node.left.rotate_left()
                            } else {
                                node.left
                            }
                        },
                        OrderedTree.Empty => std.core.abort("AVL balance invariant"),
                    };
                    return OrderedTree.Branch(OrderedNode {
                        key= node.key, value= node.value, left= left, right= node.right, height= node.height,
                    }).rotate_right();
                }
                if right_height > left_height + 1 {
                    let right = match node.right {
                        OrderedTree.Branch(child) => {
                            if child.left.height() > child.right.height() {
                                node.right.rotate_right()
                            } else {
                                node.right
                            }
                        },
                        OrderedTree.Empty => std.core.abort("AVL balance invariant"),
                    };
                    return OrderedTree.Branch(OrderedNode {
                        key= node.key, value= node.value, left= node.left, right= right, height= node.height,
                    }).rotate_left();
                }
                return self;
            },
        };
    }

    flow rotate_left(self) -> OrderedTree<K, V> ![] {
        let root = match self {
            OrderedTree.Branch(node) => node,
            OrderedTree.Empty => std.core.abort("AVL rotation invariant"),
        };
        let pivot = match root.right {
            OrderedTree.Branch(node) => node,
            OrderedTree.Empty => std.core.abort("AVL rotation invariant"),
        };
        let a = root.left.height();
        let b = pivot.left.height();
        let left_height = 1 + if a > b { a } else { b };
        let left = OrderedTree.Branch(OrderedNode {
            key= root.key, value= root.value, left= root.left, right= pivot.left, height= left_height,
        });
        let c = pivot.right.height();
        return OrderedTree.Branch(OrderedNode {
            key= pivot.key, value= pivot.value, left= left, right= pivot.right,
            height= 1 + if left_height > c { left_height } else { c },
        });
    }

    flow rotate_right(self) -> OrderedTree<K, V> ![] {
        let root = match self {
            OrderedTree.Branch(node) => node,
            OrderedTree.Empty => std.core.abort("AVL rotation invariant"),
        };
        let pivot = match root.left {
            OrderedTree.Branch(node) => node,
            OrderedTree.Empty => std.core.abort("AVL rotation invariant"),
        };
        let a = pivot.right.height();
        let b = root.right.height();
        let right_height = 1 + if a > b { a } else { b };
        let right = OrderedTree.Branch(OrderedNode {
            key= root.key, value= root.value, left= pivot.right, right= root.right, height= right_height,
        });
        let c = pivot.left.height();
        return OrderedTree.Branch(OrderedNode {
            key= pivot.key, value= pivot.value, left= pivot.left, right= right,
            height= 1 + if c > right_height { c } else { right_height },
        });
    }
}

public type OrderedMap<K, V> = {
    private tree: OrderedTree<K, V>,
    private compare: (K, K) -> i32 ![],
    private size: usize,
};

impl<K ~ Order, V> OrderedMap<K, V> {
    public flow new() -> OrderedMap<K, V> ![] {
        return OrderedMap { tree= OrderedTree.Empty, compare= (left: K, right: K) => left.compare(right), size= 0 };
    }
}

impl OrderedMap<K, V> {
    public flow new(compare: (K, K) -> i32 ![]) -> OrderedMap<K, V> ![] {
        return OrderedMap { tree= OrderedTree.Empty, compare= compare, size= 0 };
    }

    public flow get(self, key: K) -> Option<V> ![] {
        var remaining = self.tree;
        while true limit Iterations(18446744073709551615) {
            match remaining {
                OrderedTree.Empty => { return None; },
                OrderedTree.Branch(node) => {
                    let order = (self.compare)(key, node.key);
                    if order == 0 { return Some(node.value); }
                    remaining = if order < 0 { node.left } else { node.right };
                },
            };
        }
    }

    public flow contains_key(self, key: K) -> bool ![] {
        var remaining = self.tree;
        while true limit Iterations(18446744073709551615) {
            match remaining {
                OrderedTree.Empty => { return false; },
                OrderedTree.Branch(node) => {
                    let order = (self.compare)(key, node.key);
                    if order == 0 { return true; }
                    remaining = if order < 0 { node.left } else { node.right };
                },
            };
        }
    }

    public flow insert(self, key: K, value: V) -> OrderedMap<K, V> ![] {
        let (tree, added) = self.tree.insert(key, value, self.compare);
        return OrderedMap {
            tree= tree, compare= self.compare,
            size= self.size + if added { 1 } else { 0 },
        };
    }

    public flow len(self) -> usize ![] {
        return self.size;
    }

    public flow is_empty(self) -> bool ![] {
        return self.size == 0;
    }
}

public type OrderedSet<T> = {
    private tree: OrderedTree<T, unit>,
    private compare: (T, T) -> i32 ![],
    private size: usize,
};

impl<T ~ Order> OrderedSet<T> {
    public flow new() -> OrderedSet<T> ![] {
        return OrderedSet { tree= OrderedTree.Empty, compare= (left: T, right: T) => left.compare(right), size= 0 };
    }
}

impl OrderedSet<T> {
    public flow new(compare: (T, T) -> i32 ![]) -> OrderedSet<T> ![] {
        return OrderedSet { tree= OrderedTree.Empty, compare= compare, size= 0 };
    }

    public flow contains(self, value: T) -> bool ![] {
        var remaining = self.tree;
        while true limit Iterations(18446744073709551615) {
            match remaining {
                OrderedTree.Empty => { return false; },
                OrderedTree.Branch(node) => {
                    let order = (self.compare)(value, node.key);
                    if order == 0 { return true; }
                    remaining = if order < 0 { node.left } else { node.right };
                },
            };
        }
    }

    public flow insert(self, value: T) -> OrderedSet<T> ![] {
        let (tree, added) = self.tree.insert(value, (), self.compare);
        return OrderedSet {
            tree= tree, compare= self.compare,
            size= self.size + if added { 1 } else { 0 },
        };
    }

    public flow len(self) -> usize ![] {
        return self.size;
    }

    public flow is_empty(self) -> bool ![] {
        return self.size == 0;
    }
}

impl OrderedTree<K, V> {
    flow walk(self, pending: List<OrderedNode<K, V>>) -> Iter<(K, V)> ![] {
        return Iter { next= () => {
            var tree = self;
            var stack: List<OrderedNode<K, V>> = pending;
            while true limit Iterations(18446744073709551615) {
                match tree {
                    OrderedTree.Empty => { break; },
                    OrderedTree.Branch(node) => {
                        stack = node :: stack;
                        tree = node.left;
                    },
                };
            }
            return match stack {
                [] => None,
                node :: tail => Some(((node.key, node.value), node.right.walk(tail))),
            };
        } };
    }
}

impl OrderedMap<K, V> ~ Iterable<(K, V)> {
    public flow iter(self) -> Iter<(K, V)> ![] {
        return Iter { next= () => {
            var tree = self.tree;
            var stack: List<OrderedNode<K, V>> = [];
            while true limit Iterations(18446744073709551615) {
                match tree {
                    OrderedTree.Empty => { break; },
                    OrderedTree.Branch(node) => {
                        stack = node :: stack;
                        tree = node.left;
                    },
                };
            }
            return match stack {
                [] => None,
                node :: tail => Some(((node.key, node.value), node.right.walk(tail))),
            };
        } };
    }
}

impl OrderedSet<T> ~ Iterable<T> {
    public flow iter(self) -> Iter<T> ![] {
        return Iter { next= () => {
            var tree = self.tree;
            var stack: List<OrderedNode<T, unit>> = [];
            while true limit Iterations(18446744073709551615) {
                match tree {
                    OrderedTree.Empty => { break; },
                    OrderedTree.Branch(node) => {
                        stack = node :: stack;
                        tree = node.left;
                    },
                };
            }
            return match stack {
                [] => None,
                node :: tail => Some((
                    node.key,
                    node.right.walk(tail).map((entry: (T, unit)) => {
                        let (key, _) = entry;
                        return key;
                    }),
                )),
            };
        } };
    }
}
