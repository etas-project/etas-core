use crate::{MetadataArtifactError, model::*};

use super::schema as proto;

mod callable_values;
mod request_templates;
mod spec_invocations;
pub(crate) use callable_values::validate as validate_callable_values;
pub(crate) use spec_invocations::validate as validate_callable_request_target;

pub const MAX_METADATA_GRAPH_DEPTH: usize = 64;
pub const MAX_METADATA_GRAPH_NODES: usize = 100_000;

#[derive(Default)]
pub(crate) struct MetadataGraphBudget {
    nodes: usize,
}

impl MetadataGraphBudget {
    pub(crate) fn check(&self, count: usize, graph: &str) -> Result<(), MetadataArtifactError> {
        if self
            .nodes
            .checked_add(count)
            .is_none_or(|nodes| nodes > MAX_METADATA_GRAPH_NODES)
        {
            return Err(invalid(format!(
                "package metadata graph exceeds the maximum node count while validating {graph}"
            )));
        }
        Ok(())
    }
    pub(crate) fn used(&self) -> usize {
        self.nodes
    }
    pub(crate) fn charge(
        &mut self,
        count: usize,
        graph: &str,
    ) -> Result<(), MetadataArtifactError> {
        self.nodes = self
            .nodes
            .checked_add(count)
            .ok_or_else(|| invalid(format!("package metadata {graph} node count overflow")))?;
        if self.nodes > MAX_METADATA_GRAPH_NODES {
            return Err(invalid(format!(
                "package metadata graph exceeds the maximum node count while validating {graph}"
            )));
        }
        Ok(())
    }
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
    // Graph-local validation has no declaring interface; package/import gates do.
    parameter_bindings: Option<crate::MetadataTypeParameters>,
}

pub(crate) fn validate_method_type_parameters(
    method: &PublishedMethod,
    parameters: crate::MetadataTypeParameters,
) -> Result<(), MetadataArtifactError> {
    ModelGraphValidator {
        parameter_bindings: Some(parameters),
        ..Default::default()
    }
    .method(method)
}

impl ModelGraphValidator {
    fn trace_spec_definition(
        &mut self,
        definition: &etas_code::DefinitionKey,
    ) -> Result<(), MetadataArtifactError> {
        if definition.kind() != etas_code::DefinitionKind::Spec {
            return Err(invalid("trace spec definition identity is not a spec"));
        }
        self.budget.charge(
            1 + definition.module().len() + definition.declaration().len(),
            "trace spec definition identity",
        )
    }

    fn callable_contract(
        &mut self,
        target: &crate::CallableContractTarget,
        methods: &std::collections::BTreeMap<&etas_code::MethodId, &PublishedMethod>,
    ) -> Result<(), MetadataArtifactError> {
        match target {
            crate::CallableContractTarget::Item(path) => {
                self.budget.charge(path.len(), "callable contract path")?;
                if path.is_empty() || path.iter().any(String::is_empty) {
                    return Err(invalid("callable contract has an empty declaration path"));
                }
            }
            crate::CallableContractTarget::Method(identity) => {
                super::descriptors::validate_model_method_identity(identity, &mut self.budget)?;
                if !methods.contains_key(identity) {
                    return Err(invalid(
                        "callable contract method requires one published implementation owner",
                    ));
                }
            }
        }
        Ok(())
    }

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
        if let Some(graph) = &metadata.definition_descriptors {
            super::descriptors::validate_model(graph, &mut self.budget)?;
            super::descriptors::validate_public_bodies(metadata)?;
        }
        self.parameter_bindings = Some(crate::MetadataTypeParameters::from_checked_descriptors(
            metadata.definition_descriptors.as_ref(),
        ));
        if let Some(origins) = &metadata.definition_origins {
            self.budget.enter(0, "definition origins")?;
            for module in &origins.modules {
                self.budget.enter(0, "definition module")?;
                for _ in module.path.iter().chain(&module.canonical_path) {
                    self.budget.enter(0, "definition module path")?;
                }
            }
            for binding in &origins.declarations {
                self.budget.enter(0, "definition binding")?;
                self.budget.enter(0, "definition identity")?;
                for _ in binding
                    .module
                    .iter()
                    .chain(&binding.declaration)
                    .chain(binding.definition.module())
                    .chain(binding.definition.declaration())
                {
                    self.budget.enter(0, "definition binding path")?;
                }
            }
            origins
                .validate()
                .map_err(|error| invalid(error.to_string()))?;
        }
        for signature in metadata
            .types
            .iter()
            .chain(&metadata.values)
            .chain(&metadata.enums)
            .chain(&metadata.effects)
            .chain(&metadata.trace_specs)
            .chain(&metadata.protocols)
        {
            for generic in &signature.generic_params {
                self.generic_param(generic)?;
            }
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
            if signature.param_names.len() != signature.generic_params.len()
                || signature
                    .param_names
                    .iter()
                    .zip(&signature.generic_params)
                    .any(|(name, param)| name != &param.name)
            {
                return Err(invalid(
                    "spec generic parameter facts are incomplete or inconsistent",
                ));
            }
            for param in &signature.generic_params {
                self.generic_param(param)?;
            }
            if let Some(callable) = &signature.callable {
                self.callable(callable)?;
            }
            for method in &signature.methods {
                crate::spec_receivers::validate(method).map_err(invalid)?;
                if let Some(callable) = &method.signature {
                    self.callable(callable)?;
                }
            }
            for bound in &signature.super_specs {
                self.spec_bound(bound)?;
            }
        }
        let mut methods = std::collections::BTreeMap::new();
        for method in &metadata.methods {
            if methods.insert(&method.identity, method).is_some() {
                return Err(invalid("published method identity is duplicated"));
            }
            self.method(method)?;
        }
        for implementation in &metadata.spec_impls {
            self.ty(&implementation.self_type, 0)?;
            for arg in &implementation.args {
                self.ty(arg, 0)?;
            }
            for method in &implementation.methods {
                super::descriptors::validate_model_method_identity(method, &mut self.budget)?;
                let published = methods.get(method).ok_or_else(|| {
                    invalid("spec implementation references an unpublished method")
                })?;
                if published.self_type != implementation.self_type {
                    return Err(invalid(
                        "spec implementation method has a different owner type",
                    ));
                }
            }
        }
        for satisfaction in &metadata.type_spec_satisfactions {
            self.ty(&satisfaction.self_type, 0)?;
            for arg in &satisfaction.args {
                self.ty(arg, 0)?;
            }
        }
        for satisfaction in &metadata.callable_spec_satisfactions {
            self.callable_contract(&satisfaction.item, &methods)?;
            for arg in &satisfaction.args {
                self.ty(arg, 0)?;
            }
        }
        for conformance in &metadata.trace_spec_conformances {
            self.callable_contract(&conformance.item, &methods)?;
            if let TraceSpecConformanceTarget::Named {
                args, definition, ..
            } = &conformance.target
            {
                self.trace_spec_definition(definition)?;
                for arg in args {
                    self.ty(arg, 0)?;
                }
            }
        }
        for summary in &metadata.effect_summaries {
            self.residual_references(&summary.residual_checks)?;
            if let Some(requests) = &summary.handler_requests {
                self.request_template(requests)?;
            }
            if let Some(graph) = &summary.returned_callables {
                self.callable_values(graph)?;
            }
            if let Some(effects) = &summary.default_handler_effects {
                self.effect_row(&effects.direct_effects, 0)?;
                self.effect_row(&effects.default_actions, 0)?;
                for handler in &effects.handlers {
                    self.effect_ref(&handler.action, 0)?;
                    self.effect_row(&handler.produced, 0)?;
                }
            }
            self.effect_row(&summary.public_effects, 0)?;
            self.effect_row(&summary.requested_actions, 0)?;
            self.effect_row(&summary.handled_requested_actions, 0)?;
            for latent in &summary.latent_flows {
                self.residual_references(&latent.residual_checks)?;
                self.effect_row(&latent.declared_bound, 0)?;
                self.effect_row(&latent.inferred_effects, 0)?;
            }
            self.action_trace(&summary.action_trace, 0)?;
        }
        for summary in &metadata.trace_spec_summaries {
            self.trace_spec_definition(&summary.definition)?;
            self.budget
                .charge(summary.alternatives.len(), "trace spec alternatives")?;
            if summary.alternatives.is_empty() {
                return Err(invalid("trace spec requires an alternative"));
            }
            for alternative in &summary.alternatives {
                self.budget
                    .charge(alternative.len(), "trace spec clauses")?;
                if alternative.is_empty() {
                    return Err(invalid("trace spec alternative requires a clause"));
                }
                for clause in alternative {
                    self.budget
                        .charge(clause.selector_bounds.len(), "trace selector bounds")?;
                    for bound in &clause.selector_bounds {
                        validate_trace_selector_bound(clause, bound)?;
                        for arg in &bound.spec_args {
                            self.ty(arg, 0)?;
                        }
                    }
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
        self.generic_param_at_depth(param, 0)
    }

    fn generic_param_at_depth(
        &mut self,
        param: &GenericParam,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "generic parameter")?;
        for bound in &param.bounds {
            self.spec_bound_at_depth(bound, depth + 1)?;
        }
        Ok(())
    }

    fn spec_bound(&mut self, bound: &SpecBound) -> Result<(), MetadataArtifactError> {
        self.spec_bound_at_depth(bound, 0)
    }

    fn spec_bound_at_depth(
        &mut self,
        bound: &SpecBound,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "spec bound")?;
        for arg in &bound.args {
            self.ty(arg, depth + 1)?;
        }
        Ok(())
    }

    fn method(&mut self, method: &PublishedMethod) -> Result<(), MetadataArtifactError> {
        super::descriptors::validate_model_method_identity(&method.identity, &mut self.budget)?;
        self.ty(&method.self_type, 0)?;
        self.budget
            .charge(method.requirements.limits.len(), "method limits")?;
        self.budget.charge(
            method.requirements.trace_requirements.len()
                + method.requirements.trace_obligations.len(),
            "method trace requirements",
        )?;
        for requirement in method
            .requirements
            .trace_requirements
            .iter()
            .chain(&method.requirements.trace_obligations)
        {
            let MethodTraceRequirement::Clause(clause) = requirement else {
                continue;
            };
            self.budget
                .charge(clause.selector_bounds.len(), "trace selector bounds")?;
            for bound in &clause.selector_bounds {
                validate_trace_selector_bound(clause, bound)?;
                for arg in &bound.spec_args {
                    self.ty(arg, 0)?;
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
                self.effect_row(row, 0)?;
            }
        }
        for parameter in &method.owner_params {
            self.generic_param(parameter)?;
        }
        self.callable_values(&method.callable)?;
        method.validate()
    }

    fn ty(&mut self, ty: &Type, depth: usize) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "type graph")?;
        crate::type_validation::validate_type_shape(ty)?;
        if let Some(parameter) = &ty.parameter {
            super::descriptors::validate_model_generic_owner(&parameter.owner, &mut self.budget)?;
            if let Some(bindings) = &self.parameter_bindings {
                bindings.validate_reference(parameter)?;
            }
        }
        for bound in &ty.bounds {
            self.spec_bound_at_depth(bound, depth + 1)?;
        }
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
            ActionTrace::Seq(children) => {
                for child in children {
                    self.action_trace(child, depth + 1)?;
                }
            }
            ActionTrace::Choice(children) => {
                if children.is_empty() {
                    return Err(invalid("action trace choice requires an alternative"));
                }
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
            ActionTrace::ParameterCall { parameter } => {
                self.callable_request_target(parameter, depth + 1)?
            }
            ActionTrace::Empty => {}
        }
        if let ActionTrace::Widened {
            parameter_calls, ..
        } = trace
        {
            for parameter in parameter_calls {
                self.callable_request_target(parameter, depth + 1)?;
            }
            super::codec::spec_invocations::ensure_distinct(parameter_calls)?;
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
    fn trace_spec_definition(
        &mut self,
        definition: &proto::ProtoDefinitionKey,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.charge(
            1 + definition.module.len() + definition.declaration.len(),
            "trace spec definition identity",
        )
    }

    fn callable_contract(
        &mut self,
        target: Option<&proto::ProtoCallableContractTarget>,
    ) -> Result<(), MetadataArtifactError> {
        let target = target.ok_or_else(|| invalid("missing callable contract identity"))?;
        match (target.path.is_empty(), target.method.as_ref()) {
            (false, None) => self
                .budget
                .charge(target.path.len(), "callable contract path"),
            (true, Some(method)) => {
                super::descriptors::validate_method_identity(method, &mut self.budget)
            }
            _ => Err(invalid(
                "callable contract requires exactly one item or method identity",
            )),
        }
    }

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
        if let Some(graph) = &metadata.definition_descriptors {
            super::descriptors::validate_wire(graph, &mut self.budget)?;
        }
        if let Some(origins) = &metadata.definition_origins {
            self.budget.enter(0, "definition origins")?;
            for module in &origins.modules {
                self.budget.enter(0, "definition module")?;
                for _ in module.path.iter().chain(&module.canonical_path) {
                    self.budget.enter(0, "definition module path")?;
                }
            }
            for binding in &origins.declarations {
                self.budget.enter(0, "definition binding")?;
                for _ in binding.module.iter().chain(&binding.declaration) {
                    self.budget.enter(0, "definition binding path")?;
                }
                if let Some(key) = &binding.definition {
                    self.budget.enter(0, "definition identity")?;
                    for _ in key.module.iter().chain(&key.declaration) {
                        self.budget.enter(0, "definition binding path")?;
                    }
                }
            }
        }
        for signature in metadata
            .types
            .iter()
            .chain(&metadata.values)
            .chain(&metadata.enums)
            .chain(&metadata.effects)
            .chain(&metadata.trace_specs)
            .chain(&metadata.protocols)
        {
            for generic in &signature.generic_params {
                self.generic_param(generic)?;
            }
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
            if signature.param_names.len() != signature.generic_params.len()
                || signature
                    .param_names
                    .iter()
                    .zip(&signature.generic_params)
                    .any(|(name, param)| name != &param.name)
            {
                return Err(invalid(
                    "spec generic parameter facts are incomplete or inconsistent",
                ));
            }
            for param in &signature.generic_params {
                self.generic_param(param)?;
            }
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
        for method in &metadata.methods {
            self.budget.charge(method.limits.len(), "method limits")?;
            self.budget.charge(
                method.trace_requirements.len() + method.trace_obligations.len(),
                "method trace requirements",
            )?;
            for requirement in method
                .trace_requirements
                .iter()
                .chain(&method.trace_obligations)
            {
                let Some(clause) = &requirement.clause else {
                    continue;
                };
                self.budget
                    .charge(clause.selector_bounds.len(), "trace selector bounds")?;
                for bound in &clause.selector_bounds {
                    for arg in &bound.spec_args {
                        self.ty(arg, 0)?;
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
                    self.effect_row(row, 0)?;
                }
            }
            if let Some(identity) = &method.identity {
                super::descriptors::validate_method_identity(identity, &mut self.budget)?;
            }
            if let Some(ty) = &method.self_type {
                self.ty(ty, 0)?;
            }
            for param in &method.owner_params {
                self.generic_param(param)?;
            }
            if let Some(graph) = &method.callable {
                self.callable_values(graph)?;
            }
        }
        for implementation in &metadata.spec_impls {
            if let Some(ty) = implementation.self_type.as_ref() {
                self.ty(ty, 0)?;
            }
            for arg in &implementation.args {
                self.ty(arg, 0)?;
            }
            for method in &implementation.methods {
                super::descriptors::validate_method_identity(method, &mut self.budget)?;
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
            self.callable_contract(satisfaction.item.as_ref())?;
            for arg in &satisfaction.args {
                self.ty(arg, 0)?;
            }
        }
        for conformance in &metadata.trace_spec_conformances {
            self.callable_contract(conformance.item.as_ref())?;
            if let Some(definition) = &conformance.definition {
                self.trace_spec_definition(definition)?;
            }
            for arg in &conformance.args {
                self.ty(arg, 0)?;
            }
        }
        for summary in &metadata.effect_summaries {
            if let Some(references) = &summary.residual_checks {
                self.residual_references(references)?;
            }
            if let Some(requests) = &summary.handler_requests {
                self.request_template(requests)?;
            }
            if let Some(graph) = &summary.returned_callables {
                self.callable_values(graph)?;
            }
            if let Some(effects) = &summary.default_handler_effects {
                for row in [&effects.direct_effects, &effects.default_actions]
                    .into_iter()
                    .flatten()
                {
                    self.effect_row(row, 0)?;
                }
                for handler in &effects.handlers {
                    if let Some(action) = &handler.action {
                        self.effect_ref(action, 0)?;
                    }
                    if let Some(row) = &handler.produced {
                        self.effect_row(row, 0)?;
                    }
                }
            }
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
                if let Some(references) = &latent.residual_checks {
                    self.residual_references(references)?;
                }
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
            if let Some(definition) = &summary.definition {
                self.trace_spec_definition(definition)?;
            }
            self.budget
                .charge(summary.alternatives.len(), "trace spec alternatives")?;
            if summary.alternatives.is_empty() {
                return Err(invalid("trace spec requires an alternative"));
            }
            for alternative in &summary.alternatives {
                self.budget
                    .charge(alternative.clauses.len(), "trace spec clauses")?;
                if alternative.clauses.is_empty() {
                    return Err(invalid("trace spec alternative requires a clause"));
                }
                for clause in &alternative.clauses {
                    self.budget
                        .charge(clause.selector_bounds.len(), "trace selector bounds")?;
                    for bound in &clause.selector_bounds {
                        for arg in &bound.spec_args {
                            self.ty(arg, 0)?;
                        }
                    }
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
        self.generic_param_at_depth(generic, 0)
    }

    fn generic_param_at_depth(
        &mut self,
        generic: &proto::ProtoActionGenericParam,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "generic parameter")?;
        for bound in &generic.bounds {
            self.spec_bound_at_depth(bound, depth + 1)?;
        }
        Ok(())
    }

    fn spec_bound(&mut self, bound: &proto::ProtoSpecBound) -> Result<(), MetadataArtifactError> {
        self.spec_bound_at_depth(bound, 0)
    }

    fn spec_bound_at_depth(
        &mut self,
        bound: &proto::ProtoSpecBound,
        depth: usize,
    ) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "spec bound")?;
        for arg in &bound.args {
            self.ty(arg, depth + 1)?;
        }
        Ok(())
    }

    fn ty(&mut self, ty: &proto::ProtoType, depth: usize) -> Result<(), MetadataArtifactError> {
        self.budget.enter(depth, "type graph")?;
        if let Some(owner) = ty
            .parameter
            .as_ref()
            .and_then(|parameter| parameter.owner.as_ref())
        {
            super::descriptors::validate_generic_owner(owner, &mut self.budget)?;
        }
        for bound in &ty.bounds {
            self.spec_bound_at_depth(bound, depth + 1)?;
        }
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
        validate_proto_action_trace_shape(trace)?;
        self.budget
            .charge(trace.parameter_calls.len(), "action trace parameter calls")?;
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
        if let Some(parameter) = &trace.parameter {
            self.callable_request_target(parameter, depth + 1)?;
        }
        for parameter in &trace.parameter_calls {
            self.callable_request_target(parameter, depth + 1)?;
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

pub(super) fn validate_proto_action_trace_shape(
    trace: &proto::ProtoActionTrace,
) -> Result<(), MetadataArtifactError> {
    use proto::ProtoActionTraceKind as K;
    let kind = K::try_from(trace.kind)
        .map_err(|_| invalid("effect summary action trace has unknown kind"))?;
    let (event, parameter, children, actions, parameter_calls) = (
        trace.event.is_some(),
        trace.parameter.is_some(),
        !trace.children.is_empty(),
        !trace.actions.is_empty(),
        !trace.parameter_calls.is_empty(),
    );
    match kind {
        K::Empty if !event && !parameter && !children && !actions && !parameter_calls => Ok(()),
        K::Event if event && !parameter && !children && !actions && !parameter_calls => Ok(()),
        K::ParameterCall if !event && parameter && !children && !actions && !parameter_calls => {
            Ok(())
        }
        K::Seq if !event && !parameter && !actions && !parameter_calls => Ok(()),
        K::Choice if !event && !parameter && !actions && !parameter_calls => {
            if !children {
                Err(invalid("action trace choice requires an alternative"))
            } else {
                Ok(())
            }
        }
        K::Repeat if !event && !parameter && !actions && !parameter_calls => {
            if trace.children.len() != 1 {
                Err(invalid(
                    "effect summary repeat trace must contain exactly one child",
                ))
            } else {
                Ok(())
            }
        }
        K::UnknownOrder if !event && !parameter && !children && !parameter_calls => Ok(()),
        K::Widened if !event && !parameter && !children => Ok(()),
        K::Event if !event => Err(invalid(
            "effect summary action trace event is missing payload",
        )),
        K::ParameterCall if !parameter => Err(invalid(
            "effect summary parameter-call trace has empty parameter",
        )),
        _ => Err(invalid(
            "effect summary action trace contains fields invalid for its kind",
        )),
    }
}

fn invalid(message: impl Into<String>) -> MetadataArtifactError {
    MetadataArtifactError::invalid(crate::PACKAGE_METADATA_FILE, message)
}

fn validate_trace_selector_bound(
    clause: &TraceSpecClause,
    bound: &TraceSelectorBound,
) -> Result<(), MetadataArtifactError> {
    if bound.binding.is_empty()
        || bound.binding == "_"
        || bound.effect_path.is_empty()
        || bound.effect_path.iter().any(String::is_empty)
        || bound.spec_path.is_empty()
        || bound.spec_path.iter().any(String::is_empty)
    {
        return Err(invalid(
            "trace selector bound requires a binding and canonical effect/spec paths",
        ));
    }
    let row = match (&clause.kind, bound.operand) {
        (TraceSpecClauseKind::Allow | TraceSpecClauseKind::Deny, TraceSelectorOperand::Pattern) => {
            &clause.pattern
        }
        (TraceSpecClauseKind::RequireBefore, TraceSelectorOperand::Guard) => &clause.guard,
        (
            TraceSpecClauseKind::RequireBefore | TraceSpecClauseKind::RequireAfter,
            TraceSelectorOperand::Target,
        ) => &clause.target,
        (TraceSpecClauseKind::RequireAfter, TraceSelectorOperand::Obligation) => &clause.obligation,
        _ => {
            return Err(invalid(
                "trace selector bound has an invalid clause operand",
            ));
        }
    };
    let mut effects = row
        .as_ref()
        .into_iter()
        .flat_map(|row| &row.effects)
        .filter(|effect| effect.path == bound.effect_path);
    let effect = effects.next().ok_or_else(|| {
        invalid("trace selector bound does not reference an effect in its operand")
    })?;
    if effects.next().is_some() {
        return Err(invalid(
            "trace selector bound references an ambiguous effect path",
        ));
    }
    let selector = usize::try_from(bound.argument)
        .ok()
        .and_then(|index| effect.args.get(index));
    if !matches!(selector.map(|arg| &arg.kind), Some(EffectArgKind::Wildcard)) {
        return Err(invalid(
            "trace selector bound requires a wildcard at its selector position",
        ));
    }
    Ok(())
}

pub(super) fn validate_model_action(action: &ActionSignature) -> Result<(), MetadataArtifactError> {
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
    let type_names = action
        .generic_params
        .iter()
        .filter(|param| param.kind == GenericParamKind::Type)
        .map(|param| param.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for (kind, name) in action.effect_args.iter().zip(&action.selector_param_names) {
        if matches!(kind, ActionArgKind::Type)
            && (name.is_empty() || !type_names.contains(name.as_str()))
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
