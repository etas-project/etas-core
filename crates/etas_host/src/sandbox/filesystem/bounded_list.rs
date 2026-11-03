use super::workspace_io_error;
use crate::{BoundedDirectoryList, HostError, HostErrorCode};
use std::{ffi::OsString, io};

pub(super) fn collect(
    entries: impl IntoIterator<Item = io::Result<OsString>>,
    prefix_bytes: usize,
    max_entries: usize,
    max_bytes: usize,
    check: &mut impl FnMut() -> Result<(), HostError>,
) -> Result<BoundedDirectoryList<String>, HostError> {
    let mut names = Vec::new();
    let mut bytes = 0usize;
    check()?;
    for entry in entries {
        check()?;
        let name = entry.map_err(workspace_io_error)?;
        let name = name.to_str().ok_or_else(|| {
            HostError::new(
                HostErrorCode::IoFailure,
                "workspace directory entry name is not valid UTF-8",
            )
        })?;
        let next_bytes = bytes
            .checked_add(prefix_bytes)
            .and_then(|bytes| bytes.checked_add(name.len()));
        if names.len() >= max_entries || next_bytes.is_none_or(|bytes| bytes > max_bytes) {
            return Ok(BoundedDirectoryList::LimitExceeded {
                max_entries,
                max_bytes,
            });
        }
        bytes = next_bytes.ok_or_else(|| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "directory byte accounting overflow",
            )
        })?;
        names.push(name.to_owned());
    }
    check()?;
    names.sort();
    Ok(BoundedDirectoryList::Complete(names))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn non_utf8_names_are_errors_not_skipped_or_rewritten() {
        use std::os::unix::ffi::OsStringExt;
        let error = collect(
            [Ok(OsString::from_vec(vec![0xff]))],
            0,
            10,
            100,
            &mut || Ok(()),
        )
        .unwrap_err();
        assert_eq!(error.code, HostErrorCode::IoFailure);
    }
    #[test]
    fn bounds_are_checked_before_collecting_the_rest_of_the_directory() {
        let mut visited = 0;
        let entries = (0..1000).map(|_| {
            visited += 1;
            Ok(OsString::from("a"))
        });
        assert!(matches!(
            collect(entries, 0, 2, 99, &mut || Ok(())).unwrap(),
            BoundedDirectoryList::LimitExceeded { .. }
        ));
        assert_eq!(visited, 3);
    }
    #[test]
    fn exact_bounds_zero_limits_prefix_utf8_bytes_and_overflow() {
        let entries = || [Ok(OsString::from("z")), Ok(OsString::from("é"))];
        assert_eq!(
            collect(entries(), 4, 2, 11, &mut || Ok(())).unwrap(),
            BoundedDirectoryList::Complete(vec!["z".into(), "é".into()])
        );
        for (count, bytes) in [(1, 11), (2, 10), (0, 99), (99, 0)] {
            assert!(matches!(
                collect(entries(), 4, count, bytes, &mut || Ok(())).unwrap(),
                BoundedDirectoryList::LimitExceeded { .. }
            ));
        }
        assert_eq!(
            collect([], 0, 0, 0, &mut || Ok(())).unwrap(),
            BoundedDirectoryList::Complete(vec![])
        );
        assert!(matches!(
            collect(entries(), usize::MAX, 2, usize::MAX, &mut || Ok(())).unwrap(),
            BoundedDirectoryList::LimitExceeded { .. }
        ));
    }
    #[test]
    fn cancellation_is_checked_during_iteration() {
        let mut checks = 0;
        let result = collect(
            (0..100).map(|_| Ok(OsString::from("a"))),
            0,
            100,
            100,
            &mut || {
                checks += 1;
                if checks == 3 {
                    Err(HostError::new(HostErrorCode::Cancelled, "cancelled"))
                } else {
                    Ok(())
                }
            },
        );
        assert!(result.is_err());
        assert_eq!(checks, 3);
    }
}
