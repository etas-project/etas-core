mod activation;
#[cfg(all(
    target_os = "linux",
    any(target_arch = "aarch64", target_arch = "x86_64")
))]
mod linux;
mod prepared;
mod requirements;

pub use activation::CommandIsolationReport;
pub(crate) use prepared::PreparedIsolation;
pub use requirements::{CommandIsolation, IsolationRequirements, PlatformSandboxHook};
