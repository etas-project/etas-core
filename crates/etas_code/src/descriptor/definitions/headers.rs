use std::collections::{BTreeMap, BTreeSet};

use super::super::*;
use crate::{CallableId, DefinitionKey, DefinitionKind, identity::CallableIdentityRef};

#[derive(Default)]
pub(super) struct Resources {
    pub nodes: usize,
    pub text: usize,
}

struct Budget {
    used: Resources,
    limits: DescriptorLimits,
}
impl Budget {
    fn nodes(&mut self, count: usize) -> Result<(), DescriptorError> {
        self.used.nodes = self.used.nodes.saturating_add(count);
        if self.used.nodes > self.limits.max_nodes {
            return Err(DescriptorError::Limit {
                resource: "header nodes",
                limit: self.limits.max_nodes,
            });
        }
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<(), DescriptorError> {
        self.used.text = self.used.text.saturating_add(text.len());
        if self.used.text > self.limits.max_text_bytes {
            return Err(DescriptorError::Limit {
                resource: "text bytes",
                limit: self.limits.max_text_bytes,
            });
        }
        Ok(())
    }
    fn name(&mut self, name: &str) -> Result<(), DescriptorError> {
        if name.is_empty() || name.contains('\0') {
            return Err(invalid("empty or invalid header name"));
        }
        self.text(name)
    }
    fn path(&mut self, path: &[String]) -> Result<(), DescriptorError> {
        self.nodes(path.len())?;
        for segment in path {
            self.name(segment)?;
        }
        Ok(())
    }
    fn key(&mut self, key: &DefinitionKey) -> Result<(), DescriptorError> {
        self.nodes(1)?;
        self.path(key.module())?;
        self.path(key.declaration())
    }
    fn callable(&mut self, id: &CallableId) -> Result<(), DescriptorError> {
        match id.identity_ref() {
            CallableIdentityRef::Declaration(key) => self.key(key),
            CallableIdentityRef::Method(method) => {
                let owner = method.implementation();
                self.nodes(1)?;
                self.name(method.name())?;
                self.path(owner.module())?;
                self.key(owner.subject())?;
                if let Some(spec) = owner.selected_spec() {
                    self.key(spec)?;
                }
                Ok(())
            }
        }
    }
    fn reference(&mut self, reference: &AnnotationReferenceInfo) -> Result<(), DescriptorError> {
        match reference {
            AnnotationReferenceInfo::Declaration(key) => self.key(key),
            AnnotationReferenceInfo::Module { path, .. } => {
                self.nodes(1)?;
                self.path(path)
            }
        }
    }
    fn arguments(&mut self, arguments: &[AnnotationArgumentInfo]) -> Result<(), DescriptorError> {
        self.nodes(arguments.len())?;
        let mut named = BTreeSet::new();
        for arg in arguments {
            if let Some(name) = &arg.name {
                self.name(name)?;
                if !named.insert(name) {
                    return Err(invalid("duplicate annotation argument"));
                }
            }
        }
        Ok(())
    }
    fn annotation(&mut self, annotation: &AnnotationInfo) -> Result<(), DescriptorError> {
        if annotation.path.is_empty() {
            return Err(invalid("annotation has no name"));
        }
        self.path(&annotation.path)?;
        self.arguments(&annotation.arguments)?;
        let mut pending = annotation
            .arguments
            .iter()
            .map(|arg| (0usize, arg.value.as_ref()))
            .collect::<Vec<_>>();
        while let Some((depth, value)) = pending.pop() {
            if depth > self.limits.max_expansion_depth {
                return Err(DescriptorError::Limit {
                    resource: "annotation depth",
                    limit: self.limits.max_expansion_depth,
                });
            }
            self.nodes(1)?;
            match value {
                StaticValueInfo::Literal(
                    AnnotationLiteralInfo::Integer(text)
                    | AnnotationLiteralInfo::Float(text)
                    | AnnotationLiteralInfo::String(text),
                ) => self.text(text)?,
                StaticValueInfo::Literal(_) => {}
                StaticValueInfo::Reference(reference) => self.reference(reference)?,
                StaticValueInfo::Sequence { elements, .. } => {
                    self.nodes(elements.len())?;
                    pending.extend(elements.iter().map(|value| (depth + 1, value.as_ref())));
                }
                StaticValueInfo::Record(fields) => {
                    self.nodes(fields.len())?;
                    let mut names = BTreeSet::new();
                    for field in fields {
                        self.name(&field.name)?;
                        if !names.insert(&field.name) {
                            return Err(invalid("duplicate annotation record field"));
                        }
                        pending.push((depth + 1, field.value.as_ref()));
                    }
                }
                StaticValueInfo::Constructor {
                    target, arguments, ..
                } => {
                    self.reference(target)?;
                    self.arguments(arguments)?;
                    pending.extend(arguments.iter().map(|arg| (depth + 1, arg.value.as_ref())));
                }
            }
        }
        Ok(())
    }
    fn spec(&mut self, spec: &SpecInfo) -> Result<(), DescriptorError> {
        self.key(&spec.definition)?;
        self.nodes(spec.arguments.len())
    }
}

pub(super) fn measure(
    headers: &DefinitionHeaders,
    limits: DescriptorLimits,
) -> Result<Resources, DescriptorError> {
    let mut budget = Budget {
        used: Resources::default(),
        limits,
    };
    budget.nodes(headers.callables.len().saturating_add(headers.agents.len()))?;
    for callable in &headers.callables {
        budget.callable(&callable.id)?;
        budget.nodes(7)?;
    }
    for agent in &headers.agents {
        budget.key(&agent.identity)?;
        budget.nodes(1)?;
        if let Some(entry) = &agent.default_entry {
            budget.callable(entry)?;
        }
        for method in &agent.methods {
            budget.callable(method)?;
        }
        if let Some(info) = &agent.method_info {
            budget.nodes(info.len())?;
            for method in info {
                budget.callable(&method.callable)?;
                budget.nodes(2)?;
                for spec in &method.specs {
                    budget.spec(spec)?;
                }
            }
        }
        for tool in &agent.tools {
            budget.key(tool)?;
        }
        budget.nodes(agent.annotations.len().saturating_add(agent.specs.len()))?;
        for annotation in &agent.annotations {
            budget.annotation(annotation)?;
        }
        for spec in &agent.specs {
            match spec {
                AttachedSpecInfo::Named(spec) => budget.spec(spec)?,
                AttachedSpecInfo::Protocol {
                    definition,
                    arguments,
                } => {
                    budget.key(definition)?;
                    budget.nodes(arguments.len())?;
                }
                AttachedSpecInfo::InlineTrace(constraint) => {
                    budget.nodes(constraint.alternatives.len())?;
                    for alternative in &constraint.alternatives {
                        budget.nodes(alternative.len())?;
                        for pattern in alternative.iter().flat_map(TraceRuleInfo::patterns) {
                            budget.nodes(1usize.saturating_add(pattern.bounds.len()))?;
                            for bound in &pattern.bounds {
                                match &bound.target {
                                    EffectTargetInfo::Effect(key) => budget.key(key)?,
                                    EffectTargetInfo::Action { effect, name } => {
                                        budget.key(effect)?;
                                        budget.name(name)?;
                                    }
                                }
                                budget.spec(&bound.spec)?;
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(snapshots) = &headers.snapshots {
        budget.nodes(
            snapshots
                .agents
                .len()
                .saturating_add(snapshots.flows.len())
                .saturating_add(snapshots.methods.len()),
        )?;
        for snapshot in snapshots
            .agents
            .iter()
            .chain(&snapshots.flows)
            .chain(&snapshots.methods)
        {
            budget.key(&snapshot.identity)?;
            budget.callable(&snapshot.revision.definition)?;
            budget.nodes(6)?;
            budget.key(&snapshot.dependencies.lexical_owner)?;
            budget.nodes(snapshot.dependencies.modules.len())?;
            for module in &snapshot.dependencies.modules {
                budget.path(&module.module)?;
            }
            budget.nodes(snapshot.dependencies.bindings.len())?;
            for binding in &snapshot.dependencies.bindings {
                budget.path(&binding.path)?;
                budget.key(&binding.definition)?;
            }
            budget.nodes(snapshot.dependencies.artifacts.len())?;
            for artifact in &snapshot.dependencies.artifacts {
                budget.text(&artifact.import_root)?;
                budget.text(&artifact.fingerprint)?;
            }
        }
    }
    if let Some(bodies) = &headers.bodies {
        budget.nodes(bodies.entries.len())?;
        for body in &bodies.entries {
            budget.callable(&body.callable)?;
            budget.nodes(7)?;
            budget.text(&body.source.text)?;
        }
    }
    Ok(budget.used)
}

pub(super) fn validate(
    headers: &DefinitionHeaders,
    input: &DescriptorGraphInput,
    declarations: &[DefinitionTypeBinding],
) -> Result<(), DescriptorError> {
    if headers
        .snapshots
        .as_ref()
        .is_some_and(|snapshots| !snapshots.flows.is_empty() || !snapshots.methods.is_empty())
        && headers
            .agents
            .iter()
            .any(|agent| agent.method_info.is_none())
    {
        return Err(invalid(
            "flow snapshot schema omits agent method selection metadata",
        ));
    }
    if headers
        .agents
        .iter()
        .any(|agent| agent.method_info.is_some())
        && headers
            .agents
            .iter()
            .any(|agent| agent.method_info.is_none())
    {
        return Err(invalid("mixed method selection metadata versions"));
    }
    let graph = DescriptorGraph::from_validated_projection(input.clone());
    let roots = declarations
        .iter()
        .map(|binding| (&binding.definition, binding.ty))
        .collect::<BTreeMap<_, _>>();
    let mut callables = BTreeMap::new();
    for callable in &headers.callables {
        if callables.insert(&callable.id, callable).is_some() {
            return Err(invalid("duplicate callable header identity"));
        }
        let Some(TypeInfo::Function(signature)) = graph.type_info(callable.signature) else {
            return Err(invalid("callable header has no function signature"));
        };
        let expected = match callable.id.identity_ref() {
            CallableIdentityRef::Declaration(key) => GenericOwner::Declaration(key.clone()),
            CallableIdentityRef::Method(method) => GenericOwner::Method(method.clone()),
        };
        for parameter in &signature.generics {
            if graph
                .binder(*parameter)
                .is_none_or(|parameter| parameter.owner != expected)
            {
                return Err(invalid(
                    "callable header generic binder has a different owner",
                ));
            }
        }
        if let Some(key) = callable.id.as_declaration()
            && roots.get(key).copied() != Some(callable.signature)
        {
            return Err(invalid(
                "callable header disagrees with its declaration root",
            ));
        }
        let effects = &callable.analysis.effects;
        for row in [
            effects.declared_bound,
            effects.escaping,
            effects.requested,
            effects.default_handled,
        ]
        .into_iter()
        .flatten()
        {
            if graph.effect_row(row).is_none() {
                return Err(invalid("callable header references a missing effect row"));
            }
        }
        for trace in [callable.analysis.trace, callable.analysis.outcomes]
            .into_iter()
            .flatten()
        {
            if graph.trace(trace).is_none() {
                return Err(invalid("callable header references a missing trace"));
            }
        }
    }
    let mut agents = BTreeSet::new();
    let mut owned_methods = BTreeSet::new();
    for agent in &headers.agents {
        if agent.identity.kind() != DefinitionKind::Agent || !agents.insert(&agent.identity) {
            return Err(invalid("invalid or duplicate agent header identity"));
        }
        if roots.get(&agent.identity).copied() != Some(agent.component)
            || !matches!(graph.type_info(agent.component), Some(TypeInfo::Nominal { definition, .. }) if definition == &agent.identity)
        {
            return Err(invalid(
                "agent header disagrees with its nominal component root",
            ));
        }
        let mut methods = BTreeSet::new();
        for (default, method) in agent
            .default_entry
            .iter()
            .map(|id| (true, id))
            .chain(agent.methods.iter().map(|id| (false, id)))
        {
            let owner = method
                .as_method()
                .ok_or_else(|| invalid("agent callable has no implementation identity"))?;
            if owner.implementation().subject() != &agent.identity
                || (default && owner.name() != "run")
                || !methods.insert(method)
                || !callables.contains_key(method)
            {
                return Err(invalid(
                    "agent callable has a missing, duplicate or inconsistent owner",
                ));
            }
            owned_methods.insert(method);
        }
        if let Some(info) = &agent.method_info {
            let mut seen = BTreeSet::new();
            let mut inherent_names = BTreeSet::new();
            for method in info {
                if !methods.contains(&method.callable) || !seen.insert(&method.callable) {
                    return Err(invalid(
                        "method selection has an unknown or duplicate callable",
                    ));
                }
                if method.inherent
                    && !inherent_names.insert(
                        method
                            .callable
                            .as_method()
                            .ok_or_else(|| invalid("method selection has no method identity"))?
                            .name(),
                    )
                {
                    return Err(invalid("method selection has duplicate inherent names"));
                }
                let mut bindings = BTreeSet::new();
                for spec in &method.specs {
                    if spec.definition.kind() != DefinitionKind::Spec {
                        return Err(invalid(
                            "method selection references a non-spec declaration",
                        ));
                    }
                    if spec
                        .arguments
                        .iter()
                        .any(|arg| graph.type_info(*arg).is_none())
                    {
                        return Err(invalid(
                            "method selection references a missing spec argument",
                        ));
                    }
                    let arguments = spec
                        .arguments
                        .iter()
                        .map(|arg| graph.type_fingerprint(*arg))
                        .collect::<Result<Vec<_>, _>>()?;
                    if !bindings.insert((&spec.definition, arguments)) {
                        return Err(invalid("method selection has duplicate spec bindings"));
                    }
                }
            }
            if seen.len() != methods.len() {
                return Err(invalid("method selection omits an accessible callable"));
            }
        }
        let mut tools = BTreeSet::new();
        for tool in &agent.tools {
            if tool.kind() != DefinitionKind::Tool || !tools.insert(tool) {
                return Err(invalid("invalid or duplicate tool reference"));
            }
        }
        for spec in &agent.specs {
            spec.validate(&graph)?;
        }
    }
    for callable in &headers.callables {
        if let Some(method) = callable.id.as_method() {
            let subject = method.implementation().subject();
            if (subject.kind() == DefinitionKind::Agent && !owned_methods.contains(&callable.id))
                || (subject.kind() != DefinitionKind::Agent && !roots.contains_key(subject))
            {
                return Err(invalid("method header has no accessible owning definition"));
            }
        }
    }
    for binding in declarations {
        match binding.definition.kind() {
            DefinitionKind::Agent if !agents.contains(&binding.definition) => {
                return Err(invalid("agent declaration is missing its header"));
            }
            DefinitionKind::Flow | DefinitionKind::Tool => {
                let id = CallableId::declaration(binding.definition.clone())
                    .map_err(|_| invalid("invalid callable declaration"))?;
                if !callables.contains_key(&id) {
                    return Err(invalid("callable declaration is missing its header"));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn invalid(reason: &'static str) -> DescriptorError {
    DescriptorError::InvalidDescription { reason }
}
