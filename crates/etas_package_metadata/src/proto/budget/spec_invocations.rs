use super::*;
use crate::{CallableRequestTarget, SpecMethodGenericArgument};

pub(crate) fn validate(target: &CallableRequestTarget) -> Result<(), MetadataArtifactError> {
    ModelGraphValidator::default().callable_request_target(target, 0)
}

impl ModelGraphValidator {
    pub(super) fn callable_request_target(
        &mut self,
        target: &CallableRequestTarget,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "callable evidence")?;
        target.validate_shape().map_err(invalid)?;
        let CallableRequestTarget::SpecMethod(invocation) = target else {
            return Ok(());
        };
        self.budget
            .charge(invocation.spec.len(), "spec invocation path")?;
        self.ty(&invocation.receiver, depth + 1)?;
        for arg in &invocation.spec_args {
            self.ty(arg, depth + 1)?;
        }
        for argument in &invocation.generic_arguments {
            self.budget
                .enter(depth + 1, "spec invocation generic argument")?;
            match argument {
                SpecMethodGenericArgument::Type(ty) => self.ty(ty, depth + 2)?,
                SpecMethodGenericArgument::EffectRow(row) => self.effect_row(row, depth + 2)?,
            }
        }
        for argument in &invocation.callable_arguments {
            self.budget.enter(depth + 1, "spec invocation callback")?;
            self.callable_values_at_depth(&argument.sources, depth + 2)?;
        }
        Ok(())
    }
}

impl ProtoGraphValidator {
    pub(super) fn callable_request_target(
        &mut self,
        target: &proto::ProtoCallableRequestTarget,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        use proto::proto_callable_request_target::Target;
        use proto::proto_spec_method_generic_argument::Argument;
        self.budget.enter(depth, "callable evidence")?;
        match target
            .target
            .as_ref()
            .ok_or_else(|| invalid("callable evidence has no kind"))?
        {
            Target::Parameter(parameter) => {
                if parameter.is_empty() {
                    return Err(invalid("callable request parameter is empty"));
                }
            }
            Target::SpecMethod(invocation) => {
                self.budget
                    .charge(invocation.spec.len(), "spec invocation path")?;
                if let Some(ty) = &invocation.receiver {
                    self.ty(ty, depth + 1)?;
                }
                for ty in &invocation.spec_args {
                    self.ty(ty, depth + 1)?;
                }
                for argument in &invocation.generic_arguments {
                    self.budget
                        .enter(depth + 1, "spec invocation generic argument")?;
                    match argument
                        .argument
                        .as_ref()
                        .ok_or_else(|| invalid("spec invocation generic argument has no kind"))?
                    {
                        Argument::Type(ty) => self.ty(ty, depth + 2)?,
                        Argument::EffectRow(row) => self.effect_row(row, depth + 2)?,
                    }
                }
                for argument in &invocation.callable_arguments {
                    self.budget.enter(depth + 1, "spec invocation callback")?;
                    if let Some(graph) = &argument.sources {
                        self.callable_values_at_depth(graph, depth + 2)?;
                    }
                }
            }
        }
        Ok(())
    }
}
