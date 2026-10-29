use crate::{
    CodeTypeRole, StdDecl, StdRegistryBuilder, StdSymbolKind, StdType, TypeDecl, TypeDeclKind,
};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    for (name, parameters, role, description) in [
        (
            "AgentDef",
            &["A"][..],
            CodeTypeRole::AgentDefinition,
            "Immutable definition associated with the original nominal agent type.",
        ),
        (
            "CallableDef",
            &["F"][..],
            CodeTypeRole::CallableDefinition,
            "Typed immutable ordinary-flow definition, or its erased read-only view.",
        ),
        (
            "SyntaxBlock",
            &[][..],
            CodeTypeRole::SyntaxBlock,
            "Retained structured body syntax, not a source string or executable proof.",
        ),
    ] {
        let mut declaration = TypeDecl::generic(name, parameters, TypeDeclKind::Struct)
            .code_role(role)
            .intrinsic_only();
        if role == CodeTypeRole::SyntaxBlock {
            declaration = declaration.with_projected_fields(vec![crate::StdRecordField::new(
                "text",
                StdType::parse("string"),
            )]);
        } else {
            let mut fields = vec![crate::StdRecordField::new(
                "body",
                StdType::Option(Box::new(compiler_type("SyntaxBlock"))),
            )];
            fields.extend([
                crate::StdRecordField::new("effects", compiler_type("EffectSummary")),
                crate::StdRecordField::new(
                    "determinism",
                    StdType::Option(Box::new(compiler_type("Determinism"))),
                ),
                crate::StdRecordField::new(
                    "trace",
                    StdType::Option(Box::new(compiler_type("TraceSummary"))),
                ),
                crate::StdRecordField::new("revision", compiler_type("RevisionInfo")),
                crate::StdRecordField::new("dependencies", compiler_type("DependencyInfo")),
            ]);
            if role == CodeTypeRole::AgentDefinition {
                fields.extend([
                    crate::StdRecordField::new(
                        "signature",
                        StdType::Option(Box::new(compiler_type("SignatureInfo"))),
                    ),
                    crate::StdRecordField::new("identity", compiler_type("AgentId")),
                    crate::StdRecordField::new(
                        "annotations",
                        StdType::Array(Box::new(compiler_type("AnnotationInfo"))),
                    ),
                    crate::StdRecordField::new(
                        "tools",
                        StdType::Array(Box::new(compiler_type("ToolRef"))),
                    ),
                    crate::StdRecordField::new(
                        "specs",
                        StdType::Array(Box::new(compiler_type("SpecInfo"))),
                    ),
                    crate::StdRecordField::new(
                        "methods",
                        StdType::Array(Box::new(compiler_type("CallableDef"))),
                    ),
                ]);
            } else {
                fields.extend([
                    crate::StdRecordField::new("id", compiler_type("CallableId")),
                    crate::StdRecordField::new("signature", compiler_type("SignatureInfo")),
                ]);
            }
            declaration = declaration.with_projected_fields(fields);
        }
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(declaration),
            description,
        );
    }
    for (name, fields) in [
        (
            "DiagnosticLocation",
            vec![
                ("source", StdType::parse("string")),
                ("revision", StdType::parse("string")),
                ("byte_start", StdType::parse("usize")),
                ("byte_end", StdType::parse("usize")),
            ],
        ),
        (
            "RelatedLocation",
            vec![
                ("location", compiler_type("DiagnosticLocation")),
                ("message", StdType::parse("string")),
            ],
        ),
        (
            "DiagnosticDetails",
            vec![
                (
                    "expected",
                    StdType::Option(Box::new(compiler_type("DescriptorDetail"))),
                ),
                (
                    "actual",
                    StdType::Option(Box::new(compiler_type("DescriptorDetail"))),
                ),
                (
                    "constraint",
                    StdType::Option(Box::new(compiler_type("ViolatedConstraint"))),
                ),
                (
                    "related",
                    StdType::Array(Box::new(compiler_type("RelatedLocation"))),
                ),
                (
                    "analysis_path",
                    StdType::Array(Box::new(compiler_type("AnalysisStep"))),
                ),
                (
                    "trace_witness",
                    StdType::Option(Box::new(compiler_type("TraceWitness"))),
                ),
            ],
        ),
        (
            "ViolatedConstraint",
            vec![
                ("kind", StdType::parse("string")),
                (
                    "owner",
                    StdType::Option(Box::new(compiler_type("DefinitionId"))),
                ),
                ("message", StdType::parse("string")),
            ],
        ),
        (
            "AnalysisStep",
            vec![
                ("callable", compiler_type("CallableId")),
                (
                    "location",
                    StdType::Option(Box::new(compiler_type("DiagnosticLocation"))),
                ),
                ("message", StdType::parse("string")),
            ],
        ),
        (
            "DiagnosticRecord",
            vec![
                ("code", StdType::parse("string")),
                ("severity", StdType::parse("string")),
                ("message", StdType::parse("string")),
                (
                    "location",
                    StdType::Option(Box::new(compiler_type("DiagnosticLocation"))),
                ),
                (
                    "details",
                    StdType::Option(Box::new(compiler_type("DiagnosticDetails"))),
                ),
            ],
        ),
        (
            "ParseError",
            vec![(
                "diagnostics",
                StdType::Array(Box::new(compiler_type("DiagnosticRecord"))),
            )],
        ),
    ] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(
                TypeDecl::generic(name, &[], TypeDeclKind::Struct)
                    .with_representation(record(&fields)),
            ),
            "Structured compiler syntax diagnostic data.",
        );
    }
}

fn compiler_type(name: &str) -> StdType {
    StdType::Named(format!("std.compiler.{name}"))
}

fn record(fields: &[(&str, StdType)]) -> StdType {
    StdType::Record(
        fields
            .iter()
            .map(|(name, ty)| crate::StdRecordField::new(name, ty.clone()))
            .collect(),
    )
}
