use super::super::{budget::MetadataGraphBudget, schema::ProtoDefinitionKey};
use super::{annotations, schema as p};
use crate::MetadataArtifactError as Error;

fn key_units(key: &ProtoDefinitionKey) -> usize {
    1usize
        .saturating_add(key.module.len())
        .saturating_add(key.declaration.len())
}
pub(super) fn reference_units(reference: &p::AnnotationReference) -> usize {
    match &reference.kind {
        Some(p::annotation_reference::Kind::Declaration(key)) => key_units(key),
        Some(p::annotation_reference::Kind::Module(module)) => {
            1usize.saturating_add(module.path.len())
        }
        None => 0,
    }
}
pub(super) fn key(
    key: Option<&ProtoDefinitionKey>,
    budget: &mut MetadataGraphBudget,
) -> Result<(), Error> {
    budget.charge(key.map_or(0, key_units), "header declaration identity")
}
pub(super) fn callable(
    callable: &p::Callable,
    budget: &mut MetadataGraphBudget,
) -> Result<(), Error> {
    match &callable.kind {
        Some(p::callable::Kind::Declaration(value)) => key(Some(value), budget),
        Some(p::callable::Kind::Method(value)) => method(value, budget),
        None => Ok(()),
    }
}
pub(super) fn method(method: &p::Method, budget: &mut MetadataGraphBudget) -> Result<(), Error> {
    budget.charge(
        1usize.saturating_add(method.module.len()),
        "header method identity",
    )?;
    key(method.subject.as_ref(), budget)?;
    key(method.selected_spec.as_ref(), budget)
}
fn spec(spec: &p::Spec, budget: &mut MetadataGraphBudget) -> Result<(), Error> {
    key(spec.definition.as_ref(), budget)?;
    budget.charge(spec.arguments.len(), "header spec arguments")
}

pub(super) fn validate(
    headers: &p::Headers,
    budget: &mut MetadataGraphBudget,
) -> Result<(), Error> {
    if let Some(snapshots) = &headers.snapshots {
        super::snapshots::validate_wire(snapshots, budget)?;
    }
    if let Some(bodies) = &headers.bodies {
        super::bodies::validate_wire(bodies, budget)?;
    }
    budget.charge(
        headers.callables.len().saturating_add(headers.agents.len()),
        "definition headers",
    )?;
    for header in &headers.callables {
        if let Some(id) = &header.id {
            callable(id, budget)?;
        }
        budget.charge(7, "callable analysis references")?;
    }
    for agent in &headers.agents {
        key(agent.identity.as_ref(), budget)?;
        budget.charge(1, "agent component")?;
        if let Some(entry) = &agent.default_entry {
            callable(entry, budget)?;
        }
        for method in &agent.methods {
            callable(method, budget)?;
        }
        if let Some(info) = &agent.method_info {
            budget.charge(info.methods.len(), "agent method selection")?;
            for method in &info.methods {
                if let Some(id) = &method.callable {
                    callable(id, budget)?;
                }
                budget.charge(2, "method selection kind and visibility")?;
                for bound in &method.specs {
                    spec(bound, budget)?;
                }
            }
        }
        for tool in &agent.tools {
            key(Some(tool), budget)?;
        }
        budget.charge(
            agent.annotations.len().saturating_add(agent.specs.len()),
            "agent annotations and specs",
        )?;
        for annotation in &agent.annotations {
            budget.charge(
                annotation
                    .path
                    .len()
                    .saturating_add(annotation.arguments.len()),
                "annotation arguments",
            )?;
            let raw =
                annotation
                    .values
                    .iter()
                    .try_fold(annotation.values.len(), |cost, value| {
                        annotations::child_count(value).map(|children| {
                            cost.saturating_add(children)
                                .saturating_add(value_reference_units(value))
                        })
                    })?;
            budget.check(raw, "static annotation graph")?;
            let order = annotations::order(annotation)?;
            let mut costs = vec![0usize; annotation.values.len()];
            for index in order {
                let children = annotations::children(&annotation.values[index])?;
                let mut cost = 1usize
                    .saturating_add(children.len())
                    .saturating_add(value_reference_units(&annotation.values[index]));
                for child in children {
                    cost = cost.saturating_add(costs[child as usize]);
                }
                budget.check(cost, "expanded static annotation graph")?;
                costs[index] = cost;
            }
            let expanded = annotation.arguments.iter().fold(0usize, |count, arg| {
                count.saturating_add(costs[arg.value as usize])
            });
            budget.charge(expanded, "static annotation values")?;
        }
        for attached in &agent.specs {
            match &attached.kind {
                Some(
                    p::attached_spec::Kind::Named(value) | p::attached_spec::Kind::Protocol(value),
                ) => spec(value, budget)?,
                Some(p::attached_spec::Kind::Inline(constraint)) => {
                    budget.charge(
                        constraint.alternatives.len(),
                        "trace constraint alternatives",
                    )?;
                    for alternative in &constraint.alternatives {
                        budget.charge(alternative.rules.len(), "trace constraint rules")?;
                        for pattern in alternative.rules.iter().flat_map(|rule| &rule.patterns) {
                            budget.charge(
                                1usize.saturating_add(pattern.bounds.len()),
                                "trace selector bounds",
                            )?;
                            for bound in &pattern.bounds {
                                if let Some(target) = &bound.target {
                                    key(target.effect.as_ref(), budget)?;
                                }
                                if let Some(value) = &bound.spec {
                                    spec(value, budget)?;
                                }
                            }
                        }
                    }
                }
                None => {}
            }
        }
    }
    Ok(())
}

fn value_reference_units(value: &p::StaticValue) -> usize {
    match &value.kind {
        Some(p::static_value::Kind::Reference(reference)) => reference_units(reference),
        Some(p::static_value::Kind::Constructor(value)) => {
            value.target.as_ref().map_or(0, reference_units)
        }
        _ => 0,
    }
}
