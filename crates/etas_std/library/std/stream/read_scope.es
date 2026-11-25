module std.stream;

// The private owner retains native admission; a stream passed to an action only
// borrows it. Pure source handlers need no native owner.
type ReadOwner;
type ReadScope = {};

impl ReadScope {
    @std.role("std.stream.ReadScope.enter")
    flow enter<S ~ ByteStream>(stream: S) -> (S, Option<ReadOwner>) ![];
}
