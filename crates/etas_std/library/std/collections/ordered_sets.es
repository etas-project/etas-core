module std.collections;

impl OrderedSet<T> {
    flow merged(self, other: OrderedSet<T>, operation: BitOperation) -> OrderedSet<T> ![Error<CollectionError>] {
        if !self.profile.matches(other.profile) { perform Error.raise<CollectionError>(CollectionError.IncompatibleComparator); }
        var left = (self.iter().next)();
        var right = (other.iter().next)();
        var output: Array<(T, unit)> = [];
        let keep_left = operation != BitOperation.Intersection;
        let keep_right = operation == BitOperation.Union || operation == BitOperation.SymmetricDifference;
        let keep_equal = operation == BitOperation.Union || operation == BitOperation.Intersection;
        while true {
            match (left, right) {
                (None, None) => { break; },
                (Some((a, tail)), None) => {
                    if keep_left { output = output.push((a, ())); }
                    left = (tail.next)();
                },
                (None, Some((b, tail))) => {
                    if keep_right { output = output.push((b, ())); }
                    right = (tail.next)();
                },
                (Some((a, a_tail)), Some((b, b_tail))) => {
                    let order = (self.compare)(a, b);
                    if order == 0 {
                        if keep_equal { output = output.push((a, ())); }
                        left = (a_tail.next)(); right = (b_tail.next)();
                    } else if order < 0 {
                        if keep_left { output = output.push((a, ())); }
                        left = (a_tail.next)();
                    } else {
                        if keep_right { output = output.push((b, ())); }
                        right = (b_tail.next)();
                    }
                },
            };
        }
        return OrderedSet { tree= OrderedTree.from_sorted(output, 0, output.length), compare= self.compare, profile= self.profile };
    }
    public flow union(self, other: OrderedSet<T>) -> OrderedSet<T> ![Error<CollectionError>] {
        return self.merged(other, BitOperation.Union);
    }
    public flow intersection(self, other: OrderedSet<T>) -> OrderedSet<T> ![Error<CollectionError>] {
        return self.merged(other, BitOperation.Intersection);
    }
    public flow difference(self, other: OrderedSet<T>) -> OrderedSet<T> ![Error<CollectionError>] {
        return self.merged(other, BitOperation.Difference);
    }
    public flow symmetric_difference(self, other: OrderedSet<T>) -> OrderedSet<T> ![Error<CollectionError>] {
        return self.merged(other, BitOperation.SymmetricDifference);
    }
    public flow is_subset(self, other: OrderedSet<T>) -> bool ![Error<CollectionError>] {
        if !self.profile.matches(other.profile) { perform Error.raise<CollectionError>(CollectionError.IncompatibleComparator); }
        var left = (self.iter().next)();
        var right = (other.iter().next)();
        while true {
            match (left, right) {
                (None, _) => { return true; },
                (Some(_), None) => { return false; },
                (Some((a, a_tail)), Some((b, b_tail))) => {
                    let order = (self.compare)(a, b);
                    if order < 0 { return false; }
                    if order == 0 { left = (a_tail.next)(); }
                    right = (b_tail.next)();
                },
            };
        }
    }
    public flow is_disjoint(self, other: OrderedSet<T>) -> bool ![Error<CollectionError>] {
        if !self.profile.matches(other.profile) { perform Error.raise<CollectionError>(CollectionError.IncompatibleComparator); }
        var left = (self.iter().next)();
        var right = (other.iter().next)();
        while true {
            match (left, right) {
                (None, _) => { return true; },
                (_, None) => { return true; },
                (Some((a, a_tail)), Some((b, b_tail))) => {
                    let order = (self.compare)(a, b);
                    if order == 0 { return false; }
                    if order < 0 { left = (a_tail.next)(); }
                    else { right = (b_tail.next)(); }
                },
            };
        }
    }
}
