mod annotations;
mod bodies;
mod budget;
mod effects;
mod header_budget;
mod headers;
pub(super) mod identities;
mod schema;
mod snapshots;
mod specs;
mod traces;
mod types;

use etas_code::descriptor::{
    DefinitionDescriptorSet, DefinitionTypeBinding, DescriptorGraphInput, DescriptorLimits,
    TypeInfoRef,
};

use crate::{MAX_METADATA_UNCOMPRESSED_SECTION_SIZE, MetadataArtifactError, PACKAGE_METADATA_FILE};

pub(super) use budget::{validate_model, validate_wire};
pub(super) use schema::Graph as ProtoDescriptorGraph;
pub(super) use schema::Method as ProtoMethodIdentity;

pub(super) fn encode_method_identity(method: &etas_code::MethodId) -> ProtoMethodIdentity {
    identities::encode_method(method)
}

pub(super) fn decode_method_identity(
    method: ProtoMethodIdentity,
) -> Result<etas_code::MethodId, MetadataArtifactError> {
    identities::decode_method(method)
}

pub(super) fn validate_method_identity(
    method: &ProtoMethodIdentity,
    budget: &mut super::budget::MetadataGraphBudget,
) -> Result<(), MetadataArtifactError> {
    header_budget::method(method, budget)
}

pub(super) fn validate_model_method_identity(
    method: &etas_code::MethodId,
    budget: &mut super::budget::MetadataGraphBudget,
) -> Result<(), MetadataArtifactError> {
    let owner = method.implementation();
    let units = |key: &etas_code::DefinitionKey| {
        1usize
            .saturating_add(key.module().len())
            .saturating_add(key.declaration().len())
    };
    budget.charge(
        1usize
            .saturating_add(owner.module().len())
            .saturating_add(units(owner.subject()))
            .saturating_add(owner.selected_spec().map_or(0, units)),
        "spec implementation method identity",
    )
}

pub(super) fn validate_public_bodies(
    metadata: &crate::PublicMetadata,
) -> Result<(), MetadataArtifactError> {
    let Some(bodies) = metadata
        .definition_descriptors
        .as_ref()
        .and_then(|set| set.headers.as_ref())
        .and_then(|headers| headers.bodies.as_ref())
    else {
        return Ok(());
    };
    if bodies.entries.is_empty() {
        return Ok(());
    }
    let origins = metadata
        .definition_origins
        .as_ref()
        .ok_or_else(|| invalid("retained bodies require package definition origins"))?;
    bodies
        .validate_origins(origins)
        .map_err(|error| invalid(error.to_string()))
}

pub(crate) fn fingerprint(set: &DefinitionDescriptorSet) -> etas_code::CodeFingerprint {
    use prost::Message;
    etas_code::CodeFingerprint::of_bytes(
        "etas.package.definition-descriptors.v1",
        &to_proto(set).encode_to_vec(),
    )
}

pub(crate) fn validate(set: &DefinitionDescriptorSet) -> Result<usize, MetadataArtifactError> {
    let mut budget = super::budget::MetadataGraphBudget::default();
    validate_model(set, &mut budget)?;
    Ok(budget.used())
}

pub(super) fn limits() -> DescriptorLimits {
    DescriptorLimits {
        max_nodes: super::MAX_METADATA_GRAPH_NODES,
        max_edges: super::MAX_METADATA_GRAPH_NODES,
        max_expansion_depth: super::MAX_METADATA_GRAPH_DEPTH,
        max_text_bytes: MAX_METADATA_UNCOMPRESSED_SECTION_SIZE,
    }
}

pub(super) fn to_proto(set: &DefinitionDescriptorSet) -> ProtoDescriptorGraph {
    let input = &set.graph;
    ProtoDescriptorGraph {
        version: match &set.headers {
            _ if input.binders.iter().any(|binder| {
                matches!(
                    binder.owner,
                    etas_code::descriptor::GenericOwner::Implementation(_)
                        | etas_code::descriptor::GenericOwner::ImplementationPattern(_)
                )
            }) =>
            {
                10
            }
            _ if input
                .types
                .iter()
                .any(|ty| matches!(ty, etas_code::descriptor::TypeInfo::Existential { .. })) =>
            {
                9
            }
            _ if input.types.iter().any(|ty| {
                matches!(
                    ty,
                    etas_code::descriptor::TypeInfo::Builtin {
                        constructor: etas_code::descriptor::BuiltinTypeInfo::CallableDefinition,
                        ..
                    }
                )
            }) =>
            {
                8
            }
            Some(headers)
                if headers
                    .snapshots
                    .as_ref()
                    .is_some_and(|snapshots| !snapshots.methods.is_empty()) =>
            {
                7
            }
            Some(headers)
                if headers
                    .snapshots
                    .as_ref()
                    .is_some_and(|snapshots| !snapshots.flows.is_empty()) =>
            {
                6
            }
            Some(headers)
                if headers
                    .agents
                    .iter()
                    .any(|agent| agent.method_info.is_some()) =>
            {
                5
            }
            Some(headers) if headers.bodies.is_some() => 4,
            Some(headers) if headers.snapshots.is_some() => 3,
            Some(_) => 2,
            None => 1,
        },
        types: input.types.iter().map(types::encode).collect(),
        rows: input.rows.iter().map(effects::encode_row).collect(),
        binders: input.binders.iter().map(effects::encode_binder).collect(),
        traces: input.traces.iter().map(traces::encode).collect(),
        declarations: set
            .declarations
            .iter()
            .map(|binding| schema::Binding {
                definition: Some(identities::encode_key(&binding.definition)),
                ty: binding.ty.0,
            })
            .collect(),
        headers: set.headers.as_ref().map(headers::encode),
    }
}

pub(super) fn from_proto(
    input: ProtoDescriptorGraph,
) -> Result<DefinitionDescriptorSet, MetadataArtifactError> {
    validate_wire(&input, &mut super::budget::MetadataGraphBudget::default())?;
    if !matches!(
        (
            input.version,
            input.headers.as_ref().map(|headers| (
                headers.snapshots.is_some(),
                headers.bodies.is_some(),
                headers
                    .agents
                    .iter()
                    .any(|agent| agent.method_info.is_some())
            ))
        ),
        (1, None)
            | (2, Some((false, false, false)))
            | (3, Some((true, false, false)))
            | (4, Some((true, true, false)))
            | (5, Some((_, _, true)))
            | (6, Some((true, _, _)))
            | (7, Some((true, _, _)))
            | (8, None)
            | (8, Some((false, false, false)))
            | (8, Some((true, _, _)))
            | (8, Some((_, _, true)))
            | (9, _)
            | (10, _)
    ) {
        return Err(invalid("unsupported descriptor graph schema"));
    }
    let has_callable_definition = input.types.iter().any(|ty| {
        matches!(&ty.kind, Some(schema::ty::Kind::Builtin(builtin)) if builtin.constructor == 23)
    });
    let has_existential = input
        .types
        .iter()
        .any(|ty| matches!(ty.kind, Some(schema::ty::Kind::Existential(_))));
    if (has_existential && input.version < 9) || (input.version == 9 && !has_existential) {
        return Err(invalid("existential types require their descriptor schema"));
    }
    let has_implementation_binders = input.binders.iter().any(|binder| {
        matches!(
            binder.owner.as_ref().and_then(|owner| owner.kind.as_ref()),
            Some(
                schema::generic_owner::Kind::Implementation(_)
                    | schema::generic_owner::Kind::ImplementationPattern(_)
            )
        )
    });
    if has_implementation_binders != (input.version == 10) {
        return Err(invalid(
            "implementation binders require their descriptor schema",
        ));
    }
    if (has_callable_definition && !matches!(input.version, 8..=10))
        || (input.version == 8 && !has_callable_definition)
    {
        return Err(invalid(
            "callable definition types require their descriptor schema",
        ));
    }
    if (matches!(input.version, 5..=7)
        || (matches!(input.version, 8..=10)
            && input.headers.as_ref().is_some_and(|headers| {
                headers
                    .agents
                    .iter()
                    .any(|agent| agent.method_info.is_some())
            })))
        && input.headers.as_ref().is_some_and(|headers| {
            headers
                .agents
                .iter()
                .any(|agent| agent.method_info.is_none())
        })
    {
        return Err(invalid(
            "method selection schema omits an agent's selection table",
        ));
    }
    let has_flows = input
        .headers
        .as_ref()
        .and_then(|headers| headers.snapshots.as_ref())
        .is_some_and(|snapshots| !snapshots.flows.is_empty());
    if (has_flows && !matches!(input.version, 6..=10)) || (input.version == 6 && !has_flows) {
        return Err(invalid(
            "flow snapshot coordinates require their descriptor schema",
        ));
    }
    let has_methods = input
        .headers
        .as_ref()
        .and_then(|headers| headers.snapshots.as_ref())
        .is_some_and(|snapshots| !snapshots.methods.is_empty());
    if (has_methods && !matches!(input.version, 7..=10)) || (input.version == 7 && !has_methods) {
        return Err(invalid(
            "method snapshot coordinates require their descriptor schema",
        ));
    }
    let declarations = input
        .declarations
        .into_iter()
        .map(|binding| {
            Ok(DefinitionTypeBinding {
                definition: identities::decode_key(required(
                    binding.definition,
                    "descriptor declaration identity",
                )?)?,
                ty: TypeInfoRef(binding.ty),
            })
        })
        .collect::<Result<_, MetadataArtifactError>>()?;
    let headers = input.headers.map(headers::decode).transpose()?;
    let input = DescriptorGraphInput {
        types: input
            .types
            .into_iter()
            .map(types::decode)
            .collect::<Result<_, _>>()?,
        rows: input
            .rows
            .into_iter()
            .map(effects::decode_row)
            .collect::<Result<_, _>>()?,
        binders: input
            .binders
            .into_iter()
            .map(effects::decode_binder)
            .collect::<Result<_, _>>()?,
        traces: input
            .traces
            .into_iter()
            .map(traces::decode)
            .collect::<Result<_, _>>()?,
    };
    // Wire references are inert until the shared descriptor validator accepts them.
    let set = DefinitionDescriptorSet {
        graph: input,
        declarations,
        headers,
    };
    set.validate(limits())
        .map_err(|error| invalid(error.to_string()))?;
    Ok(set)
}

fn required<T>(value: Option<T>, field: &'static str) -> Result<T, MetadataArtifactError> {
    value.ok_or_else(|| invalid(format!("descriptor is missing {field}")))
}

fn invalid(reason: impl Into<String>) -> MetadataArtifactError {
    MetadataArtifactError::invalid(PACKAGE_METADATA_FILE, reason)
}

#[cfg(test)]
mod tests;
