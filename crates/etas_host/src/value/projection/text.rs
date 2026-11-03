use crate::{HostError, HostErrorCode};

pub(crate) struct TextLength {
    remaining: usize,
}

impl TextLength {
    pub(crate) fn new(bytes: usize) -> Result<Self, HostError> {
        if bytes > isize::MAX as usize {
            return Err(length_error());
        }
        Ok(Self { remaining: bytes })
    }

    pub(crate) fn consume(&mut self, chunk: &str) -> Result<(), HostError> {
        self.remaining = self
            .remaining
            .checked_sub(chunk.len())
            .ok_or_else(length_error)?;
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<(), HostError> {
        if self.remaining == 0 {
            Ok(())
        } else {
            Err(length_error())
        }
    }
}

pub(super) fn materialize<'a>(
    bytes: usize,
    chunks: impl IntoIterator<Item = &'a str>,
) -> Result<String, HostError> {
    let mut length = TextLength::new(bytes)?;
    let mut output = String::new();
    output.try_reserve_exact(bytes).map_err(|_| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "cannot allocate the owned host text result",
        )
    })?;
    for chunk in chunks {
        length.consume(chunk)?;
        output.push_str(chunk);
    }
    length.finish()?;
    Ok(output)
}

fn length_error() -> HostError {
    HostError::new(
        HostErrorCode::SchemaMismatch,
        "host text projection byte length does not match its UTF-8 chunks",
    )
}

#[cfg(test)]
mod tests;
