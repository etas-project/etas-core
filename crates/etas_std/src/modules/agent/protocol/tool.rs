use crate::{StdModuleId, StdRegistryBuilder, StdSymbolId};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) -> StdSymbolId {
    let tool_ref = super::record(
        builder,
        module,
        "ToolRef",
        &[],
        vec![
            super::field("id", "string"),
            super::field("revision", "string"),
        ],
    );
    super::record(
        builder,
        module,
        "StepDiagnostic",
        &[],
        vec![
            super::field("code", "string"),
            super::field("message", "string"),
            super::field("path", "Option[string]"),
        ],
    );
    super::record(
        builder,
        module,
        "ToolCall",
        &["C"],
        vec![
            super::field("id", "string"),
            super::field("tool", "std.agent.ToolRef"),
            super::field("args", "std.json.JsonValue"),
        ],
    );
    super::record(
        builder,
        module,
        "ToolResult",
        &["C"],
        vec![
            super::field("call_id", "string"),
            super::field(
                "result",
                "Result[std.json.JsonValue, std.agent.StepDiagnostic]",
            ),
        ],
    );
    tool_ref
}
