use crate::{HostError, HostErrorCode};

pub fn assert_host_error_code(error: &HostError, code: HostErrorCode) {
    assert_eq!(error.code, code);
}
