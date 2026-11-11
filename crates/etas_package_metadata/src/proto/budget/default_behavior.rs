use super::*;

pub(crate) fn validate(value: &DefaultBehavior) -> Result<(), MetadataArtifactError> {
    ModelGraphValidator::default().default_behavior(value, 0)
}

impl ModelGraphValidator {
    pub(super) fn default_behavior(
        &mut self,
        value: &DefaultBehavior,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "default behavior")?;
        self.effect_footprint(&value.direct, depth + 1)?;
        self.budget
            .charge(value.guarded.len(), "default behavior contributions")?;
        for entry in &value.guarded {
            if entry.guards.is_empty() {
                return Err(invalid(
                    "default behavior contribution requires checked guards",
                ));
            }
            for guard in &entry.guards {
                self.effect_ref(guard, depth + 1)?;
            }
            self.effect_footprint(&entry.footprint, depth + 1)?;
        }
        Ok(())
    }

    fn effect_footprint(
        &mut self,
        value: &EffectFootprint,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "effect footprint")?;
        for row in [
            &value.escaping_effects,
            &value.requested_actions,
            &value.default_actions,
            &value.handled_actions,
        ] {
            self.effect_row(row, depth + 1)?;
        }
        if let Some(requests) = &value.handler_requests {
            self.request_template_at_depth(requests, depth + 1)?;
        }
        self.residual_references(&value.residual_checks)?;
        self.budget.charge(
            value.behavior.host_requirements.len()
                + value.behavior.interpreter_features.len()
                + value.requirements.limits.len(),
            "effect footprint requirements",
        )?;
        self.budget.charge(
            value.requirements.trace_requirements.len()
                + value.requirements.trace_obligations.len(),
            "effect footprint trace requirements",
        )?;
        for requirement in value
            .requirements
            .trace_requirements
            .iter()
            .chain(&value.requirements.trace_obligations)
        {
            if let MethodTraceRequirement::Clause(clause) = requirement {
                self.budget
                    .charge(clause.selector_bounds.len(), "trace selector bounds")?;
                for bound in &clause.selector_bounds {
                    validate_trace_selector_bound(clause, bound)?;
                    for arg in &bound.spec_args {
                        self.ty(arg, depth + 1)?;
                    }
                }
                for row in [
                    &clause.pattern,
                    &clause.guard,
                    &clause.target,
                    &clause.obligation,
                ]
                .into_iter()
                .flatten()
                {
                    self.effect_row(row, depth + 1)?;
                }
            }
        }
        Ok(())
    }
}

impl ProtoGraphValidator {
    pub(super) fn default_behavior(
        &mut self,
        value: &proto::ProtoDefaultBehavior,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "default behavior")?;
        if let Some(direct) = &value.direct {
            self.effect_footprint(direct, depth + 1)?;
        }
        self.budget
            .charge(value.guarded.len(), "default behavior contributions")?;
        for entry in &value.guarded {
            for guard in &entry.guards {
                self.effect_ref(guard, depth + 1)?;
            }
            if let Some(footprint) = &entry.footprint {
                self.effect_footprint(footprint, depth + 1)?;
            }
        }
        Ok(())
    }

    fn effect_footprint(
        &mut self,
        value: &proto::ProtoEffectFootprint,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "effect footprint")?;
        for row in [
            &value.escaping_effects,
            &value.requested_actions,
            &value.default_actions,
            &value.handled_actions,
        ]
        .into_iter()
        .flatten()
        {
            self.effect_row(row, depth + 1)?;
        }
        if let Some(requests) = &value.handler_requests {
            self.request_template_at_depth(requests, depth + 1)?;
        }
        if let Some(references) = &value.residual_checks {
            self.residual_references(references)?;
        }
        if let Some(behavior) = &value.behavior {
            self.budget.charge(
                behavior.host_requirements.len() + behavior.interpreter_features.len(),
                "effect footprint behavior",
            )?;
        }
        self.budget.charge(
            value.limits.len() + value.trace_requirements.len() + value.trace_obligations.len(),
            "effect footprint requirements",
        )?;
        for requirement in value
            .trace_requirements
            .iter()
            .chain(&value.trace_obligations)
        {
            if let Some(clause) = &requirement.clause {
                self.budget
                    .charge(clause.selector_bounds.len(), "trace selector bounds")?;
                for bound in &clause.selector_bounds {
                    for arg in &bound.spec_args {
                        self.ty(arg, depth + 1)?;
                    }
                }
                for row in [
                    &clause.pattern,
                    &clause.guard,
                    &clause.target,
                    &clause.obligation,
                ]
                .into_iter()
                .flatten()
                {
                    self.effect_row(row, depth + 1)?;
                }
            }
        }
        Ok(())
    }
}
