use super::*;

impl ModelGraphValidator {
    pub(super) fn request_template(
        &mut self,
        template: &RequestInstantiationTemplate,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(0, "request instantiation template")?;
        self.budget.charge(
            template.actions.len() + template.parameters.len(),
            "request instantiation template",
        )?;
        template.validate_shape()?;
        for action in &template.actions {
            self.effect_ref(&action.action, 0)?;
            if let Some(bindings) = &action.type_bindings {
                self.budget
                    .charge(bindings.len(), "request type bindings")?;
                for binding in bindings {
                    self.ty(&binding.ty, 0)?;
                }
            }
        }
        for parameter in &template.parameters {
            for effect in &parameter.excluded {
                self.effect_ref(effect, 0)?;
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
        self.budget.enter(0, "request instantiation template")?;
        self.budget.charge(
            template.actions.len() + template.parameters.len(),
            "request instantiation template",
        )?;
        for action in &template.actions {
            if let Some(action) = &action.action {
                self.effect_ref(action, 0)?;
            }
            if let Some(bindings) = &action.type_bindings {
                self.budget
                    .charge(bindings.bindings.len(), "request type bindings")?;
                for binding in &bindings.bindings {
                    if let Some(ty) = &binding.ty {
                        self.ty(ty, 0)?;
                    }
                }
            }
        }
        for parameter in &template.parameters {
            for effect in &parameter.excluded {
                self.effect_ref(effect, 0)?;
            }
        }
        Ok(())
    }
}
