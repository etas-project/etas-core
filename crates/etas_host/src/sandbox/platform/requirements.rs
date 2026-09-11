#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IsolationRequirements {
    pub filesystem: bool,
    pub network: bool,
    pub process: bool,
}
impl IsolationRequirements {
    pub const fn all() -> Self {
        Self {
            filesystem: true,
            network: true,
            process: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandIsolation {
    Denied,
    TrustedUnconfined,
    Required {
        backend: PlatformSandboxHook,
        guarantees: IsolationRequirements,
    },
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
