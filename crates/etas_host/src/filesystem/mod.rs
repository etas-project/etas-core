mod client;
mod local;
mod protocol;

pub use client::FilesystemClient;
pub use local::LocalFilesystemClient;
pub use protocol::{
    BoundedDirectoryList, BoundedFileRead, FilesystemEntry, FilesystemOperation, FilesystemRequest,
    FilesystemResponse, FilesystemStat,
};
