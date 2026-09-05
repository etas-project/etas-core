mod client;
mod local;
mod protocol;

pub use client::FilesystemClient;
pub use local::{LocalFilesystemClient, WorkspaceRegionRegistry};
pub use protocol::{
    FilesystemEntry, FilesystemOperation, FilesystemRequest, FilesystemResponse, FilesystemStat,
};
