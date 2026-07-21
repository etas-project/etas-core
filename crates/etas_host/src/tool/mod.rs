pub mod client;
pub mod http;
pub mod mcp;
pub mod process;
pub mod protocol;

pub use client::ToolClient;
pub use http::{HttpToolProtocolAdapter, HttpToolRequestEnvelope, HttpToolResponseEnvelope};
pub use mcp::{McpToolProtocolAdapter, McpToolRequestEnvelope, McpToolResponseEnvelope};
pub use process::{
    ProcessToolProtocolAdapter, ProcessToolRequestEnvelope, ProcessToolResponseEnvelope,
};
pub use protocol::{ToolRef, ToolRequest, ToolResponse, ToolSchema};
