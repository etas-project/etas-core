use super::*;

impl ModelGraphValidator {
    pub(super) fn request_template(
        &mut self,
        template: &RequestInstantiationTemplate,
    ) -> Result<(), MetadataArtifactError> {
        self.request_template_at_depth(template, 0)
    }

    pub(super) fn request_template_at_depth(
        &mut self,
        template: &RequestInstantiationTemplate,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "request instantiation template")?;
        self.budget.charge(
            template.actions.len() + template.parameters.len(),
            "request instantiation template",
        )?;
        template.validate_shape()?;
        for action in &template.actions {
            self.effect_ref(&action.action, depth + 1)?;
            if let Some(bindings) = &action.type_bindings {
                self.budget
                    .charge(bindings.len(), "request type bindings")?;
                for binding in bindings {
                    self.ty(&binding.ty, depth + 1)?;
                }
            }
        }
        for parameter in &template.parameters {
            self.callable_request_target(&parameter.parameter, depth + 1)?;
            for effect in &parameter.excluded {
                self.effect_ref(effect, depth + 1)?;
            }
        }
        Ok(())
    }
}

impl ProtoGraphValidator {
    pub(super) fn request_template(
        &mut self,
        template: &proto::ProtoRequestInstantiationTemplate,
    ) -> Result<(), MetadataArtifactError> {
        self.request_template_at_depth(template, 0)
    }

    pub(super) fn request_template_at_depth(
        &mut self,
        template: &proto::ProtoRequestInstantiationTemplate,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "request instantiation template")?;
        self.budget.charge(
            template.actions.len() + template.parameters.len(),
            "request instantiation template",
        )?;
        for action in &template.actions {
            if let Some(action) = &action.action {
                self.effect_ref(action, depth + 1)?;
            }
            if let Some(bindings) = &action.type_bindings {
                self.budget
                    .charge(bindings.bindings.len(), "request type bindings")?;
                for binding in &bindings.bindings {
                    if let Some(ty) = &binding.ty {
                        self.ty(ty, depth + 1)?;
                    }
                }
            }
        }
        for parameter in &template.parameters {
            if let Some(target) = &parameter.parameter {
                self.callable_request_target(target, depth + 1)?;
            }
            for effect in &parameter.excluded {
                self.effect_ref(effect, depth + 1)?;
            }
        }
        Ok(())
    }
}
