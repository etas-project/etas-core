use std::sync::Arc;

use etas_code::{artifact::CheckedCodeRef, report::CheckReport};

use super::CompilerFailure;
use crate::{
    HostRequestId,
    execution::{ExternalOutcome, OperationResponse},
};

#[derive(Clone, Debug)]
pub struct CompilerResponse {
    pub id: HostRequestId,
    pub result: Result<CompilerResult, CompilerFailure>,
}

#[derive(Clone, Debug)]
pub enum CompilerResult {
    Checked(Arc<CheckReport>),
    Compiled(CheckedCodeRef),
}

impl OperationResponse for CompilerResponse {
    fn external_outcome(&self) -> ExternalOutcome {
        ExternalOutcome::Confirmed
    }
}
