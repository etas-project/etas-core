use crate::{HostError, HostErrorCode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformSandbox {
    hooks: Vec<PlatformSandboxHook>,
}

impl PlatformSandbox {
    pub fn new(hooks: Vec<PlatformSandboxHook>) -> Self {
        Self { hooks }
    }

    pub fn require_hook(&self, hook: PlatformSandboxHook) -> Result<(), HostError> {
        if self.hooks.contains(&hook) {
            Ok(())
        } else {
            Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "required platform sandbox hook is not configured",
            )
            .with_detail("hook", hook.name()))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformSandboxHook {
    Landlock,
    Container,
    WasiPreopen,
}

impl PlatformSandboxHook {
    pub fn name(self) -> &'static str {
        match self {
            Self::Landlock => "landlock",
            Self::Container => "container",
            Self::WasiPreopen => "wasi-preopen",
        }
    }
}
