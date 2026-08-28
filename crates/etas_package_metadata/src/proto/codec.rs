use prost::Message;

use std::path::Path;

use crate::{
    EncodedMetadataSection, MetadataArtifactError, MetadataArtifactHeader, MetadataSectionKind,
    decode_metadata_artifact, section_from_message, validate_artifact_schema,
};

use super::schema::*;
use crate::model::*;

pub(crate) fn package_metadata_to_proto(metadata: &PackageMetadata) -> ProtoPackageGraphSection {
    ProtoPackageGraphSection {
        version: metadata.version,
        package: Some(package_identity_to_proto(&metadata.package)),
        dependencies: metadata
            .dependencies
            .iter()
            .map(resolved_dependency_to_proto)
            .collect(),
        external_modules: metadata
            .external_modules
            .iter()
            .map(external_module_to_proto)
            .collect(),
        public_metadata: Some(public_metadata_to_proto(&metadata.public_metadata)),
        effect_metadata: Some(effect_metadata_to_proto(&metadata.effect_metadata)),
        tool_bindings: metadata
            .tool_bindings
            .iter()
            .map(tool_binding_to_proto)
            .collect(),
        bins: metadata.bins.iter().map(bin_to_proto).collect(),
    }
}

pub(crate) fn package_metadata_from_proto(
    section: ProtoPackageGraphSection,
) -> Result<PackageMetadata, MetadataArtifactError> {
    Ok(PackageMetadata {
        version: section.version,
        package: section
            .package
            .map(package_identity_from_proto)
            .transpose()?
            .ok_or_else(|| {
                MetadataArtifactError::invalid(
                    crate::PACKAGE_METADATA_FILE,
                    "package_graph section is missing package identity",
                )
            })?,
        dependencies: section
            .dependencies
            .into_iter()
            .map(resolved_dependency_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        external_modules: section
            .external_modules
            .into_iter()
            .map(external_module_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        public_metadata: section
            .public_metadata
            .map(public_metadata_from_proto)
            .transpose()?
            .unwrap_or_default(),
        effect_metadata: section
            .effect_metadata
            .map(effect_metadata_from_proto)
            .transpose()?
            .unwrap_or_default(),
        tool_bindings: section
            .tool_bindings
            .into_iter()
            .map(tool_binding_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        bins: section
            .bins
            .into_iter()
            .map(bin_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub fn package_metadata_to_sections(metadata: &PackageMetadata) -> Vec<EncodedMetadataSection> {
    let exports = exports_section(metadata);
    let type_contracts = type_contracts_section(&metadata.public_metadata);
    let effect_contracts =
        effect_contracts_section(&metadata.public_metadata, &metadata.effect_metadata);
    let tool_contracts = tool_contracts_section(&metadata.public_metadata, &metadata.tool_bindings);
    let trace_spec_contracts = trace_spec_contracts_section(&metadata.public_metadata);
    let public_symbols = public_symbols_section(&metadata.public_metadata);
    let mut sections = vec![section_from_message(
        MetadataSectionKind::PackageGraph,
        package_metadata_to_proto(metadata),
    )];
    if !exports.modules.is_empty() || !exports.re_exports.is_empty() {
        sections.push(section_from_message(MetadataSectionKind::Exports, exports));
    }
    if !type_contracts.types.is_empty()
        || !type_contracts.enums.is_empty()
        || !type_contracts.flows.is_empty()
        || !type_contracts.agents.is_empty()
        || !type_contracts.tools.is_empty()
        || !type_contracts.effects.is_empty()
        || !type_contracts.actions.is_empty()
        || !type_contracts.trace_specs.is_empty()
        || !type_contracts.protocols.is_empty()
    {
        sections.push(section_from_message(
            MetadataSectionKind::TypeContracts,
            type_contracts,
        ));
    }
    if !effect_contracts.summaries.is_empty()
        || !effect_contracts.tags.is_empty()
        || !effect_contracts.extensions.is_empty()
    {
        sections.push(section_from_message(
            MetadataSectionKind::EffectContracts,
            effect_contracts,
        ));
    }
    if !tool_contracts.signatures.is_empty()
        || !tool_contracts.schemas.is_empty()
        || !tool_contracts.bindings.is_empty()
    {
        sections.push(section_from_message(
            MetadataSectionKind::ToolContracts,
            tool_contracts,
        ));
    }
    if !trace_spec_contracts.trace_specs.is_empty() || !trace_spec_contracts.summaries.is_empty() {
        sections.push(section_from_message(
            MetadataSectionKind::TraceSpecContracts,
            trace_spec_contracts,
        ));
    }
    if !public_symbols.symbols.is_empty() {
        sections.push(section_from_message(
            MetadataSectionKind::PublicSymbols,
            public_symbols,
        ));
    }
    sections
}

pub fn package_metadata_from_package_graph_payload(
    payload: &[u8],
) -> Result<PackageMetadata, MetadataArtifactError> {
    let graph = ProtoPackageGraphSection::decode(payload).map_err(|source| {
        MetadataArtifactError::invalid(
            crate::PACKAGE_METADATA_FILE,
            format!("package_graph protobuf payload is invalid: {source}"),
        )
    })?;
    package_metadata_from_proto(graph)
}

pub fn package_metadata_from_artifact(
    path: &Path,
    bytes: &[u8],
) -> Result<(MetadataArtifactHeader, PackageMetadata), MetadataArtifactError> {
    let artifact = decode_metadata_artifact(path, bytes)?;
    validate_artifact_schema(&artifact.header)?;
    let section = artifact
        .sections
        .get(&MetadataSectionKind::PackageGraph)
        .ok_or_else(|| {
            MetadataArtifactError::invalid(
                path,
                "package metadata artifact is missing package_graph section",
            )
        })?;
    let metadata = package_metadata_from_package_graph_payload(section)?;
    Ok((artifact.header, metadata))
}

fn exports_section(metadata: &PackageMetadata) -> ProtoExportsSection {
    ProtoExportsSection {
        modules: metadata
            .external_modules
            .iter()
            .map(external_module_to_proto)
            .collect(),
        re_exports: metadata
            .public_metadata
            .re_exports
            .iter()
            .map(re_export_to_proto)
            .collect(),
    }
}

fn type_contracts_section(metadata: &PublicMetadata) -> ProtoTypeContractsSection {
    ProtoTypeContractsSection {
        types: metadata
            .types
            .iter()
            .map(named_signature_to_proto)
            .collect(),
        values: metadata
            .values
            .iter()
            .map(named_signature_to_proto)
            .collect(),
        enums: metadata
            .enums
            .iter()
            .map(named_signature_to_proto)
            .collect(),
        flows: metadata
            .flows
            .iter()
            .map(callable_signature_to_proto)
            .collect(),
        agents: metadata
            .agents
            .iter()
            .map(callable_signature_to_proto)
            .collect(),
        tools: metadata
            .tools
            .iter()
            .map(callable_signature_to_proto)
            .collect(),
        effects: metadata
            .effects
            .iter()
            .map(named_signature_to_proto)
            .collect(),
        actions: metadata
            .actions
            .iter()
            .map(action_signature_to_proto)
            .collect(),
        trace_specs: metadata
            .trace_specs
            .iter()
            .map(named_signature_to_proto)
            .collect(),
        spec_signatures: metadata
            .spec_signatures
            .iter()
            .map(spec_signature_to_proto)
            .collect(),
        spec_impls: metadata.spec_impls.iter().map(spec_impl_to_proto).collect(),
        type_spec_satisfactions: metadata
            .type_spec_satisfactions
            .iter()
            .map(type_spec_satisfaction_to_proto)
            .collect(),
        callable_spec_satisfactions: metadata
            .callable_spec_satisfactions
            .iter()
            .map(callable_spec_satisfaction_to_proto)
            .collect(),
        trace_spec_conformances: metadata
            .trace_spec_conformances
            .iter()
            .map(trace_spec_conformance_to_proto)
            .collect(),
        protocols: metadata
            .protocols
            .iter()
            .map(named_signature_to_proto)
            .collect(),
    }
}

fn effect_contracts_section(
    public: &PublicMetadata,
    effects: &EffectMetadata,
) -> ProtoEffectContractsSection {
    ProtoEffectContractsSection {
        summaries: public
            .effect_summaries
            .iter()
            .map(effect_summary_to_proto)
            .collect(),
        tags: effects.tags.iter().map(effect_tag_to_proto).collect(),
        extensions: effects
            .extensions
            .iter()
            .map(effect_extension_to_proto)
            .collect(),
    }
}

fn tool_contracts_section(
    public: &PublicMetadata,
    bindings: &[ToolBinding],
) -> ProtoToolContractsSection {
    ProtoToolContractsSection {
        signatures: public
            .tools
            .iter()
            .map(callable_signature_to_proto)
            .collect(),
        schemas: public
            .tool_schemas
            .iter()
            .map(tool_schema_to_proto)
            .collect(),
        bindings: bindings.iter().map(tool_binding_to_proto).collect(),
    }
}

fn trace_spec_contracts_section(public: &PublicMetadata) -> ProtoTraceSpecContractsSection {
    ProtoTraceSpecContractsSection {
        trace_specs: public
            .trace_specs
            .iter()
            .map(named_signature_to_proto)
            .collect(),
        summaries: public
            .trace_spec_summaries
            .iter()
            .map(trace_spec_summary_to_proto)
            .collect(),
    }
}

fn public_symbols_section(public: &PublicMetadata) -> ProtoPublicSymbolsSection {
    let mut symbols = Vec::new();
    symbols.extend(
        public
            .types
            .iter()
            .map(|symbol| public_symbol_to_proto("type", &symbol.path, symbol.visibility)),
    );
    symbols.extend(
        public
            .enums
            .iter()
            .map(|symbol| public_symbol_to_proto("enum", &symbol.path, symbol.visibility)),
    );
    symbols.extend(
        public
            .flows
            .iter()
            .map(|symbol| public_symbol_to_proto("flow", &symbol.path, symbol.visibility)),
    );
    symbols.extend(
        public
            .agents
            .iter()
            .map(|symbol| public_symbol_to_proto("agent", &symbol.path, symbol.visibility)),
    );
    symbols.extend(
        public
            .tools
            .iter()
            .map(|symbol| public_symbol_to_proto("tool", &symbol.path, symbol.visibility)),
    );
    symbols.extend(
        public
            .effects
            .iter()
            .map(|symbol| public_symbol_to_proto("effect", &symbol.path, symbol.visibility)),
    );
    symbols.extend(
        public
            .actions
            .iter()
            .map(|symbol| public_symbol_to_proto("action", &symbol.path, symbol.visibility)),
    );
    symbols.extend(
        public
            .trace_specs
            .iter()
            .map(|symbol| public_symbol_to_proto("trace_spec", &symbol.path, symbol.visibility)),
    );
    symbols.extend(
        public
            .protocols
            .iter()
            .map(|symbol| public_symbol_to_proto("protocol", &symbol.path, symbol.visibility)),
    );
    ProtoPublicSymbolsSection { symbols }
}

fn public_symbol_to_proto(
    kind: &str,
    path: &[String],
    visibility: Visibility,
) -> ProtoPublicSymbol {
    ProtoPublicSymbol {
        kind: kind.to_owned(),
        path: path.to_vec(),
        visibility: visibility_to_wire(visibility).to_owned(),
    }
}

fn package_identity_to_proto(identity: &PackageIdentity) -> ProtoPackageIdentity {
    ProtoPackageIdentity {
        name: identity.name.clone(),
        version: identity.version.clone(),
        edition: identity.edition.clone(),
    }
}

fn package_identity_from_proto(
    identity: ProtoPackageIdentity,
) -> Result<PackageIdentity, MetadataArtifactError> {
    Ok(PackageIdentity {
        name: required(identity.name, "package identity name")?,
        version: required(identity.version, "package identity version")?,
        edition: required(identity.edition, "package identity edition")?,
    })
}

fn resolved_dependency_to_proto(dependency: &ResolvedDependency) -> ProtoResolvedDependency {
    ProtoResolvedDependency {
        identity: Some(package_identity_to_proto(&dependency.identity)),
        import_root: dependency.import_root.clone(),
        source: Some(resolved_source_to_proto(&dependency.source)),
        dependencies: dependency
            .dependencies
            .iter()
            .map(resolved_dependency_to_proto)
            .collect(),
        public_metadata: Some(public_metadata_to_proto(&dependency.public_metadata)),
        effect_metadata: Some(effect_metadata_to_proto(&dependency.effect_metadata)),
        tool_bindings: dependency
            .tool_bindings
            .iter()
            .map(tool_binding_to_proto)
            .collect(),
    }
}

fn resolved_dependency_from_proto(
    dependency: ProtoResolvedDependency,
) -> Result<ResolvedDependency, MetadataArtifactError> {
    Ok(ResolvedDependency {
        identity: dependency
            .identity
            .map(package_identity_from_proto)
            .transpose()?
            .ok_or_else(|| {
                MetadataArtifactError::invalid(
                    crate::PACKAGE_METADATA_FILE,
                    "resolved dependency is missing identity",
                )
            })?,
        import_root: required(dependency.import_root, "resolved dependency import root")?,
        source: dependency
            .source
            .map(resolved_source_from_proto)
            .transpose()?
            .ok_or_else(|| {
                MetadataArtifactError::invalid(
                    crate::PACKAGE_METADATA_FILE,
                    "resolved dependency is missing source",
                )
            })?,
        dependencies: dependency
            .dependencies
            .into_iter()
            .map(resolved_dependency_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        public_metadata: dependency
            .public_metadata
            .map(public_metadata_from_proto)
            .transpose()?
            .unwrap_or_default(),
        effect_metadata: dependency
            .effect_metadata
            .map(effect_metadata_from_proto)
            .transpose()?
            .unwrap_or_default(),
        tool_bindings: dependency
            .tool_bindings
            .into_iter()
            .map(tool_binding_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn resolved_source_to_proto(source: &ResolvedDependencySource) -> ProtoResolvedSource {
    match source {
        ResolvedDependencySource::Builtin { checksum } => ProtoResolvedSource {
            kind: "builtin".to_owned(),
            checksum: checksum.clone(),
            ..Default::default()
        },
        ResolvedDependencySource::Registry {
            registry,
            checksum,
            store,
        } => ProtoResolvedSource {
            kind: "registry".to_owned(),
            registry: registry.clone(),
            checksum: checksum.clone(),
            store: store.clone().unwrap_or_default(),
            ..Default::default()
        },
        ResolvedDependencySource::Git {
            url,
            rev,
            checksum,
            store,
        } => ProtoResolvedSource {
            kind: "git".to_owned(),
            url: url.clone(),
            rev: rev.clone(),
            checksum: checksum.clone(),
            store: store.clone().unwrap_or_default(),
            ..Default::default()
        },
        ResolvedDependencySource::GitHubClone {
            repo,
            rev,
            checksum,
            store,
        } => ProtoResolvedSource {
            kind: "github".to_owned(),
            url: repo.clone(),
            rev: rev.clone(),
            checksum: checksum.clone(),
            store: store.clone().unwrap_or_default(),
            ..Default::default()
        },
        ResolvedDependencySource::GitHubRelease {
            repo,
            release,
            asset,
            asset_checksum,
            payload_checksum,
            store,
        } => ProtoResolvedSource {
            kind: "github_release".to_owned(),
            url: repo.clone(),
            rev: release.clone(),
            path: asset.clone(),
            checksum: payload_checksum.clone(),
            asset_checksum: asset_checksum.clone(),
            store: store.clone().unwrap_or_default(),
            ..Default::default()
        },
        ResolvedDependencySource::Path { path, checksum } => ProtoResolvedSource {
            kind: "path".to_owned(),
            path: path.clone(),
            checksum: checksum.clone(),
            ..Default::default()
        },
        ResolvedDependencySource::Vendor {
            path,
            checksum,
            store,
        } => ProtoResolvedSource {
            kind: "vendor".to_owned(),
            path: path.clone(),
            checksum: checksum.clone(),
            store: store.clone().unwrap_or_default(),
            ..Default::default()
        },
    }
}

fn resolved_source_from_proto(
    source: ProtoResolvedSource,
) -> Result<ResolvedDependencySource, MetadataArtifactError> {
    let store = (!source.store.is_empty()).then_some(source.store);
    match source.kind.as_str() {
        "builtin" => Ok(ResolvedDependencySource::Builtin {
            checksum: required(source.checksum, "builtin source checksum")?,
        }),
        "registry" => Ok(ResolvedDependencySource::Registry {
            registry: required(source.registry, "registry source registry")?,
            checksum: required(source.checksum, "registry source checksum")?,
            store,
        }),
        "git" => Ok(ResolvedDependencySource::Git {
            url: required(source.url, "git source url")?,
            rev: required(source.rev, "git source rev")?,
            checksum: required(source.checksum, "git source checksum")?,
            store,
        }),
        "github" => Ok(ResolvedDependencySource::GitHubClone {
            repo: required(source.url, "github source repo")?,
            rev: required(source.rev, "github source rev")?,
            checksum: required(source.checksum, "github source checksum")?,
            store,
        }),
        "github_release" => Ok(ResolvedDependencySource::GitHubRelease {
            repo: required(source.url, "github release source repo")?,
            release: required(source.rev, "github release source release")?,
            asset: required(source.path, "github release source asset")?,
            asset_checksum: required(
                source.asset_checksum,
                "github release source asset checksum",
            )?,
            payload_checksum: required(source.checksum, "github release source payload checksum")?,
            store,
        }),
        "path" => Ok(ResolvedDependencySource::Path {
            path: required(source.path, "path source path")?,
            checksum: required(source.checksum, "path source checksum")?,
        }),
        "vendor" => Ok(ResolvedDependencySource::Vendor {
            path: required(source.path, "vendor source path")?,
            checksum: required(source.checksum, "vendor source checksum")?,
            store,
        }),
        other => Err(MetadataArtifactError::invalid(
            crate::PACKAGE_METADATA_FILE,
            format!("resolved dependency source kind `{other}` is not supported"),
        )),
    }
}

fn external_module_to_proto(module: &ExternalModule) -> ProtoExternalModule {
    ProtoExternalModule {
        package: module.package.as_ref().map(module_owner_to_proto),
        id: module.id,
        path: module.path.clone(),
        exports: module
            .exports
            .iter()
            .map(external_export_to_proto)
            .collect(),
    }
}

fn external_module_from_proto(
    module: ProtoExternalModule,
) -> Result<ExternalModule, MetadataArtifactError> {
    Ok(ExternalModule {
        package: module.package.map(module_owner_from_proto).transpose()?,
        id: module.id,
        path: required_path(module.path, "external module path")?,
        exports: module
            .exports
            .into_iter()
            .map(external_export_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn module_owner_to_proto(owner: &ExternalModuleOwner) -> ProtoPackageOwner {
    ProtoPackageOwner {
        identity: Some(package_identity_to_proto(&owner.identity)),
        import_root: owner.import_root.clone(),
    }
}

fn module_owner_from_proto(
    owner: ProtoPackageOwner,
) -> Result<ExternalModuleOwner, MetadataArtifactError> {
    Ok(ExternalModuleOwner {
        identity: owner
            .identity
            .map(package_identity_from_proto)
            .transpose()?
            .ok_or_else(|| invalid("external module owner is missing package identity"))?,
        import_root: required(owner.import_root, "external module owner import root")?,
    })
}

fn external_export_to_proto(export: &ExternalExport) -> ProtoExternalExport {
    ProtoExternalExport {
        id: export.id,
        name: export.name.clone(),
        visibility: visibility_to_wire(export.visibility).to_owned(),
    }
}

fn external_export_from_proto(
    export: ProtoExternalExport,
) -> Result<ExternalExport, MetadataArtifactError> {
    Ok(ExternalExport {
        id: export.id,
        name: required(export.name, "external export name")?,
        visibility: visibility_from_wire(&export.visibility)?,
    })
}

fn public_metadata_to_proto(metadata: &PublicMetadata) -> ProtoPublicMetadataSection {
    ProtoPublicMetadataSection {
        modules: metadata
            .modules
            .iter()
            .map(external_module_to_proto)
            .collect(),
        types: metadata
            .types
            .iter()
            .map(named_signature_to_proto)
            .collect(),
        values: metadata
            .values
            .iter()
            .map(named_signature_to_proto)
            .collect(),
        enums: metadata
            .enums
            .iter()
            .map(named_signature_to_proto)
            .collect(),
        flows: metadata
            .flows
            .iter()
            .map(callable_signature_to_proto)
            .collect(),
        agents: metadata
            .agents
            .iter()
            .map(callable_signature_to_proto)
            .collect(),
        tools: metadata
            .tools
            .iter()
            .map(callable_signature_to_proto)
            .collect(),
        effects: metadata
            .effects
            .iter()
            .map(named_signature_to_proto)
            .collect(),
        actions: metadata
            .actions
            .iter()
            .map(action_signature_to_proto)
            .collect(),
        trace_specs: metadata
            .trace_specs
            .iter()
            .map(named_signature_to_proto)
            .collect(),
        spec_signatures: metadata
            .spec_signatures
            .iter()
            .map(spec_signature_to_proto)
            .collect(),
        spec_impls: metadata.spec_impls.iter().map(spec_impl_to_proto).collect(),
        type_spec_satisfactions: metadata
            .type_spec_satisfactions
            .iter()
            .map(type_spec_satisfaction_to_proto)
            .collect(),
        callable_spec_satisfactions: metadata
            .callable_spec_satisfactions
            .iter()
            .map(callable_spec_satisfaction_to_proto)
            .collect(),
        trace_spec_conformances: metadata
            .trace_spec_conformances
            .iter()
            .map(trace_spec_conformance_to_proto)
            .collect(),
        protocols: metadata
            .protocols
            .iter()
            .map(named_signature_to_proto)
            .collect(),
        effect_summaries: metadata
            .effect_summaries
            .iter()
            .map(effect_summary_to_proto)
            .collect(),
        action_summaries: metadata
            .action_summaries
            .iter()
            .map(action_summary_to_proto)
            .collect(),
        tool_schemas: metadata
            .tool_schemas
            .iter()
            .map(tool_schema_to_proto)
            .collect(),
        trace_spec_summaries: metadata
            .trace_spec_summaries
            .iter()
            .map(trace_spec_summary_to_proto)
            .collect(),
        re_exports: metadata.re_exports.iter().map(re_export_to_proto).collect(),
        annotations: metadata
            .annotations
            .iter()
            .map(annotation_metadata_to_proto)
            .collect(),
        fingerprint: metadata.fingerprint.clone().unwrap_or_default(),
    }
}

fn public_metadata_from_proto(
    metadata: ProtoPublicMetadataSection,
) -> Result<PublicMetadata, MetadataArtifactError> {
    Ok(PublicMetadata {
        modules: metadata
            .modules
            .into_iter()
            .map(external_module_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        types: metadata
            .types
            .into_iter()
            .map(named_signature_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        values: metadata
            .values
            .into_iter()
            .map(named_signature_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        enums: metadata
            .enums
            .into_iter()
            .map(named_signature_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        flows: metadata
            .flows
            .into_iter()
            .map(callable_signature_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        agents: metadata
            .agents
            .into_iter()
            .map(callable_signature_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        tools: metadata
            .tools
            .into_iter()
            .map(callable_signature_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        effects: metadata
            .effects
            .into_iter()
            .map(named_signature_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        actions: metadata
            .actions
            .into_iter()
            .map(action_signature_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        trace_specs: metadata
            .trace_specs
            .into_iter()
            .map(named_signature_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        spec_signatures: metadata
            .spec_signatures
            .into_iter()
            .map(spec_signature_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        spec_impls: metadata
            .spec_impls
            .into_iter()
            .map(spec_impl_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        type_spec_satisfactions: metadata
            .type_spec_satisfactions
            .into_iter()
            .map(type_spec_satisfaction_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        callable_spec_satisfactions: metadata
            .callable_spec_satisfactions
            .into_iter()
            .map(callable_spec_satisfaction_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        trace_spec_conformances: metadata
            .trace_spec_conformances
            .into_iter()
            .map(trace_spec_conformance_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        protocols: metadata
            .protocols
            .into_iter()
            .map(named_signature_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        effect_summaries: metadata
            .effect_summaries
            .into_iter()
            .map(effect_summary_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        action_summaries: metadata
            .action_summaries
            .into_iter()
            .map(action_summary_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        tool_schemas: metadata
            .tool_schemas
            .into_iter()
            .map(tool_schema_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        trace_spec_summaries: metadata
            .trace_spec_summaries
            .into_iter()
            .map(trace_spec_summary_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        re_exports: metadata
            .re_exports
            .into_iter()
            .map(re_export_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        annotations: metadata
            .annotations
            .into_iter()
            .map(annotation_metadata_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        fingerprint: (!metadata.fingerprint.is_empty()).then_some(metadata.fingerprint),
    })
}

fn annotation_metadata_to_proto(annotation: &AnnotationMetadata) -> ProtoAnnotationMetadata {
    ProtoAnnotationMetadata {
        item: annotation.item.clone(),
        path: annotation.path.clone(),
        args: annotation
            .args
            .iter()
            .map(annotation_arg_to_proto)
            .collect(),
    }
}

fn annotation_metadata_from_proto(
    annotation: ProtoAnnotationMetadata,
) -> Result<AnnotationMetadata, MetadataArtifactError> {
    Ok(AnnotationMetadata {
        item: required_path(annotation.item, "annotation item")?,
        path: required_path(annotation.path, "annotation path")?,
        args: annotation
            .args
            .into_iter()
            .map(annotation_arg_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn annotation_arg_to_proto(arg: &AnnotationArgMetadata) -> ProtoAnnotationArgMetadata {
    ProtoAnnotationArgMetadata {
        name: arg.name.clone(),
        value: vec![annotation_value_to_proto(&arg.value)],
    }
}

fn annotation_arg_from_proto(
    arg: ProtoAnnotationArgMetadata,
) -> Result<AnnotationArgMetadata, MetadataArtifactError> {
    Ok(AnnotationArgMetadata {
        name: arg.name,
        value: annotation_single_value(arg.value, "annotation argument value")?,
    })
}

fn annotation_field_to_proto(field: &AnnotationFieldMetadata) -> ProtoAnnotationFieldMetadata {
    ProtoAnnotationFieldMetadata {
        name: field.name.clone(),
        value: vec![annotation_value_to_proto(&field.value)],
    }
}

fn annotation_field_from_proto(
    field: ProtoAnnotationFieldMetadata,
) -> Result<AnnotationFieldMetadata, MetadataArtifactError> {
    Ok(AnnotationFieldMetadata {
        name: required(field.name, "annotation record field name")?,
        value: annotation_single_value(field.value, "annotation record field value")?,
    })
}

fn annotation_value_to_proto(value: &AnnotationValueMetadata) -> ProtoAnnotationValueMetadata {
    ProtoAnnotationValueMetadata {
        kind: annotation_value_kind_to_wire(&value.kind).to_owned(),
        value: value.value.clone(),
        path: value.path.clone(),
        elements: value
            .elements
            .iter()
            .map(annotation_value_to_proto)
            .collect(),
        fields: value.fields.iter().map(annotation_field_to_proto).collect(),
    }
}

fn annotation_value_from_proto(
    value: ProtoAnnotationValueMetadata,
) -> Result<AnnotationValueMetadata, MetadataArtifactError> {
    Ok(AnnotationValueMetadata {
        kind: annotation_value_kind_from_wire(&value.kind)?,
        value: value.value,
        path: value.path,
        elements: value
            .elements
            .into_iter()
            .map(annotation_value_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        fields: value
            .fields
            .into_iter()
            .map(annotation_field_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn annotation_single_value(
    mut values: Vec<ProtoAnnotationValueMetadata>,
    field: &str,
) -> Result<AnnotationValueMetadata, MetadataArtifactError> {
    if values.len() != 1 {
        return Err(invalid(format!("{field} must contain exactly one value")));
    }
    annotation_value_from_proto(values.remove(0))
}

fn annotation_value_kind_to_wire(kind: &AnnotationValueKind) -> &'static str {
    match kind {
        AnnotationValueKind::Unit => "unit",
        AnnotationValueKind::Bool => "bool",
        AnnotationValueKind::Int => "int",
        AnnotationValueKind::Float => "float",
        AnnotationValueKind::String => "string",
        AnnotationValueKind::Char => "char",
        AnnotationValueKind::Path => "path",
        AnnotationValueKind::Array => "array",
        AnnotationValueKind::List => "list",
        AnnotationValueKind::Set => "set",
        AnnotationValueKind::Tuple => "tuple",
        AnnotationValueKind::Record => "record",
        AnnotationValueKind::Constructor => "constructor",
        AnnotationValueKind::Limit => "limit",
    }
}

fn annotation_value_kind_from_wire(
    kind: &str,
) -> Result<AnnotationValueKind, MetadataArtifactError> {
    match kind {
        "unit" => Ok(AnnotationValueKind::Unit),
        "bool" => Ok(AnnotationValueKind::Bool),
        "int" => Ok(AnnotationValueKind::Int),
        "float" => Ok(AnnotationValueKind::Float),
        "string" => Ok(AnnotationValueKind::String),
        "char" => Ok(AnnotationValueKind::Char),
        "path" => Ok(AnnotationValueKind::Path),
        "array" => Ok(AnnotationValueKind::Array),
        "list" => Ok(AnnotationValueKind::List),
        "set" => Ok(AnnotationValueKind::Set),
        "tuple" => Ok(AnnotationValueKind::Tuple),
        "record" => Ok(AnnotationValueKind::Record),
        "constructor" => Ok(AnnotationValueKind::Constructor),
        "limit" => Ok(AnnotationValueKind::Limit),
        other => Err(invalid(format!(
            "annotation value kind `{other}` is not supported"
        ))),
    }
}

fn named_signature_to_proto(signature: &NamedSignature) -> ProtoNamedSignature {
    ProtoNamedSignature {
        path: signature.path.clone(),
        visibility: visibility_to_wire(signature.visibility).to_owned(),
        ty: signature.ty.as_ref().map(type_to_proto),
    }
}

fn named_signature_from_proto(
    signature: ProtoNamedSignature,
) -> Result<NamedSignature, MetadataArtifactError> {
    Ok(NamedSignature {
        path: required_path(signature.path, "named signature path")?,
        visibility: visibility_from_wire(&signature.visibility)?,
        ty: signature.ty.map(type_from_proto).transpose()?,
    })
}

fn callable_signature_to_proto(signature: &CallableSignature) -> ProtoCallableSignature {
    ProtoCallableSignature {
        path: signature.path.clone(),
        generic_params: signature
            .generic_params
            .iter()
            .map(generic_param_to_proto)
            .collect(),
        param_names: signature.param_names.clone(),
        input: signature.input.iter().map(type_to_proto).collect(),
        output: signature.output.as_ref().map(type_to_proto),
        effects: signature.effects.as_ref().map(effect_row_to_proto),
        visibility: visibility_to_wire(signature.visibility).to_owned(),
    }
}

fn callable_signature_from_proto(
    signature: ProtoCallableSignature,
) -> Result<CallableSignature, MetadataArtifactError> {
    Ok(CallableSignature {
        path: required_path(signature.path, "callable signature path")?,
        generic_params: signature
            .generic_params
            .into_iter()
            .map(generic_param_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        param_names: signature.param_names,
        input: signature
            .input
            .into_iter()
            .map(type_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        output: signature.output.map(type_from_proto).transpose()?,
        effects: signature.effects.map(effect_row_from_proto).transpose()?,
        visibility: visibility_from_wire(&signature.visibility)?,
    })
}

fn spec_signature_to_proto(signature: &SpecSignature) -> ProtoSpecSignature {
    ProtoSpecSignature {
        path: signature.path.clone(),
        visibility: visibility_to_wire(signature.visibility).to_owned(),
        kind: spec_kind_to_wire(signature.kind).to_owned(),
        param_names: signature.param_names.clone(),
        callable: signature.callable.as_ref().map(callable_signature_to_proto),
        methods: signature.methods.iter().map(spec_method_to_proto).collect(),
        super_specs: signature
            .super_specs
            .iter()
            .map(spec_bound_to_proto)
            .collect(),
    }
}

fn spec_signature_from_proto(
    signature: ProtoSpecSignature,
) -> Result<SpecSignature, MetadataArtifactError> {
    Ok(SpecSignature {
        path: required_path(signature.path, "spec signature path")?,
        visibility: visibility_from_wire(&signature.visibility)?,
        kind: spec_kind_from_wire(&signature.kind)?,
        param_names: signature.param_names,
        callable: signature
            .callable
            .map(callable_signature_from_proto)
            .transpose()?,
        methods: signature
            .methods
            .into_iter()
            .map(spec_method_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        super_specs: signature
            .super_specs
            .into_iter()
            .map(spec_bound_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn spec_method_to_proto(method: &SpecMethod) -> ProtoSpecMethod {
    ProtoSpecMethod {
        name: method.name.clone(),
        path: method.path.clone(),
        signature: method.signature.as_ref().map(callable_signature_to_proto),
    }
}

fn spec_method_from_proto(method: ProtoSpecMethod) -> Result<SpecMethod, MetadataArtifactError> {
    Ok(SpecMethod {
        name: required(method.name, "spec method name")?,
        path: required_path(method.path, "spec method path")?,
        signature: method
            .signature
            .map(callable_signature_from_proto)
            .transpose()?,
    })
}

fn spec_bound_to_proto(bound: &SpecBound) -> ProtoSpecBound {
    ProtoSpecBound {
        spec: bound.spec.clone(),
        args: bound.args.iter().map(type_to_proto).collect(),
    }
}

fn spec_bound_from_proto(bound: ProtoSpecBound) -> Result<SpecBound, MetadataArtifactError> {
    Ok(SpecBound {
        spec: required_path(bound.spec, "spec bound path")?,
        args: bound
            .args
            .into_iter()
            .map(type_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn spec_impl_to_proto(implementation: &SpecImpl) -> ProtoSpecImpl {
    ProtoSpecImpl {
        self_type: Some(type_to_proto(&implementation.self_type)),
        spec: implementation.spec.clone(),
        args: implementation.args.iter().map(type_to_proto).collect(),
        methods: implementation.methods.clone(),
    }
}

fn spec_impl_from_proto(implementation: ProtoSpecImpl) -> Result<SpecImpl, MetadataArtifactError> {
    Ok(SpecImpl {
        self_type: type_from_proto(required_message(
            implementation.self_type,
            "spec impl self type",
        )?)?,
        spec: required_path(implementation.spec, "spec impl spec path")?,
        args: implementation
            .args
            .into_iter()
            .map(type_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        methods: implementation.methods,
    })
}

fn type_spec_satisfaction_to_proto(fact: &TypeSpecSatisfaction) -> ProtoTypeSpecSatisfaction {
    ProtoTypeSpecSatisfaction {
        self_type: Some(type_to_proto(&fact.self_type)),
        spec: fact.spec.clone(),
        args: fact.args.iter().map(type_to_proto).collect(),
    }
}

fn type_spec_satisfaction_from_proto(
    fact: ProtoTypeSpecSatisfaction,
) -> Result<TypeSpecSatisfaction, MetadataArtifactError> {
    Ok(TypeSpecSatisfaction {
        self_type: type_from_proto(required_message(
            fact.self_type,
            "type spec satisfaction self type",
        )?)?,
        spec: required_path(fact.spec, "type spec satisfaction spec path")?,
        args: fact
            .args
            .into_iter()
            .map(type_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn callable_spec_satisfaction_to_proto(
    fact: &CallableSpecSatisfaction,
) -> ProtoCallableSpecSatisfaction {
    ProtoCallableSpecSatisfaction {
        item: fact.item.clone(),
        spec: fact.spec.clone(),
        args: fact.args.iter().map(type_to_proto).collect(),
    }
}

fn callable_spec_satisfaction_from_proto(
    fact: ProtoCallableSpecSatisfaction,
) -> Result<CallableSpecSatisfaction, MetadataArtifactError> {
    Ok(CallableSpecSatisfaction {
        item: required_path(fact.item, "callable spec satisfaction item path")?,
        spec: required_path(fact.spec, "callable spec satisfaction spec path")?,
        args: fact
            .args
            .into_iter()
            .map(type_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn trace_spec_conformance_to_proto(fact: &TraceSpecConformance) -> ProtoTraceSpecConformance {
    match &fact.target {
        TraceSpecConformanceTarget::Inline => ProtoTraceSpecConformance {
            item: fact.item.clone(),
            target_kind: "inline".to_owned(),
            spec: Vec::new(),
            args: Vec::new(),
        },
        TraceSpecConformanceTarget::Named { spec, args } => ProtoTraceSpecConformance {
            item: fact.item.clone(),
            target_kind: "named".to_owned(),
            spec: spec.clone(),
            args: args.iter().map(type_to_proto).collect(),
        },
    }
}

fn trace_spec_conformance_from_proto(
    fact: ProtoTraceSpecConformance,
) -> Result<TraceSpecConformance, MetadataArtifactError> {
    let item = required_path(fact.item, "trace spec conformance item path")?;
    let target = match fact.target_kind.as_str() {
        "inline" | "" if fact.spec.is_empty() => TraceSpecConformanceTarget::Inline,
        "named" => TraceSpecConformanceTarget::Named {
            spec: required_path(fact.spec, "trace spec conformance spec path")?,
            args: fact
                .args
                .into_iter()
                .map(type_from_proto)
                .collect::<Result<Vec<_>, _>>()?,
        },
        kind => {
            return Err(invalid(format!(
                "unknown trace spec conformance target kind `{kind}`"
            )));
        }
    };
    Ok(TraceSpecConformance { item, target })
}

fn action_signature_to_proto(signature: &ActionSignature) -> ProtoActionSignature {
    ProtoActionSignature {
        path: signature.path.clone(),
        generic_params: signature
            .generic_params
            .iter()
            .map(generic_param_to_proto)
            .collect(),
        params: signature.params.iter().map(type_to_proto).collect(),
        effect_args: signature
            .effect_args
            .iter()
            .map(action_arg_kind_to_proto)
            .collect(),
        output: signature.output.as_ref().map(type_to_proto),
        returns_never: signature.returns_never,
        visibility: visibility_to_wire(signature.visibility).to_owned(),
        selector_param_names: signature.selector_param_names.clone(),
        selector_defaults: signature
            .selector_defaults
            .iter()
            .map(optional_effect_arg_to_proto)
            .collect(),
    }
}

fn action_signature_from_proto(
    signature: ProtoActionSignature,
) -> Result<ActionSignature, MetadataArtifactError> {
    let signature = ActionSignature {
        path: required_path(signature.path, "action signature path")?,
        generic_params: signature
            .generic_params
            .into_iter()
            .map(generic_param_from_proto)
            .collect::<Result<Vec<_>, MetadataArtifactError>>()?,
        params: signature
            .params
            .into_iter()
            .map(type_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        effect_args: signature
            .effect_args
            .into_iter()
            .map(action_arg_kind_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        selector_param_names: signature.selector_param_names,
        selector_defaults: signature
            .selector_defaults
            .into_iter()
            .map(optional_effect_arg_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        output: signature.output.map(type_from_proto).transpose()?,
        returns_never: signature.returns_never,
        visibility: visibility_from_wire(&signature.visibility)?,
    };
    validate_action_selector_metadata(&signature)?;
    Ok(signature)
}

fn generic_param_to_proto(param: &GenericParam) -> ProtoActionGenericParam {
    ProtoActionGenericParam {
        name: param.name.clone(),
        bounds: param.bounds.iter().map(spec_bound_to_proto).collect(),
        kind: match param.kind {
            GenericParamKind::Type => "type",
            GenericParamKind::Effect => "effect",
        }
        .to_owned(),
    }
}

fn generic_param_from_proto(
    param: ProtoActionGenericParam,
) -> Result<GenericParam, MetadataArtifactError> {
    Ok(GenericParam {
        name: required(param.name, "callable generic parameter name")?,
        kind: match param.kind.as_str() {
            "type" => GenericParamKind::Type,
            "effect" => GenericParamKind::Effect,
            other => {
                return Err(invalid(format!(
                    "unknown callable generic parameter kind `{other}`"
                )));
            }
        },
        bounds: param
            .bounds
            .into_iter()
            .map(spec_bound_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn optional_effect_arg_to_proto(arg: &Option<EffectArg>) -> ProtoOptionalEffectArg {
    ProtoOptionalEffectArg {
        has_value: arg.is_some(),
        value: arg.as_ref().map(effect_arg_to_proto),
    }
}

fn optional_effect_arg_from_proto(
    arg: ProtoOptionalEffectArg,
) -> Result<Option<EffectArg>, MetadataArtifactError> {
    if !arg.has_value {
        return Ok(None);
    }
    let value = arg
        .value
        .ok_or_else(|| invalid("selector default is marked present but missing value"))?;
    Ok(Some(effect_arg_from_proto(value)?))
}

fn action_arg_kind_to_proto(kind: &ActionArgKind) -> ProtoActionArgKind {
    match kind {
        ActionArgKind::Type => ProtoActionArgKind {
            kind: "type".to_owned(),
            ty: String::new(),
        },
        ActionArgKind::MemoryPlace => ProtoActionArgKind {
            kind: "memory_place".to_owned(),
            ty: String::new(),
        },
        ActionArgKind::StaticResourcePath { ty } => ProtoActionArgKind {
            kind: "static_resource_path".to_owned(),
            ty: ty.clone(),
        },
        ActionArgKind::StringPattern => ProtoActionArgKind {
            kind: "string_pattern".to_owned(),
            ty: String::new(),
        },
    }
}

fn action_arg_kind_from_proto(
    kind: ProtoActionArgKind,
) -> Result<ActionArgKind, MetadataArtifactError> {
    match kind.kind.as_str() {
        "type" => {
            reject_unexpected_action_arg_ty(&kind)?;
            Ok(ActionArgKind::Type)
        }
        "memory_place" => {
            reject_unexpected_action_arg_ty(&kind)?;
            Ok(ActionArgKind::MemoryPlace)
        }
        "static_resource_path" => Ok(ActionArgKind::StaticResourcePath {
            ty: required(kind.ty, "value path action argument type")?,
        }),
        "string_pattern" => {
            reject_unexpected_action_arg_ty(&kind)?;
            Ok(ActionArgKind::StringPattern)
        }
        other => Err(invalid(format!(
            "action argument kind `{other}` is not supported"
        ))),
    }
}

fn validate_action_selector_metadata(
    signature: &ActionSignature,
) -> Result<(), MetadataArtifactError> {
    if signature.selector_param_names.len() != signature.effect_args.len() {
        return Err(invalid(format!(
            "action signature `{}` selector_param_names length {} does not match effect_args length {}",
            signature.path.join("."),
            signature.selector_param_names.len(),
            signature.effect_args.len()
        )));
    }
    if signature.selector_defaults.len() != signature.effect_args.len() {
        return Err(invalid(format!(
            "action signature `{}` selector_defaults length {} does not match effect_args length {}",
            signature.path.join("."),
            signature.selector_defaults.len(),
            signature.effect_args.len()
        )));
    }
    let generic_names = signature
        .generic_params
        .iter()
        .map(|param| param.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if generic_names.len() != signature.generic_params.len() {
        return Err(invalid(format!(
            "action signature `{}` contains duplicate generic parameter names",
            signature.path.join(".")
        )));
    }
    for (kind, name) in signature
        .effect_args
        .iter()
        .zip(&signature.selector_param_names)
    {
        if matches!(kind, ActionArgKind::Type)
            && (name.is_empty() || !generic_names.contains(name.as_str()))
        {
            return Err(invalid(format!(
                "action signature `{}` type selector `{name}` does not name a declared generic parameter",
                signature.path.join(".")
            )));
        }
    }
    for (index, (kind, default)) in signature
        .effect_args
        .iter()
        .zip(&signature.selector_defaults)
        .enumerate()
    {
        let Some(default) = default else {
            continue;
        };
        if !effect_arg_matches_action_arg_kind(default, kind) {
            return Err(invalid(format!(
                "action signature `{}` selector default at index {index} does not match selector kind",
                signature.path.join(".")
            )));
        }
    }
    Ok(())
}

fn effect_arg_matches_action_arg_kind(arg: &EffectArg, kind: &ActionArgKind) -> bool {
    if matches!(arg.kind, EffectArgKind::Wildcard) {
        return true;
    }
    match kind {
        ActionArgKind::Type => matches!(arg.kind, EffectArgKind::Type),
        ActionArgKind::MemoryPlace => matches!(arg.kind, EffectArgKind::Path),
        ActionArgKind::StaticResourcePath { .. } => matches!(arg.kind, EffectArgKind::Path),
        ActionArgKind::StringPattern => matches!(
            arg.kind,
            EffectArgKind::String | EffectArgKind::Int | EffectArgKind::Path
        ),
    }
}

fn reject_unexpected_action_arg_ty(kind: &ProtoActionArgKind) -> Result<(), MetadataArtifactError> {
    if kind.ty.is_empty() {
        Ok(())
    } else {
        Err(invalid(format!(
            "action argument kind `{}` must not carry a type name",
            kind.kind
        )))
    }
}

fn type_to_proto(ty: &Type) -> ProtoType {
    ProtoType {
        kind: type_kind_to_wire(ty.kind).to_owned(),
        name: ty.name.clone(),
        path: ty.path.clone(),
        children: ty.children.iter().map(type_to_proto).collect(),
        fields: ty.fields.iter().map(type_field_to_proto).collect(),
        effects: ty.effects.as_ref().map(effect_row_to_proto),
        produced_effects: ty.produced_effects.as_ref().map(effect_row_to_proto),
    }
}

fn type_from_proto(ty: ProtoType) -> Result<Type, MetadataArtifactError> {
    let kind = type_kind_from_wire(&ty.kind)?;
    let children = ty
        .children
        .into_iter()
        .map(type_from_proto)
        .collect::<Result<Vec<_>, _>>()?;
    let fields = ty
        .fields
        .into_iter()
        .map(type_field_from_proto)
        .collect::<Result<Vec<_>, _>>()?;
    let effects = ty.effects.map(effect_row_from_proto).transpose()?;
    let produced_effects = ty.produced_effects.map(effect_row_from_proto).transpose()?;
    validate_type_shape(
        kind,
        &ty.name,
        &ty.path,
        &children,
        &fields,
        &effects,
        &produced_effects,
    )?;
    Ok(Type {
        kind,
        name: ty.name,
        path: ty.path,
        children,
        fields,
        effects,
        produced_effects,
    })
}

fn type_field_to_proto(field: &TypeField) -> ProtoTypeField {
    ProtoTypeField {
        name: field.name.clone(),
        ty: Some(type_to_proto(&field.ty)),
    }
}

fn type_field_from_proto(field: ProtoTypeField) -> Result<TypeField, MetadataArtifactError> {
    if field.name.is_empty() {
        return Err(invalid("record field name is required"));
    }
    let ty = field.ty.map(type_from_proto).transpose()?.ok_or_else(|| {
        MetadataArtifactError::invalid(
            crate::PACKAGE_METADATA_FILE,
            "record field type is required",
        )
    })?;
    Ok(TypeField {
        name: field.name,
        ty,
    })
}

fn effect_row_to_proto(row: &EffectRow) -> ProtoEffectRow {
    ProtoEffectRow {
        effects: row.effects.iter().map(effect_ref_to_proto).collect(),
        tail: row.tail.clone(),
    }
}

fn effect_row_from_proto(row: ProtoEffectRow) -> Result<EffectRow, MetadataArtifactError> {
    Ok(EffectRow {
        effects: row
            .effects
            .into_iter()
            .map(effect_ref_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        tail: row.tail,
    })
}

fn effect_ref_to_proto(effect: &EffectRef) -> ProtoEffectRef {
    ProtoEffectRef {
        path: effect.path.clone(),
        args: effect.args.iter().map(effect_arg_to_proto).collect(),
    }
}

fn effect_ref_from_proto(effect: ProtoEffectRef) -> Result<EffectRef, MetadataArtifactError> {
    if effect.path.is_empty() {
        return Err(invalid("effect reference path is required"));
    }
    Ok(EffectRef {
        path: effect.path,
        args: effect
            .args
            .into_iter()
            .map(effect_arg_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn effect_arg_to_proto(arg: &EffectArg) -> ProtoEffectArg {
    ProtoEffectArg {
        kind: effect_arg_kind_to_wire(arg.kind).to_owned(),
        ty: arg.ty.as_ref().map(type_to_proto),
        path: arg.path.clone(),
        value: arg.value.clone(),
    }
}

fn effect_arg_from_proto(arg: ProtoEffectArg) -> Result<EffectArg, MetadataArtifactError> {
    let kind = effect_arg_kind_from_wire(&arg.kind)?;
    let ty = arg.ty.map(type_from_proto).transpose()?;
    match kind {
        EffectArgKind::Type if ty.is_none() => {
            return Err(invalid("effect type arg is missing type"));
        }
        EffectArgKind::Path if arg.path.is_empty() => {
            return Err(invalid("effect path arg is missing path"));
        }
        _ => {}
    }
    Ok(EffectArg {
        kind,
        ty,
        path: arg.path,
        value: arg.value,
    })
}

fn effect_metadata_to_proto(metadata: &EffectMetadata) -> ProtoEffectMetadataSection {
    ProtoEffectMetadataSection {
        tags: metadata.tags.iter().map(effect_tag_to_proto).collect(),
        extensions: metadata
            .extensions
            .iter()
            .map(effect_extension_to_proto)
            .collect(),
    }
}

fn effect_metadata_from_proto(
    metadata: ProtoEffectMetadataSection,
) -> Result<EffectMetadata, MetadataArtifactError> {
    Ok(EffectMetadata {
        tags: metadata
            .tags
            .into_iter()
            .map(effect_tag_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
        extensions: metadata
            .extensions
            .into_iter()
            .map(effect_extension_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn effect_summary_to_proto(summary: &EffectSummary) -> ProtoEffectSummary {
    ProtoEffectSummary {
        item: summary.item.clone(),
        public_effects: Some(effect_row_to_proto(&summary.public_effects)),
        requested_actions: Some(effect_row_to_proto(&summary.requested_actions)),
        handled_requested_actions: Some(effect_row_to_proto(&summary.handled_requested_actions)),
        latent_flows: summary
            .latent_flows
            .iter()
            .map(latent_flow_summary_to_proto)
            .collect(),
    }
}

fn effect_summary_from_proto(
    summary: ProtoEffectSummary,
) -> Result<EffectSummary, MetadataArtifactError> {
    Ok(EffectSummary {
        item: required_path(summary.item, "effect summary item")?,
        public_effects: summary
            .public_effects
            .map(effect_row_from_proto)
            .transpose()?
            .unwrap_or_default(),
        requested_actions: summary
            .requested_actions
            .map(effect_row_from_proto)
            .transpose()?
            .unwrap_or_default(),
        handled_requested_actions: summary
            .handled_requested_actions
            .map(effect_row_from_proto)
            .transpose()?
            .unwrap_or_default(),
        latent_flows: summary
            .latent_flows
            .into_iter()
            .map(latent_flow_summary_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn latent_flow_summary_to_proto(summary: &LatentFlowSummary) -> ProtoLatentFlowSummary {
    ProtoLatentFlowSummary {
        declared_bound: Some(effect_row_to_proto(&summary.declared_bound)),
        inferred_effects: Some(effect_row_to_proto(&summary.inferred_effects)),
    }
}

fn latent_flow_summary_from_proto(
    summary: ProtoLatentFlowSummary,
) -> Result<LatentFlowSummary, MetadataArtifactError> {
    let declared_bound = summary
        .declared_bound
        .map(effect_row_from_proto)
        .transpose()?
        .unwrap_or_default();
    let inferred_effects = summary
        .inferred_effects
        .map(effect_row_from_proto)
        .transpose()?
        .unwrap_or_default();
    if declared_bound.effects.is_empty() && inferred_effects.effects.is_empty() {
        return Err(invalid(
            "latent flow summary must contain a declared bound or inferred effects",
        ));
    }
    Ok(LatentFlowSummary {
        declared_bound,
        inferred_effects,
    })
}

fn effect_tag_to_proto(tag: &EffectTag) -> ProtoEffectTag {
    ProtoEffectTag {
        path: tag.path.clone(),
        runtime_requirement: tag.runtime_requirement.clone().unwrap_or_default(),
    }
}

fn effect_tag_from_proto(tag: ProtoEffectTag) -> Result<EffectTag, MetadataArtifactError> {
    Ok(EffectTag {
        path: required_path(tag.path, "effect tag path")?,
        runtime_requirement: (!tag.runtime_requirement.is_empty())
            .then_some(tag.runtime_requirement),
    })
}

fn effect_extension_to_proto(extension: &EffectExtension) -> ProtoEffectExtension {
    ProtoEffectExtension {
        child: extension.child.clone(),
        parent: extension.parent.clone(),
    }
}

fn effect_extension_from_proto(
    extension: ProtoEffectExtension,
) -> Result<EffectExtension, MetadataArtifactError> {
    Ok(EffectExtension {
        child: required_path(extension.child, "effect extension child")?,
        parent: required_path(extension.parent, "effect extension parent")?,
    })
}

fn action_summary_to_proto(summary: &ActionSummary) -> ProtoActionSummary {
    ProtoActionSummary {
        action: summary.action.clone(),
        args: summary.args.clone(),
    }
}

fn action_summary_from_proto(
    summary: ProtoActionSummary,
) -> Result<ActionSummary, MetadataArtifactError> {
    Ok(ActionSummary {
        action: required_path(summary.action, "action summary action")?,
        args: summary.args,
    })
}

fn tool_schema_to_proto(schema: &ToolSchema) -> ProtoToolSchema {
    ProtoToolSchema {
        tool: schema.tool.clone(),
        schema_json: schema.schema_json.clone(),
    }
}

fn tool_schema_from_proto(schema: ProtoToolSchema) -> Result<ToolSchema, MetadataArtifactError> {
    Ok(ToolSchema {
        tool: required_path(schema.tool, "tool schema tool")?,
        schema_json: required(schema.schema_json, "tool schema json")?,
    })
}

fn tool_binding_to_proto(binding: &ToolBinding) -> ProtoToolBinding {
    ProtoToolBinding {
        tool: binding.tool.clone(),
        kind: binding.kind.clone(),
        provider: binding.provider.clone(),
        effect_row: binding.effect_row.clone(),
        action_row: binding.action_row.clone(),
    }
}

fn tool_binding_from_proto(
    binding: ProtoToolBinding,
) -> Result<ToolBinding, MetadataArtifactError> {
    Ok(ToolBinding {
        tool: required(binding.tool, "tool binding tool")?,
        kind: required(binding.kind, "tool binding kind")?,
        provider: required(binding.provider, "tool binding provider")?,
        effect_row: binding.effect_row,
        action_row: binding.action_row,
    })
}

fn trace_spec_summary_to_proto(summary: &TraceSpecSummary) -> ProtoTraceSpecSummary {
    ProtoTraceSpecSummary {
        trace_spec: summary.trace_spec.clone(),
        clauses: summary
            .clauses
            .iter()
            .map(trace_spec_clause_to_proto)
            .collect(),
    }
}

fn trace_spec_summary_from_proto(
    summary: ProtoTraceSpecSummary,
) -> Result<TraceSpecSummary, MetadataArtifactError> {
    Ok(TraceSpecSummary {
        trace_spec: required_path(summary.trace_spec, "trace spec summary path")?,
        clauses: summary
            .clauses
            .into_iter()
            .map(trace_spec_clause_from_proto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn trace_spec_clause_to_proto(clause: &TraceSpecClause) -> ProtoTraceSpecClause {
    ProtoTraceSpecClause {
        kind: trace_spec_clause_kind_to_wire(clause.kind.clone()).to_owned(),
        pattern: clause.pattern.as_ref().map(effect_row_to_proto),
        guard: clause.guard.as_ref().map(effect_row_to_proto),
        target: clause.target.as_ref().map(effect_row_to_proto),
        obligation: clause.obligation.as_ref().map(effect_row_to_proto),
    }
}

fn trace_spec_clause_from_proto(
    clause: ProtoTraceSpecClause,
) -> Result<TraceSpecClause, MetadataArtifactError> {
    Ok(TraceSpecClause {
        kind: trace_spec_clause_kind_from_wire(&required(clause.kind, "trace spec clause kind")?)?,
        pattern: clause.pattern.map(effect_row_from_proto).transpose()?,
        guard: clause.guard.map(effect_row_from_proto).transpose()?,
        target: clause.target.map(effect_row_from_proto).transpose()?,
        obligation: clause.obligation.map(effect_row_from_proto).transpose()?,
    })
}

fn re_export_to_proto(re_export: &ReExport) -> ProtoReExport {
    ProtoReExport {
        from: re_export.from.clone(),
        exported: re_export.exported.clone(),
    }
}

fn re_export_from_proto(re_export: ProtoReExport) -> Result<ReExport, MetadataArtifactError> {
    Ok(ReExport {
        from: required_path(re_export.from, "re-export source")?,
        exported: required_path(re_export.exported, "re-export target")?,
    })
}

fn bin_to_proto(bin: &BinTarget) -> ProtoBinTarget {
    ProtoBinTarget {
        name: bin.name.clone(),
        module: bin.module.clone(),
        flow: bin.flow.clone(),
    }
}

fn bin_from_proto(bin: ProtoBinTarget) -> Result<BinTarget, MetadataArtifactError> {
    Ok(BinTarget {
        name: required(bin.name, "bin target name")?,
        module: required(bin.module, "bin target module")?,
        flow: required(bin.flow, "bin target flow")?,
    })
}

fn visibility_to_wire(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Public => "public",
        Visibility::Private => "private",
    }
}

fn visibility_from_wire(value: &str) -> Result<Visibility, MetadataArtifactError> {
    match value {
        "public" => Ok(Visibility::Public),
        "private" => Ok(Visibility::Private),
        other => Err(invalid(format!("visibility `{other}` is not supported"))),
    }
}

fn trace_spec_clause_kind_to_wire(kind: TraceSpecClauseKind) -> &'static str {
    match kind {
        TraceSpecClauseKind::Allow => "allow",
        TraceSpecClauseKind::Deny => "deny",
        TraceSpecClauseKind::RequireBefore => "require_before",
        TraceSpecClauseKind::RequireAfter => "require_after",
    }
}

fn trace_spec_clause_kind_from_wire(
    value: &str,
) -> Result<TraceSpecClauseKind, MetadataArtifactError> {
    match value {
        "allow" => Ok(TraceSpecClauseKind::Allow),
        "deny" => Ok(TraceSpecClauseKind::Deny),
        "require_before" => Ok(TraceSpecClauseKind::RequireBefore),
        "require_after" => Ok(TraceSpecClauseKind::RequireAfter),
        other => Err(invalid(format!(
            "trace spec clause kind `{other}` is not supported"
        ))),
    }
}

fn spec_kind_to_wire(kind: SpecKind) -> &'static str {
    match kind {
        SpecKind::Type => "type",
        SpecKind::Callable => "callable",
        SpecKind::Trace => "trace",
    }
}

fn spec_kind_from_wire(value: &str) -> Result<SpecKind, MetadataArtifactError> {
    match value {
        "type" | "" => Ok(SpecKind::Type),
        "callable" => Ok(SpecKind::Callable),
        "trace" => Ok(SpecKind::Trace),
        other => Err(invalid(format!("spec kind `{other}` is not supported"))),
    }
}

fn type_kind_to_wire(kind: TypeKind) -> &'static str {
    match kind {
        TypeKind::Primitive => "primitive",
        TypeKind::Var => "var",
        TypeKind::Named => "named",
        TypeKind::Applied => "applied",
        TypeKind::Alias => "alias",
        TypeKind::Nominal => "nominal",
        TypeKind::Array => "array",
        TypeKind::List => "list",
        TypeKind::Map => "map",
        TypeKind::Set => "set",
        TypeKind::Range => "range",
        TypeKind::Slice => "slice",
        TypeKind::Option => "option",
        TypeKind::Result => "result",
        TypeKind::Record => "record",
        TypeKind::Tuple => "tuple",
        TypeKind::Function => "function",
        TypeKind::Handler => "handler",
        TypeKind::Trusted => "trusted",
        TypeKind::Untrusted => "untrusted",
        TypeKind::Secret => "secret",
        TypeKind::Public => "public",
        TypeKind::Sanitized => "sanitized",
        TypeKind::Prompt => "prompt",
        TypeKind::PromptPart => "prompt_part",
        TypeKind::Message => "message",
        TypeKind::MemorySelection => "memory_selection",
        TypeKind::Store => "store",
        TypeKind::MemoryRegion => "memory_region",
        TypeKind::ResourceHandle => "resource_handle",
    }
}

fn type_kind_from_wire(value: &str) -> Result<TypeKind, MetadataArtifactError> {
    match value {
        "primitive" => Ok(TypeKind::Primitive),
        "var" => Ok(TypeKind::Var),
        "named" => Ok(TypeKind::Named),
        "applied" => Ok(TypeKind::Applied),
        "alias" => Ok(TypeKind::Alias),
        "nominal" => Ok(TypeKind::Nominal),
        "array" => Ok(TypeKind::Array),
        "list" => Ok(TypeKind::List),
        "map" => Ok(TypeKind::Map),
        "set" => Ok(TypeKind::Set),
        "range" => Ok(TypeKind::Range),
        "slice" => Ok(TypeKind::Slice),
        "option" => Ok(TypeKind::Option),
        "result" => Ok(TypeKind::Result),
        "record" => Ok(TypeKind::Record),
        "tuple" => Ok(TypeKind::Tuple),
        "function" => Ok(TypeKind::Function),
        "handler" => Ok(TypeKind::Handler),
        "trusted" => Ok(TypeKind::Trusted),
        "untrusted" => Ok(TypeKind::Untrusted),
        "secret" => Ok(TypeKind::Secret),
        "public" => Ok(TypeKind::Public),
        "sanitized" => Ok(TypeKind::Sanitized),
        "prompt" => Ok(TypeKind::Prompt),
        "prompt_part" => Ok(TypeKind::PromptPart),
        "message" => Ok(TypeKind::Message),
        "memory_selection" => Ok(TypeKind::MemorySelection),
        "store" => Ok(TypeKind::Store),
        "memory_region" => Ok(TypeKind::MemoryRegion),
        "resource_handle" => Ok(TypeKind::ResourceHandle),
        other => Err(invalid(format!("type kind `{other}` is not supported"))),
    }
}

fn validate_type_shape(
    kind: TypeKind,
    name: &str,
    path: &[String],
    children: &[Type],
    fields: &[TypeField],
    effects: &Option<EffectRow>,
    produced_effects: &Option<EffectRow>,
) -> Result<(), MetadataArtifactError> {
    match kind {
        TypeKind::Primitive | TypeKind::Var | TypeKind::ResourceHandle if name.is_empty() => {
            Err(invalid(format!(
                "type kind `{}` requires name",
                type_kind_to_wire(kind)
            )))
        }
        TypeKind::Named | TypeKind::Applied | TypeKind::Alias | TypeKind::Nominal
            if path.is_empty() =>
        {
            Err(invalid(format!(
                "type kind `{}` requires path",
                type_kind_to_wire(kind)
            )))
        }
        TypeKind::Applied if children.is_empty() => {
            Err(invalid("applied type requires at least one argument child"))
        }
        TypeKind::Alias if children.len() != 1 => {
            Err(invalid("alias type requires exactly one target child"))
        }
        TypeKind::Nominal if children.len() > 1 => Err(invalid(
            "nominal type can carry at most one representation child",
        )),
        TypeKind::Array
        | TypeKind::List
        | TypeKind::Set
        | TypeKind::Range
        | TypeKind::Slice
        | TypeKind::Option
        | TypeKind::Trusted
        | TypeKind::Untrusted
        | TypeKind::Secret
        | TypeKind::Public
        | TypeKind::Sanitized
        | TypeKind::Message
        | TypeKind::MemorySelection
        | TypeKind::MemoryRegion
            if children.len() != 1 =>
        {
            Err(invalid(format!(
                "type kind `{}` requires exactly one child",
                type_kind_to_wire(kind)
            )))
        }
        TypeKind::Map | TypeKind::Result | TypeKind::Store if children.len() != 2 => {
            Err(invalid(format!(
                "type kind `{}` requires exactly two children",
                type_kind_to_wire(kind)
            )))
        }
        TypeKind::Function if children.is_empty() => {
            Err(invalid("function type requires an output child"))
        }
        TypeKind::Handler if effects.is_none() => {
            Err(invalid("handler type requires handled effects"))
        }
        TypeKind::Handler if children.len() > 1 => {
            Err(invalid("handler type can carry at most one result child"))
        }
        TypeKind::Record if children.is_empty() && fields.is_empty() => {
            Err(invalid("record type requires at least one field"))
        }
        TypeKind::Record if !children.is_empty() => Err(invalid("record type cannot use children")),
        kind if kind != TypeKind::Record && !fields.is_empty() => Err(invalid(format!(
            "type kind `{}` cannot use record fields",
            type_kind_to_wire(kind)
        ))),
        kind if !matches!(kind, TypeKind::Function | TypeKind::Handler) && effects.is_some() => {
            Err(invalid(format!(
                "type kind `{}` cannot use effects",
                type_kind_to_wire(kind)
            )))
        }
        kind if kind != TypeKind::Handler && produced_effects.is_some() => Err(invalid(format!(
            "type kind `{}` cannot use handler produced effects",
            type_kind_to_wire(kind)
        ))),
        _ => Ok(()),
    }
}

fn effect_arg_kind_to_wire(kind: EffectArgKind) -> &'static str {
    match kind {
        EffectArgKind::Type => "type",
        EffectArgKind::Path => "path",
        EffectArgKind::String => "string",
        EffectArgKind::Int => "int",
        EffectArgKind::Wildcard => "wildcard",
    }
}

fn effect_arg_kind_from_wire(value: &str) -> Result<EffectArgKind, MetadataArtifactError> {
    match value {
        "type" => Ok(EffectArgKind::Type),
        "path" => Ok(EffectArgKind::Path),
        "string" => Ok(EffectArgKind::String),
        "int" => Ok(EffectArgKind::Int),
        "wildcard" => Ok(EffectArgKind::Wildcard),
        other => Err(invalid(format!(
            "effect arg kind `{other}` is not supported"
        ))),
    }
}

fn required(value: String, field: &str) -> Result<String, MetadataArtifactError> {
    if value.is_empty() {
        Err(invalid(format!("{field} is required")))
    } else {
        Ok(value)
    }
}

fn required_message<T>(value: Option<T>, field: &str) -> Result<T, MetadataArtifactError> {
    value.ok_or_else(|| invalid(format!("{field} is required")))
}

fn required_path(path: Vec<String>, field: &str) -> Result<Vec<String>, MetadataArtifactError> {
    if path.is_empty() || path.iter().any(|segment| segment.is_empty()) {
        Err(invalid(format!("{field} is required")))
    } else {
        Ok(path)
    }
}

fn invalid(message: impl Into<String>) -> MetadataArtifactError {
    MetadataArtifactError::invalid(crate::PACKAGE_METADATA_FILE, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_rejects_github_release_without_asset_checksum() {
        let mut graph = valid_graph();
        graph.dependencies[0]
            .source
            .as_mut()
            .unwrap()
            .asset_checksum
            .clear();

        let error = package_metadata_from_proto(graph).unwrap_err();

        assert!(error.to_string().contains("asset checksum"));
    }

    #[test]
    fn decode_rejects_invalid_visibility() {
        let mut graph = valid_graph();
        graph.public_metadata = Some(ProtoPublicMetadataSection {
            flows: vec![ProtoCallableSignature {
                path: vec!["demo".to_owned(), "main".to_owned()],
                visibility: "internal".to_owned(),
                ..Default::default()
            }],
            ..Default::default()
        });

        let error = package_metadata_from_proto(graph).unwrap_err();

        assert!(error.to_string().contains("visibility"));
    }

    #[test]
    fn decode_rejects_invalid_type_shape() {
        let mut graph = valid_graph();
        graph.public_metadata = Some(ProtoPublicMetadataSection {
            flows: vec![ProtoCallableSignature {
                path: vec!["demo".to_owned(), "main".to_owned()],
                input: vec![ProtoType {
                    kind: "array".to_owned(),
                    ..Default::default()
                }],
                visibility: "public".to_owned(),
                ..Default::default()
            }],
            ..Default::default()
        });

        let error = package_metadata_from_proto(graph).unwrap_err();

        assert!(error.to_string().contains("exactly one child"));
    }

    #[test]
    fn decode_rejects_record_field_without_type() {
        let mut graph = valid_graph();
        graph.public_metadata = Some(ProtoPublicMetadataSection {
            flows: vec![ProtoCallableSignature {
                path: vec!["demo".to_owned(), "main".to_owned()],
                input: vec![ProtoType {
                    kind: "record".to_owned(),
                    fields: vec![ProtoTypeField {
                        name: "value".to_owned(),
                        ty: None,
                    }],
                    ..Default::default()
                }],
                visibility: "public".to_owned(),
                ..Default::default()
            }],
            ..Default::default()
        });

        let error = package_metadata_from_proto(graph).unwrap_err();

        assert!(error.to_string().contains("record field type"));
    }

    #[test]
    fn decode_rejects_tool_binding_without_provider() {
        let mut graph = valid_graph();
        graph.tool_bindings = vec![ProtoToolBinding {
            tool: "demo.tool".to_owned(),
            kind: "provider".to_owned(),
            provider: String::new(),
            ..Default::default()
        }];

        let error = package_metadata_from_proto(graph).unwrap_err();

        assert!(error.to_string().contains("tool binding provider"));
    }

    fn valid_graph() -> ProtoPackageGraphSection {
        ProtoPackageGraphSection {
            version: 1,
            package: Some(identity("demo", "0.1.0")),
            dependencies: vec![ProtoResolvedDependency {
                identity: Some(identity("dep", "1.2.3")),
                import_root: "dep".to_owned(),
                source: Some(ProtoResolvedSource {
                    kind: "github_release".to_owned(),
                    url: "owner/dep".to_owned(),
                    rev: "v1.2.3".to_owned(),
                    path: "dep.etaspkg".to_owned(),
                    checksum: "blake3:payload".to_owned(),
                    asset_checksum: "blake3:asset".to_owned(),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn identity(name: &str, version: &str) -> ProtoPackageIdentity {
        ProtoPackageIdentity {
            name: name.to_owned(),
            version: version.to_owned(),
            edition: "2026".to_owned(),
        }
    }
}
