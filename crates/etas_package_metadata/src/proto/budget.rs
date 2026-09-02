use crate::{MetadataArtifactError, model::*};

use super::schema as proto;

pub(crate) const MAX_METADATA_GRAPH_DEPTH: usize = 64;
pub(crate) const MAX_METADATA_GRAPH_NODES: usize = 100_000;

#[derive(Default)]
pub(crate) struct MetadataGraphBudget {
    nodes: usize,
}

impl MetadataGraphBudget {
    pub(crate) fn enter(&mut self, depth: usize, graph: &str) -> Result<(), MetadataArtifactError> {
        if depth > MAX_METADATA_GRAPH_DEPTH {
            return Err(invalid(format!(
                "package metadata {graph} exceeds the maximum depth"
            )));
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| invalid(format!("package metadata {graph} node count overflow")))?;
        if self.nodes > MAX_METADATA_GRAPH_NODES {
            return Err(invalid(format!(
                "package metadata graph exceeds the maximum node count while validating {graph}"
            )));
        }
        Ok(())
    }
}

pub(crate) fn validate_package_metadata_graph(
    metadata: &PackageMetadata,
) -> Result<(), MetadataArtifactError> {
    let mut validator = ModelGraphValidator::default();
    validator.budget.enter(0, "package graph")?;
    validator.public_metadata(&metadata.public_metadata)?;
    for dependency in &metadata.dependencies {
        validator.dependency(dependency, 1)?;
    }
    Ok(())
}

pub(crate) fn validate_proto_package_metadata_graph(
    metadata: &proto::ProtoPackageGraphSection,
) -> Result<(), MetadataArtifactError> {
    let mut validator = ProtoGraphValidator::default();
    validator.budget.enter(0, "package graph")?;
    if let Some(public_metadata) = metadata.public_metadata.as_ref() {
        validator.public_metadata(public_metadata)?;
    }
    for dependency in &metadata.dependencies {
        validator.dependency(dependency, 1)?;
    }
    Ok(())
}

#[derive(Default)]
struct ModelGraphValidator {
    budget: MetadataGraphBudget,
}

impl ModelGraphValidator {
    fn dependency(
        &mut self,
        dependency: &ResolvedDependency,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "dependency graph")?;
        self.public_metadata(&dependency.public_metadata)?;
        for child in &dependency.dependencies {
            self.dependency(child, depth + 1)?;
        }
        Ok(())
    }

    fn public_metadata(&mut self, metadata: &PublicMetadata) -> Result<(), MetadataArtifactError> {
        for signature in metadata
            .types
            .iter()
            .chain(&metadata.values)
            .chain(&metadata.enums)
            .chain(&metadata.effects)
            .chain(&metadata.trace_specs)
            .chain(&metadata.protocols)
        {
            if let Some(ty) = &signature.ty {
                self.ty(ty, 0)?;
            }
        }
        for signature in metadata
            .flows
            .iter()
            .chain(&metadata.agents)
            .chain(&metadata.tools)
        {
            self.callable(signature)?;
        }
        for action in &metadata.actions {
            validate_model_action(action)?;
            for param in &action.generic_params {
                self.generic_param(param)?;
            }
            for ty in &action.params {
                self.ty(ty, 0)?;
            }
            if let Some(output) = &action.output {
                self.ty(output, 0)?;
            }
            for default in action.selector_defaults.iter().flatten() {
                self.effect_arg(default, 0)?;
            }
        }
        for signature in &metadata.spec_signatures {
            if let Some(callable) = &signature.callable {
                self.callable(callable)?;
            }
            for method in &signature.methods {
                if let Some(callable) = &method.signature {
                    self.callable(callable)?;
                }
            }
            for bound in &signature.super_specs {
                self.spec_bound(bound)?;
            }
        }
        for implementation in &metadata.spec_impls {
            self.ty(&implementation.self_type, 0)?;
            for arg in &implementation.args {
                self.ty(arg, 0)?;
            }
        }
        for satisfaction in &metadata.type_spec_satisfactions {
            self.ty(&satisfaction.self_type, 0)?;
            for arg in &satisfaction.args {
                self.ty(arg, 0)?;
            }
        }
        for satisfaction in &metadata.callable_spec_satisfactions {
            for arg in &satisfaction.args {
                self.ty(arg, 0)?;
            }
        }
        for conformance in &metadata.trace_spec_conformances {
            if let TraceSpecConformanceTarget::Named { args, .. } = &conformance.target {
                for arg in args {
                    self.ty(arg, 0)?;
                }
            }
        }
        for summary in &metadata.effect_summaries {
            self.effect_row(&summary.public_effects, 0)?;
            self.effect_row(&summary.requested_actions, 0)?;
            self.effect_row(&summary.handled_requested_actions, 0)?;
            for latent in &summary.latent_flows {
                self.effect_row(&latent.declared_bound, 0)?;
                self.effect_row(&latent.inferred_effects, 0)?;
            }
            self.action_trace(&summary.action_trace, 0)?;
        }
        for summary in &metadata.trace_spec_summaries {
            for clause in &summary.clauses {
                for row in [
                    clause.pattern.as_ref(),
                    clause.guard.as_ref(),
                    clause.target.as_ref(),
                    clause.obligation.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    self.effect_row(row, 0)?;
                }
            }
        }
        for annotation in &metadata.annotations {
            for arg in &annotation.args {
                self.annotation_value(&arg.value, 0)?;
            }
        }
        Ok(())
    }

    fn callable(&mut self, signature: &CallableSignature) -> Result<(), MetadataArtifactError> {
        for param in &signature.generic_params {
            self.generic_param(param)?;
        }
        for input in &signature.input {
            self.ty(input, 0)?;
        }
        if let Some(output) = &signature.output {
            self.ty(output, 0)?;
        }
        if let Some(effects) = &signature.effects {
            self.effect_row(effects, 0)?;
        }
        Ok(())
    }

    fn generic_param(&mut self, param: &GenericParam) -> Result<(), MetadataArtifactError> {
        for bound in &param.bounds {
            self.spec_bound(bound)?;
        }
        Ok(())
    }

    fn spec_bound(&mut self, bound: &SpecBound) -> Result<(), MetadataArtifactError> {
        for arg in &bound.args {
            self.ty(arg, 0)?;
        }
        Ok(())
    }

    fn ty(&mut self, ty: &Type, depth: usize) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "type graph")?;
        for child in &ty.children {
            self.ty(child, depth + 1)?;
        }
        for field in &ty.fields {
            self.ty(&field.ty, depth + 1)?;
        }
        if let Some(effects) = &ty.effects {
            self.effect_row(effects, depth + 1)?;
        }
        if let Some(effects) = &ty.produced_effects {
            self.effect_row(effects, depth + 1)?;
        }
        Ok(())
    }

    fn effect_row(&mut self, row: &EffectRow, depth: usize) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "effect graph")?;
        for effect in &row.effects {
            self.effect_ref(effect, depth + 1)?;
        }
        Ok(())
    }

    fn effect_ref(
        &mut self,
        effect: &EffectRef,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "effect graph")?;
        for arg in &effect.args {
            self.effect_arg(arg, depth + 1)?;
        }
        Ok(())
    }

    fn effect_arg(&mut self, arg: &EffectArg, depth: usize) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "effect graph")?;
        if let Some(ty) = &arg.ty {
            self.ty(ty, depth + 1)?;
        }
        Ok(())
    }

    fn action_trace(
        &mut self,
        trace: &ActionTrace,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "action trace")?;
        match trace {
            ActionTrace::Event(event) => self.effect_ref(&event.action, depth + 1)?,
            ActionTrace::Seq(children) | ActionTrace::Choice(children) => {
                for child in children {
                    self.action_trace(child, depth + 1)?;
                }
            }
            ActionTrace::Repeat(child) => self.action_trace(child, depth + 1)?,
            ActionTrace::UnknownOrder(actions) | ActionTrace::Widened { actions, .. } => {
                for action in actions {
                    self.effect_ref(action, depth + 1)?;
                }
            }
            ActionTrace::Empty | ActionTrace::ParameterCall { .. } => {}
        }
        Ok(())
    }

    fn annotation_value(
        &mut self,
        value: &AnnotationValueMetadata,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "annotation graph")?;
        for element in &value.elements {
            self.annotation_value(element, depth + 1)?;
        }
        for field in &value.fields {
            self.annotation_value(&field.value, depth + 1)?;
        }
        Ok(())
    }
}

#[derive(Default)]
struct ProtoGraphValidator {
    budget: MetadataGraphBudget,
}

impl ProtoGraphValidator {
    fn dependency(
        &mut self,
        dependency: &proto::ProtoResolvedDependency,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "dependency graph")?;
        if let Some(public_metadata) = dependency.public_metadata.as_ref() {
            self.public_metadata(public_metadata)?;
        }
        for child in &dependency.dependencies {
            self.dependency(child, depth + 1)?;
        }
        Ok(())
    }

    fn public_metadata(
        &mut self,
        metadata: &proto::ProtoPublicMetadataSection,
    ) -> Result<(), MetadataArtifactError> {
        for signature in metadata
            .types
            .iter()
            .chain(&metadata.values)
            .chain(&metadata.enums)
            .chain(&metadata.effects)
            .chain(&metadata.trace_specs)
            .chain(&metadata.protocols)
        {
            if let Some(ty) = signature.ty.as_ref() {
                self.ty(ty, 0)?;
            }
        }
        for signature in metadata
            .flows
            .iter()
            .chain(&metadata.agents)
            .chain(&metadata.tools)
        {
            self.callable(signature)?;
        }
        for action in &metadata.actions {
            for generic in &action.generic_params {
                self.generic_param(generic)?;
            }
            for ty in &action.params {
                self.ty(ty, 0)?;
            }
            if let Some(output) = action.output.as_ref() {
                self.ty(output, 0)?;
            }
            for default in &action.selector_defaults {
                if let Some(arg) = default.value.as_ref() {
                    self.effect_arg(arg, 0)?;
                }
            }
        }
        for signature in &metadata.spec_signatures {
            if let Some(callable) = signature.callable.as_ref() {
                self.callable(callable)?;
            }
            for method in &signature.methods {
                if let Some(callable) = method.signature.as_ref() {
                    self.callable(callable)?;
                }
            }
            for bound in &signature.super_specs {
                self.spec_bound(bound)?;
            }
        }
        for implementation in &metadata.spec_impls {
            if let Some(ty) = implementation.self_type.as_ref() {
                self.ty(ty, 0)?;
            }
            for arg in &implementation.args {
                self.ty(arg, 0)?;
            }
        }
        for satisfaction in &metadata.type_spec_satisfactions {
            if let Some(ty) = satisfaction.self_type.as_ref() {
                self.ty(ty, 0)?;
            }
            for arg in &satisfaction.args {
                self.ty(arg, 0)?;
            }
        }
        for satisfaction in &metadata.callable_spec_satisfactions {
            for arg in &satisfaction.args {
                self.ty(arg, 0)?;
            }
        }
        for conformance in &metadata.trace_spec_conformances {
            for arg in &conformance.args {
                self.ty(arg, 0)?;
            }
        }
        for summary in &metadata.effect_summaries {
            for row in [
                summary.public_effects.as_ref(),
                summary.requested_actions.as_ref(),
                summary.handled_requested_actions.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                self.effect_row(row, 0)?;
            }
            for latent in &summary.latent_flows {
                if let Some(row) = latent.declared_bound.as_ref() {
                    self.effect_row(row, 0)?;
                }
                if let Some(row) = latent.inferred_effects.as_ref() {
                    self.effect_row(row, 0)?;
                }
            }
            if let Some(trace) = summary.action_trace.as_ref() {
                self.action_trace(trace, 0)?;
            }
        }
        for summary in &metadata.trace_spec_summaries {
            for clause in &summary.clauses {
                for row in [
                    clause.pattern.as_ref(),
                    clause.guard.as_ref(),
                    clause.target.as_ref(),
                    clause.obligation.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    self.effect_row(row, 0)?;
                }
            }
        }
        for annotation in &metadata.annotations {
            for arg in &annotation.args {
                for value in &arg.value {
                    self.annotation_value(value, 0)?;
                }
            }
        }
        Ok(())
    }

    fn callable(
        &mut self,
        signature: &proto::ProtoCallableSignature,
    ) -> Result<(), MetadataArtifactError> {
        for generic in &signature.generic_params {
            self.generic_param(generic)?;
        }
        for input in &signature.input {
            self.ty(input, 0)?;
        }
        if let Some(output) = signature.output.as_ref() {
            self.ty(output, 0)?;
        }
        if let Some(effects) = signature.effects.as_ref() {
            self.effect_row(effects, 0)?;
        }
        Ok(())
    }

    fn generic_param(
        &mut self,
        generic: &proto::ProtoActionGenericParam,
    ) -> Result<(), MetadataArtifactError> {
        for bound in &generic.bounds {
            self.spec_bound(bound)?;
        }
        Ok(())
    }

    fn spec_bound(&mut self, bound: &proto::ProtoSpecBound) -> Result<(), MetadataArtifactError> {
        for arg in &bound.args {
            self.ty(arg, 0)?;
        }
        Ok(())
    }

    fn ty(&mut self, ty: &proto::ProtoType, depth: usize) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "type graph")?;
        for child in &ty.children {
            self.ty(child, depth + 1)?;
        }
        for field in &ty.fields {
            if let Some(field_ty) = field.ty.as_ref() {
                self.ty(field_ty, depth + 1)?;
            }
        }
        if let Some(row) = ty.effects.as_ref() {
            self.effect_row(row, depth + 1)?;
        }
        if let Some(row) = ty.produced_effects.as_ref() {
            self.effect_row(row, depth + 1)?;
        }
        Ok(())
    }

    fn effect_row(
        &mut self,
        row: &proto::ProtoEffectRow,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "effect graph")?;
        for effect in &row.effects {
            self.effect_ref(effect, depth + 1)?;
        }
        Ok(())
    }

    fn effect_ref(
        &mut self,
        effect: &proto::ProtoEffectRef,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "effect graph")?;
        for arg in &effect.args {
            self.effect_arg(arg, depth + 1)?;
        }
        Ok(())
    }

    fn effect_arg(
        &mut self,
        arg: &proto::ProtoEffectArg,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "effect graph")?;
        if let Some(ty) = arg.ty.as_ref() {
            self.ty(ty, depth + 1)?;
        }
        Ok(())
    }

    fn action_trace(
        &mut self,
        trace: &proto::ProtoActionTrace,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "action trace")?;
        if let Some(event) = trace.event.as_ref()
            && let Some(action) = event.action.as_ref()
        {
            self.effect_ref(action, depth + 1)?;
        }
        for child in &trace.children {
            self.action_trace(child, depth + 1)?;
        }
        for action in &trace.actions {
            self.effect_ref(action, depth + 1)?;
        }
        Ok(())
    }

    fn annotation_value(
        &mut self,
        value: &proto::ProtoAnnotationValueMetadata,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "annotation graph")?;
        for element in &value.elements {
            self.annotation_value(element, depth + 1)?;
        }
        for field in &value.fields {
            for field_value in &field.value {
                self.annotation_value(field_value, depth + 1)?;
            }
        }
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> MetadataArtifactError {
    MetadataArtifactError::invalid(crate::PACKAGE_METADATA_FILE, message)
}

fn validate_model_action(action: &ActionSignature) -> Result<(), MetadataArtifactError> {
    if action.path.is_empty() || action.path.iter().any(String::is_empty) {
        return Err(invalid("action signature path is required"));
    }
    if action.effect_args.len() != action.selector_param_names.len() {
        return Err(invalid(format!(
            "action signature `{}` selector_param_names length {} does not match effect_args length {}",
            action.path.join("."),
            action.selector_param_names.len(),
            action.effect_args.len()
        )));
    }
    if action.effect_args.len() != action.selector_defaults.len() {
        return Err(invalid(format!(
            "action signature `{}` selector_defaults length {} does not match effect_args length {}",
            action.path.join("."),
            action.selector_defaults.len(),
            action.effect_args.len()
        )));
    }
    let generic_names = action
        .generic_params
        .iter()
        .map(|param| param.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if generic_names.len() != action.generic_params.len()
        || generic_names.iter().any(|name| name.is_empty())
    {
        return Err(invalid(format!(
            "action signature `{}` contains empty or duplicate generic parameter names",
            action.path.join(".")
        )));
    }
    for (kind, name) in action.effect_args.iter().zip(&action.selector_param_names) {
        if name.is_empty()
            || matches!(kind, ActionArgKind::Type) && !generic_names.contains(name.as_str())
        {
            return Err(invalid(format!(
                "action signature `{}` selector `{name}` does not name a compatible generic parameter",
                action.path.join(".")
            )));
        }
    }
    for (index, (kind, default)) in action
        .effect_args
        .iter()
        .zip(&action.selector_defaults)
        .enumerate()
    {
        if default
            .as_ref()
            .is_some_and(|default| !model_effect_arg_matches_kind(default, kind))
        {
            return Err(invalid(format!(
                "action signature `{}` selector default at index {index} does not match selector kind",
                action.path.join(".")
            )));
        }
    }
    Ok(())
}

fn model_effect_arg_matches_kind(arg: &EffectArg, kind: &ActionArgKind) -> bool {
    if matches!(arg.kind, EffectArgKind::Wildcard) {
        return true;
    }
    match kind {
        ActionArgKind::Type => matches!(arg.kind, EffectArgKind::Type),
        ActionArgKind::MemoryPlace | ActionArgKind::StaticResourcePath { .. } => {
            matches!(arg.kind, EffectArgKind::Path)
        }
        ActionArgKind::StringPattern => matches!(
            arg.kind,
            EffectArgKind::String | EffectArgKind::Int | EffectArgKind::Path
        ),
    }
}
