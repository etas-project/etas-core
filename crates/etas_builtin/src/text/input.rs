/// An immutable UTF-8 view whose chunks need not have a contiguous backing.
/// Chunks are in source order and their byte lengths sum to `byte_len`.
pub trait TextInput {
    type Chunks<'a>: DoubleEndedIterator<Item = &'a str>
    where
        Self: 'a;

    fn byte_len(&self) -> usize;
    /// Return an existing contiguous view without materializing one.
    fn contiguous(&self) -> Option<&str>;
    fn chunks(&self) -> Self::Chunks<'_>;
}

impl TextInput for str {
    type Chunks<'a> = std::iter::Once<&'a str>;

    fn byte_len(&self) -> usize {
        self.len()
    }

    fn contiguous(&self) -> Option<&str> {
        Some(self)
    }

    fn chunks(&self) -> Self::Chunks<'_> {
        std::iter::once(self)
    }
}
