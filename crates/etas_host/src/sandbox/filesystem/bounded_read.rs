use std::io::{ErrorKind, Read};

use super::{BoundedFileRead, HostError, HostErrorCode, workspace_io_error};

pub(super) fn read(reader: &mut impl Read, max_bytes: usize) -> Result<BoundedFileRead, HostError> {
    read_controlled(reader, max_bytes, &mut || Ok(()))
}

pub(super) fn read_controlled(
    reader: &mut impl Read,
    max_bytes: usize,
    check: &mut impl FnMut() -> Result<(), HostError>,
) -> Result<BoundedFileRead, HostError> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let available = max_bytes - bytes.len();
        if available == 0 {
            let mut probe = [0];
            return if read_chunk(reader, &mut probe, check)? == 0 {
                Ok(BoundedFileRead::Complete(bytes))
            } else {
                Ok(BoundedFileRead::LimitExceeded { max_bytes })
            };
        }
        let requested = available.min(buffer.len());
        let count = read_chunk(reader, &mut buffer[..requested], check)?;
        if count == 0 {
            return Ok(BoundedFileRead::Complete(bytes));
        }
        let required = bytes.len() + count;
        if bytes.capacity() < required {
            let capacity = bytes
                .capacity()
                .saturating_mul(2)
                .max(required)
                .min(max_bytes);
            bytes
                .try_reserve_exact(capacity - bytes.len())
                .map_err(|_| {
                    HostError::new(
                        HostErrorCode::ProviderUnavailable,
                        "bounded file read allocation failed",
                    )
                })?;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}

fn read_chunk(
    reader: &mut impl Read,
    buffer: &mut [u8],
    check: &mut impl FnMut() -> Result<(), HostError>,
) -> Result<usize, HostError> {
    loop {
        check()?;
        match reader.read(buffer) {
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            result => return result.map_err(workspace_io_error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct GrowingReader {
        read: usize,
        largest_request: usize,
    }

    impl Read for GrowingReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            self.read += buffer.len();
            self.largest_request = self.largest_request.max(buffer.len());
            buffer.fill(7);
            Ok(buffer.len())
        }
    }

    #[test]
    fn continuously_growing_input_consumes_only_limit_plus_one() {
        for limit in [0, 1, 2048, 8192, 30_000] {
            let mut source = GrowingReader {
                read: 0,
                largest_request: 0,
            };
            assert_eq!(
                read(&mut source, limit).unwrap(),
                BoundedFileRead::LimitExceeded { max_bytes: limit }
            );
            assert_eq!(source.read, limit + 1);
            assert!(source.largest_request <= 8192);
        }
    }

    #[test]
    fn exact_limit_empty_input_and_usize_max_do_not_truncate_or_overflow() {
        for (data, limit) in [(&b""[..], 0), (&b"abc"[..], 3), (&b"abc"[..], usize::MAX)] {
            let mut input = data;
            assert_eq!(
                read(&mut input, limit).unwrap(),
                BoundedFileRead::Complete(data.to_vec())
            );
        }
        assert_eq!(
            read(&mut &b"abcd"[..], 3).unwrap(),
            BoundedFileRead::LimitExceeded { max_bytes: 3 }
        );
    }

    #[test]
    fn interrupted_read_is_retried_but_io_failure_never_returns_a_prefix() {
        struct FailingReader {
            step: usize,
        }
        impl Read for FailingReader {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                self.step += 1;
                match self.step {
                    1 => Err(std::io::Error::from(ErrorKind::Interrupted)),
                    2 => {
                        buffer[0] = 7;
                        Ok(1)
                    }
                    _ => Err(std::io::Error::from(ErrorKind::PermissionDenied)),
                }
            }
        }
        assert!(read(&mut FailingReader { step: 0 }, 2048).is_err());
    }

    #[test]
    fn cancellation_between_chunks_discards_the_partial_result() {
        let mut input = GrowingReader {
            read: 0,
            largest_request: 0,
        };
        let mut checkpoints = 0;
        let error = read_controlled(&mut input, 30_000, &mut || {
            checkpoints += 1;
            if checkpoints == 1 {
                Ok(())
            } else {
                Err(HostError::new(
                    HostErrorCode::Cancelled,
                    "cancelled test read",
                ))
            }
        })
        .unwrap_err();
        assert_eq!(error.code, HostErrorCode::Cancelled);
        assert_eq!(input.read, 8192);
        assert_eq!(checkpoints, 2);
    }

    #[test]
    fn interrupted_input_still_polls_the_operation_checkpoint() {
        struct Interrupted;
        impl Read for Interrupted {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(ErrorKind::Interrupted))
            }
        }
        let mut checkpoints = 0;
        let error = read_controlled(&mut Interrupted, 2048, &mut || {
            checkpoints += 1;
            if checkpoints < 3 {
                Ok(())
            } else {
                Err(HostError::new(
                    HostErrorCode::TimedOut,
                    "test read deadline",
                ))
            }
        })
        .unwrap_err();
        assert_eq!(error.code, HostErrorCode::TimedOut);
        assert_eq!(checkpoints, 3);
    }
}
