use crate::{HostError, HostErrorCode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandPolicy {
    pub allowed_programs: Vec<String>,
    pub isolation: super::platform::CommandIsolation,
}

impl CommandPolicy {
    pub fn deny_all() -> Self {
        Self {
            allowed_programs: Vec::new(),
            isolation: super::platform::CommandIsolation::Denied,
        }
    }

    pub fn allow_trusted_programs(allowed_programs: Vec<String>) -> Self {
        Self {
            allowed_programs,
            isolation: super::platform::CommandIsolation::TrustedUnconfined,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSandbox {
    policy: CommandPolicy,
}

impl CommandSandbox {
    pub fn new(policy: CommandPolicy) -> Self {
        Self { policy }
    }

    pub fn check_program(&self, program: &str) -> Result<(), HostError> {
        if self
            .policy
            .allowed_programs
            .iter()
            .any(|allowed| allowed == program)
        {
            Ok(())
        } else {
            Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "command execution is not allowlisted",
            )
            .with_detail("program", program))
        }
    }
}
