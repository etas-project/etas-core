use super::*;
use std::{
    ffi::CString,
    fs::{DirBuilder, File, Metadata, OpenOptions},
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    },
};
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"ETASRPK1";

/// Owner-only local key storage. All file access stays relative to the opened directory.
#[derive(Debug)]
pub struct DirectoryReplayKeyProvider {
    directory: File,
}

impl DirectoryReplayKeyProvider {
    pub fn open(path: &Path) -> Result<Self, ReplayKeyError> {
        match DirBuilder::new().recursive(true).mode(0o700).create(path) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(error) => return Err(io_error("create private replay-key directory", error)),
        }
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
            .map_err(|error| io_error("open private replay-key directory", error))?;
        validate_metadata(
            &directory
                .metadata()
                .map_err(|error| io_error("inspect replay-key directory", error))?,
            true,
        )?;
        Ok(Self { directory })
    }

    fn open_key(&self, id: ReplayKeyId, create: bool) -> Result<File, ReplayKeyError> {
        let name = CString::new(id.to_hex()).map_err(|_| {
            ReplayKeyError::new(ReplayKeyErrorKind::Invalid, "invalid replay key filename")
        })?;
        let flags = libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | if create {
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL
            } else {
                libc::O_RDONLY | libc::O_NONBLOCK
            };
        // The private directory descriptor pins the selected store even if its path is replaced.
        let fd = unsafe { libc::openat(self.directory.as_raw_fd(), name.as_ptr(), flags, 0o600) };
        if fd < 0 {
            let error = std::io::Error::last_os_error();
            if !create && error.kind() == std::io::ErrorKind::NotFound {
                return Err(ReplayKeyError::new(
                    ReplayKeyErrorKind::Missing,
                    format!("checkpoint replay key {id} is unavailable"),
                ));
            }
            return Err(io_error("open checkpoint replay key", error));
        }
        // openat returned a new owned descriptor; File closes it on every path.
        let file = unsafe { File::from_raw_fd(fd) };
        validate_metadata(
            &file
                .metadata()
                .map_err(|error| io_error("inspect checkpoint replay key", error))?,
            false,
        )?;
        Ok(file)
    }
}

impl ReplayKeyProvider for DirectoryReplayKeyProvider {
    fn validate_authority(
        &self,
        authority: &crate::AuthorityContext,
    ) -> Result<(), ReplayKeyError> {
        let fs = &authority.sandbox.filesystem;
        let mut roots = std::collections::BTreeSet::new();
        for root in fs
            .read_roots
            .iter()
            .chain(&fs.write_roots)
            .chain(&fs.delete_roots)
        {
            roots.insert(directory_identity(root.directory().as_raw_fd())?);
        }
        let mut current = self
            .directory
            .try_clone()
            .map_err(|error| io_error("inspect replay-key authority", error))?;
        let mut visited = std::collections::BTreeSet::new();
        loop {
            let identity = directory_identity(current.as_raw_fd())?;
            if roots.contains(&identity) {
                return Err(ReplayKeyError::new(
                    ReplayKeyErrorKind::Invalid,
                    "checkpoint replay-key directory is accessible to program filesystem authority",
                ));
            }
            if !visited.insert(identity) {
                return Err(ReplayKeyError::new(
                    ReplayKeyErrorKind::Invalid,
                    "checkpoint replay-key directory has cyclic ancestry",
                ));
            }
            let fd = unsafe {
                libc::openat(
                    current.as_raw_fd(),
                    c"..".as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(io_error(
                    "inspect replay-key directory ancestry",
                    std::io::Error::last_os_error(),
                ));
            }
            let parent = unsafe { File::from_raw_fd(fd) };
            if directory_identity(parent.as_raw_fd())? == identity {
                return Ok(());
            }
            current = parent;
        }
    }

    fn create(&self) -> Result<Arc<ReplayKey>, ReplayKeyError> {
        ReplayKey::generate().map(Arc::new)
    }

    fn retain(&self, key: &Arc<ReplayKey>) -> Result<(), ReplayKeyError> {
        match self.resolve(key.id()) {
            Ok(_) => return Ok(()),
            Err(error) if error.kind == ReplayKeyErrorKind::Missing => (),
            Err(error) => return Err(error),
        }
        let mut file = self.open_key(key.id(), true)?;
        let mut encoded = Zeroizing::new([0u8; 40]);
        encoded[..8].copy_from_slice(MAGIC);
        encoded[8..].copy_from_slice(key.material.as_ref());
        file.write_all(encoded.as_ref())
            .and_then(|()| file.sync_all())
            .and_then(|()| self.directory.sync_all())
            .map_err(|error| io_error("persist checkpoint replay key", error))?;
        Ok(())
    }

    fn resolve(&self, id: ReplayKeyId) -> Result<Arc<ReplayKey>, ReplayKeyError> {
        let mut file = self.open_key(id, false)?;
        let mut encoded = Zeroizing::new([0u8; 40]);
        file.read_exact(encoded.as_mut())
            .map_err(|error| io_error("read checkpoint replay key", error))?;
        let mut extra = [0; 1];
        if file
            .read(&mut extra)
            .map_err(|error| io_error("validate checkpoint replay key length", error))?
            != 0
            || &encoded[..8] != MAGIC
        {
            return Err(ReplayKeyError::new(
                ReplayKeyErrorKind::Invalid,
                "invalid checkpoint replay key file",
            ));
        }
        let mut material = Zeroizing::new([0; 32]);
        material.copy_from_slice(&encoded[8..]);
        let key = ReplayKey::from_secret(material);
        if key.id() != id {
            return Err(ReplayKeyError::new(
                ReplayKeyErrorKind::Invalid,
                "checkpoint replay key material does not match its identity",
            ));
        }
        Ok(Arc::new(key))
    }
}

fn directory_identity(fd: std::os::fd::RawFd) -> Result<(u64, u64), ReplayKeyError> {
    use std::os::fd::BorrowedFd;
    // Borrow only; the temporary duplicate owns its descriptor, not the caller's.
    let fd = unsafe { BorrowedFd::borrow_raw(fd) }
        .try_clone_to_owned()
        .map_err(|error| io_error("duplicate replay-key directory descriptor", error))?;
    let metadata = File::from(fd)
        .metadata()
        .map_err(|error| io_error("inspect replay-key directory identity", error))?;
    Ok((metadata.dev(), metadata.ino()))
}

fn validate_metadata(metadata: &Metadata, directory: bool) -> Result<(), ReplayKeyError> {
    // The store may be read-only for a consumer, but never accessible to another principal.
    let owner = unsafe { libc::geteuid() };
    if metadata.uid() != owner
        || metadata.mode() & 0o077 != 0
        || metadata.mode() & 0o400 == 0
        || (directory && metadata.mode() & 0o100 == 0)
        || if directory {
            !metadata.is_dir()
        } else {
            !metadata.is_file() || metadata.nlink() != 1
        }
    {
        return Err(ReplayKeyError::new(
            ReplayKeyErrorKind::Invalid,
            "checkpoint replay-key store must be owner-only, without symlinks or hard-linked key files",
        ));
    }
    Ok(())
}

fn io_error(action: &str, error: std::io::Error) -> ReplayKeyError {
    ReplayKeyError::new(
        ReplayKeyErrorKind::Unavailable,
        format!("cannot {action}: {error}"),
    )
}
