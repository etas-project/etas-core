mod client;
mod local_stdio;
mod protocol;

pub use client::ConsoleClient;
pub use local_stdio::LocalStdioClient;
pub use protocol::{ConsoleOperation, ConsoleRequest, ConsoleResponse, ConsoleResult};
