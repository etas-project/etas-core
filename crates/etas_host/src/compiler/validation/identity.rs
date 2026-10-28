use etas_code::{
    descriptor::DescriptorLimits,
    report::{CheckReport, CheckStatus},
};

use crate::{
    HostError, HostErrorCode,
    compiler::{
        CompilerFailure, CompilerOperation, CompilerRequest, CompilerResponse, CompilerResult,
    },
    execution::OperationContext,
};

pub fn validate_compiler_request(
    request: &CompilerRequest,
    operation: &OperationContext,
) -> Result<(), HostError> {
    if operation.request() != Some(request.id) || operation.trace() != &request.trace {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "compiler request does not belong to its operation context",
        ));
    }
    operation.signal().check()?;
    request.budget.check_time()?;
    if !request.authority.allows(&request.action()) {
        return Err(HostError::new(
            HostErrorCode::AuthorityDenied,
            "compiler action is not authorized",
        ));
    }
    Ok(())
}

pub fn validate_compiler_response(
    request: &CompilerRequest,
    response: &CompilerResponse,
    limits: DescriptorLimits,
) -> Result<(), HostError> {
    let invalid = || {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "compiler response does not match the requested definition snapshot",
        )
    };
    if response.id != request.id {
        return Err(invalid());
    }
    match &response.result {
        Ok(CompilerResult::Checked(report)) if request.operation == CompilerOperation::Check => {
            validate_report(request, report, limits)
        }
        Ok(CompilerResult::Compiled(artifact))
            if request.operation == CompilerOperation::Compile =>
        {
            if artifact.identity() != request.definition.identity()
                || artifact.revision() != request.definition.revision()
            {
                return Err(invalid());
            }
            Ok(())
        }
        Err(CompilerFailure::Rejected(report))
            if request.operation == CompilerOperation::Compile
                && report.status == CheckStatus::Rejected =>
        {
            validate_report(request, report, limits)
        }
        Err(CompilerFailure::Service { diagnostics }) if !diagnostics.is_empty() => {
            for diagnostic in diagnostics {
                diagnostic.validate(limits).map_err(|error| {
                    HostError::new(HostErrorCode::InvalidResponse, error.to_string())
                })?;
            }
            Ok(())
        }
        _ => Err(invalid()),
    }
}

fn validate_report(
    request: &CompilerRequest,
    report: &CheckReport,
    limits: DescriptorLimits,
) -> Result<(), HostError> {
    if &report.revision != request.definition.revision()
        || report.dependencies.lexical_owner != *request.definition.identity()
        || report.dependencies.environment != request.definition.revision().inputs().environment
    {
        return Err(HostError::new(
            HostErrorCode::InvalidResponse,
            "compiler report belongs to a different frozen definition",
        ));
    }
    report
        .validate(limits)
        .map_err(|error| HostError::new(HostErrorCode::InvalidResponse, error.to_string()))
}
