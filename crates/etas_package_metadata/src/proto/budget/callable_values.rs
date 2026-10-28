use super::*;
use crate::CallableValueGraph;

pub(crate) fn validate(graph: &CallableValueGraph) -> Result<usize, MetadataArtifactError> {
    let mut validator = ModelGraphValidator::default();
    validator.callable_values(graph)?;
    Ok(validator.budget.used())
}

impl ModelGraphValidator {
    pub(super) fn callable_values(
        &mut self,
        graph: &CallableValueGraph,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(0, "callable value graph")?;
        self.budget
            .charge(graph.sources.len(), "callable return sources")?;
        for node in &graph.values {
            self.budget.enter(0, "callable value")?;
            if let Some(requests) = &node.handler_requests {
                self.request_template(requests)?;
            }
            self.budget.charge(
                node.behavior.host_requirements.len() + node.behavior.interpreter_features.len(),
                "callable behavior",
            )?;
            self.budget.charge(
                node.generic_bindings.len()
                    + node.parameter_bindings.len()
                    + node.param_names.len(),
                "callable bindings",
            )?;
            self.budget.charge(
                node.returned_sources.as_ref().map_or(0, Vec::len),
                "callable return sources",
            )?;
            for generic in &node.generic_params {
                self.generic_param(generic)?;
            }
            for ty in node.params.iter().chain(std::iter::once(&node.output)) {
                self.ty(ty, 0)?;
            }
            for row in [
                &node.public_effects,
                &node.requested_actions,
                &node.handled_requested_actions,
                &node.default_handler_effects.direct_effects,
                &node.default_handler_effects.default_actions,
            ] {
                self.effect_row(row, 0)?;
            }
            for handler in &node.default_handler_effects.handlers {
                self.effect_ref(&handler.action, 0)?;
                self.effect_row(&handler.produced, 0)?;
            }
            self.action_trace(&node.action_trace, 0)?;
        }
        graph.validate_references(None)
    }
}

impl ProtoGraphValidator {
    pub(super) fn callable_values(
        &mut self,
        graph: &proto::ProtoCallableValueGraph,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(0, "callable value graph")?;
        self.budget
            .charge(graph.sources.len(), "callable return sources")?;
        for node in &graph.values {
            self.budget.enter(0, "callable value")?;
            if let Some(requests) = &node.handler_requests {
                self.request_template(requests)?;
            }
            if let Some(behavior) = &node.behavior {
                self.budget.charge(
                    behavior.host_requirements.len() + behavior.interpreter_features.len(),
                    "callable behavior",
                )?;
            }
            self.budget.charge(
                node.generic_bindings.len()
                    + node.parameter_bindings.len()
                    + node.param_names.len(),
                "callable bindings",
            )?;
            self.budget.charge(
                node.returned_sources
                    .as_ref()
                    .map_or(0, |sources| sources.sources.len()),
                "callable return sources",
            )?;
            for generic in &node.generic_params {
                self.generic_param(generic)?;
            }
            for ty in node.params.iter().chain(node.output.iter()) {
                self.ty(ty, 0)?;
            }
            for row in [
                &node.public_effects,
                &node.requested_actions,
                &node.handled_requested_actions,
            ]
            .into_iter()
            .flatten()
            {
                self.effect_row(row, 0)?;
            }
            if let Some(defaults) = &node.default_handler_effects {
                for row in [&defaults.direct_effects, &defaults.default_actions]
                    .into_iter()
                    .flatten()
                {
                    self.effect_row(row, 0)?;
                }
                for handler in &defaults.handlers {
                    if let Some(action) = &handler.action {
                        self.effect_ref(action, 0)?;
                    }
                    if let Some(row) = &handler.produced {
                        self.effect_row(row, 0)?;
                    }
                }
            }
            if let Some(trace) = &node.action_trace {
                self.action_trace(trace, 0)?;
            }
        }
        Ok(())
    }
}
