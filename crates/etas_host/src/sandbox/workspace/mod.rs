mod path;
mod registry;
mod root;

use crate::{HostError, HostErrorCode};
use cap_std::fs::OpenOptions;
pub use path::{WorkspacePath, WorkspacePathRef, WorkspaceRegionId, normalize_relative};
pub use registry::WorkspaceRegionRegistry;
pub use root::WorkspaceRoot;

pub(crate) fn workspace_io_error(error: std::io::Error) -> HostError {
    let code = match error.kind() {
        std::io::ErrorKind::PermissionDenied => HostErrorCode::AuthorityDenied,
        std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidInput => {
            HostErrorCode::InvalidRequest
        }
        _ => HostErrorCode::ProviderUnavailable,
    };
    HostError::new(code, "workspace capability operation failed")
        .with_detail("error", error.to_string())
}

pub(super) fn read_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        // Opening a raced-in FIFO must not block before its file kind is checked.
        options.custom_flags(libc::O_NONBLOCK);
    }
    options
}

#[cfg(test)]
mod tests;
