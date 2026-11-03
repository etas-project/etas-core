use crate::{
    StdDecl, StdRecordField, StdRegistryBuilder, StdSymbolKind, StdType, TypeDecl, TypeDeclKind,
};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    // Descriptor handles retain structured native metadata. They are descriptive
    // values, never a source of type equality, permission or compilation evidence.
    for (name, role) in [
        ("AgentId", crate::CodeTypeRole::AgentId),
        ("TypeInfo", crate::CodeTypeRole::TypeInfo),
        ("GenericOwnerInfo", crate::CodeTypeRole::GenericOwnerInfo),
        ("ToolRef", crate::CodeTypeRole::ToolRef),
        ("StaticValueInfo", crate::CodeTypeRole::StaticValueInfo),
        (
            "TraceConstraintInfo",
            crate::CodeTypeRole::TraceConstraintInfo,
        ),
        ("EffectRowInfo", crate::CodeTypeRole::EffectRowInfo),
        ("ActionSummary", crate::CodeTypeRole::ActionSummary),
        ("TraceSummary", crate::CodeTypeRole::TraceSummary),
        ("Determinism", crate::CodeTypeRole::Determinism),
        ("DefinitionId", crate::CodeTypeRole::DefinitionId),
        ("CallableId", crate::CodeTypeRole::CallableId),
        ("DescriptorDetail", crate::CodeTypeRole::DescriptorDetail),
        ("TraceWitness", crate::CodeTypeRole::TraceWitness),
    ] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(
                TypeDecl::generic(name, &[], TypeDeclKind::Struct)
                    .code_role(role)
                    .intrinsic_only(),
            ),
            "Structured descriptive compiler metadata, not executable evidence.",
        );
    }
    for (name, fields) in [
        (
            "AnnotationArgumentInfo",
            vec![
                field("name", option(string())),
                field("value", named("StaticValueInfo")),
            ],
        ),
        (
            "AnnotationInfo",
            vec![
                field("path", array(string())),
                field("arguments", array(named("AnnotationArgumentInfo"))),
            ],
        ),
        (
            "SpecInfo",
            vec![
                field("definition", option(named("DefinitionId"))),
                field("arguments", array(named("TypeInfo"))),
                field("trace", option(named("TraceConstraintInfo"))),
            ],
        ),
        (
            "GenericParameterInfo",
            vec![
                field("owner", named("GenericOwnerInfo")),
                field("position", StdType::parse("usize")),
                field("name", string()),
                field("kind", string()),
                field("bounds", array(named("SpecInfo"))),
            ],
        ),
        (
            "ParameterInfo",
            vec![
                field("name", option(string())),
                field("ty", named("TypeInfo")),
            ],
        ),
        (
            "SignatureInfo",
            vec![
                field("generics", array(named("GenericParameterInfo"))),
                field("parameters", array(named("ParameterInfo"))),
                field("output", named("TypeInfo")),
                field("declared_output", option(named("TypeInfo"))),
                field("effects", option(named("EffectRowInfo"))),
            ],
        ),
        (
            "RevisionInfo",
            vec![
                field("key", string()),
                field("content", string()),
                field("parent", option(string())),
            ],
        ),
        (
            "ModuleOriginInfo",
            vec![field("module", array(string())), field("package", string())],
        ),
        (
            "DefinitionBindingInfo",
            vec![
                field("path", array(string())),
                field("definition", named("DefinitionId")),
            ],
        ),
        (
            "DependencyArtifactInfo",
            vec![
                field("import_root", string()),
                field("fingerprint", string()),
            ],
        ),
        (
            "DependencyInfo",
            vec![
                field("environment", string()),
                field("lexical_owner", named("DefinitionId")),
                field("standard_package", string()),
                field("modules", array(named("ModuleOriginInfo"))),
                field("bindings", array(named("DefinitionBindingInfo"))),
                field("artifacts", array(named("DependencyArtifactInfo"))),
            ],
        ),
        (
            "EffectSummary",
            vec![
                field("declared_bound", option(named("EffectRowInfo"))),
                field("escaping", option(named("EffectRowInfo"))),
                field("requested", option(named("ActionSummary"))),
                field("default_handled", option(named("ActionSummary"))),
            ],
        ),
        (
            "RuntimeCheckInfo",
            vec![
                field("kind", string()),
                field("location", option(named("DiagnosticLocation"))),
                field("spec", option(named("DefinitionId"))),
            ],
        ),
        (
            "ChangeReport",
            vec![
                field("schema", string()),
                field("identity", named("DefinitionId")),
                field("old_revision", named("RevisionInfo")),
                field("new_revision", named("RevisionInfo")),
                field("old_body_available", boolean()),
                field("new_body_available", boolean()),
                field("body_changed", option(boolean())),
                field("signature_changed", boolean()),
                field("dependencies_changed", boolean()),
                field("declared_bound_changed", boolean()),
                field("escaping_changed", option(boolean())),
                field("requested_changed", option(boolean())),
                field("default_handled_changed", option(boolean())),
                field("determinism_changed", option(boolean())),
                field("trace_changed", option(boolean())),
            ],
        ),
        (
            "CheckReport",
            vec![
                field("status", string()),
                field("diagnostics", array(named("DiagnosticRecord"))),
                field("revision", named("RevisionInfo")),
                field("dependencies", named("DependencyInfo")),
                field("snapshot_fingerprint", string()),
                field("effects", named("EffectSummary")),
                field("determinism", option(named("Determinism"))),
                field("trace", option(named("TraceSummary"))),
                field("runtime_checks", array(named("RuntimeCheckInfo"))),
            ],
        ),
    ] {
        declare(builder, module, name, fields, false);
    }
    declare(
        builder,
        module,
        "CompileError",
        vec![
            field("kind", string()),
            field("diagnostics", array(named("DiagnosticRecord"))),
        ],
        true,
    );
}

fn declare(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    fields: Vec<StdRecordField>,
    mutable: bool,
) {
    let declaration = TypeDecl::generic(name, &[], TypeDeclKind::Struct);
    let declaration = if fields.is_empty() {
        declaration.intrinsic_only()
    } else if mutable {
        declaration.with_representation(StdType::Record(fields))
    } else {
        declaration
            .with_representation(StdType::Record(fields.clone()))
            .with_projected_fields(fields)
    };
    builder.symbol(
        module,
        name,
        StdSymbolKind::Type,
        StdDecl::Type(declaration),
        "Immutable compiler report metadata, not executable evidence.",
    );
}

fn field(name: &str, ty: StdType) -> StdRecordField {
    StdRecordField::new(name, ty)
}
fn named(name: &str) -> StdType {
    StdType::Named(format!("std.compiler.{name}"))
}
fn string() -> StdType {
    StdType::Primitive(crate::StdPrimitiveType::String)
}
fn boolean() -> StdType {
    StdType::Primitive(crate::StdPrimitiveType::Bool)
}
fn array(ty: StdType) -> StdType {
    StdType::Array(Box::new(ty))
}
fn option(ty: StdType) -> StdType {
    StdType::Option(Box::new(ty))
}
