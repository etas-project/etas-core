module std.collections;

public enum CollectionError {
    InvalidChunkSize(usize),
    InvalidWindowSize(usize),
    InvalidStep(usize),
    InvalidCapacity(usize),
    Full,
    IncompatibleBounds,
    IncompatibleComparator,
}
