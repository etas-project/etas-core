mod client;
mod local;
mod protocol;

pub use client::FilesystemClient;
pub use local::LocalFilesystemClient;
pub use protocol::{
    FilesystemEntry, FilesystemOperation, FilesystemRequest, FilesystemResponse, FilesystemStat,
};
