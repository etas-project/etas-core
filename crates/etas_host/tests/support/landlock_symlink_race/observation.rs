use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::Path,
};

use serde::Serialize;
use serde_json::json;

#[derive(Debug)]
pub(super) struct Failure {
    pub stage: Stage,
    pub error: io::Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(super) enum Stage {
    #[serde(rename = "read.open")]
    ReadOpen,
    #[serde(rename = "read.fstat")]
    ReadStat,
    #[serde(rename = "read.content")]
    ReadContent,
    #[serde(rename = "write.open")]
    WriteOpen,
    #[serde(rename = "write.fstat")]
    WriteStat,
    #[serde(rename = "write.content")]
    WriteContent,
    #[serde(rename = "control.wait_for_racing")]
    ControlWait,
    #[serde(rename = "control.identity")]
    ControlIdentity,
}

#[derive(Debug, PartialEq, Eq)]
enum FailureClass {
    AccessDenied,
    LookupUnavailable,
    Unexpected,
}

impl Failure {
    fn classify(&self) -> FailureClass {
        // A racing lookup miss produced no descriptor; it is not evidence of
        // permission denial. Stable outside probes still require EACCES.
        if !matches!(self.stage, Stage::ReadOpen | Stage::WriteOpen) {
            return FailureClass::Unexpected;
        }
        match self.error.raw_os_error() {
            Some(libc::EACCES) => FailureClass::AccessDenied,
            Some(libc::ENOENT) => FailureClass::LookupUnavailable,
            _ => FailureClass::Unexpected,
        }
    }

    pub fn is_lookup_miss(&self) -> bool {
        self.classify() == FailureClass::LookupUnavailable
    }
}

fn stage<T>(name: Stage, result: io::Result<T>) -> Result<T, Failure> {
    result.map_err(|error| Failure { stage: name, error })
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct Identity {
    device: u64,
    inode: u64,
}

fn identity(file: &File) -> io::Result<Identity> {
    let metadata = file.metadata()?;
    Ok(Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

pub(super) fn read_through(link: &Path) -> Result<(Identity, String), Failure> {
    let mut file = stage(Stage::ReadOpen, File::open(link.join("readable")))?;
    let identity = stage(Stage::ReadStat, identity(&file))?;
    let mut text = String::new();
    stage(Stage::ReadContent, file.read_to_string(&mut text))?;
    Ok((identity, text))
}

pub(super) fn write_through(link: &Path) -> Result<Identity, Failure> {
    let mut file = stage(
        Stage::WriteOpen,
        OpenOptions::new().write(true).open(link.join("writable")),
    )?;
    let identity = stage(Stage::WriteStat, identity(&file))?;
    stage(Stage::WriteContent, file.write_all(b"inside"))?;
    Ok(identity)
}

pub(super) fn assert_outside_denied(link: &Path) -> (Option<i32>, Option<i32>) {
    let read = read_through(link).unwrap_err();
    let write = write_through(link).unwrap_err();
    assert_eq!(read.stage, Stage::ReadOpen);
    assert_eq!(write.stage, Stage::WriteOpen);
    assert_eq!(read.classify(), FailureClass::AccessDenied);
    assert_eq!(write.classify(), FailureClass::AccessDenied);
    (read.error.raw_os_error(), write.error.raw_os_error())
}

#[derive(Default, Serialize)]
pub(super) struct Observations {
    pub reads: usize,
    pub writes: usize,
    pub denied_reads: usize,
    pub denied_writes: usize,
    read_lookup_unavailable: usize,
    write_lookup_unavailable: usize,
    errors: Vec<ErrorCount>,
}

#[derive(Serialize)]
struct ErrorCount {
    stage: Stage,
    errno: Option<i32>,
    count: usize,
}

impl Observations {
    pub fn observe_racing_failure(&mut self, iteration: usize, link: &Path, failure: Failure) {
        let errno = failure.error.raw_os_error();
        let class = failure.classify();
        if let Some(entry) = self
            .errors
            .iter_mut()
            .find(|entry| entry.stage == failure.stage && entry.errno == errno)
        {
            entry.count += 1;
        } else {
            self.errors.push(ErrorCount {
                stage: failure.stage,
                errno,
                count: 1,
            });
        }
        match (&class, failure.stage) {
            (FailureClass::AccessDenied, Stage::ReadOpen) => self.denied_reads += 1,
            (FailureClass::AccessDenied, Stage::WriteOpen) => self.denied_writes += 1,
            (FailureClass::LookupUnavailable, Stage::ReadOpen) => self.read_lookup_unavailable += 1,
            (FailureClass::LookupUnavailable, Stage::WriteOpen) => {
                self.write_lookup_unavailable += 1
            }
            _ => {}
        }
        if class == FailureClass::Unexpected {
            self.print();
            // These are post-error observations, not an atomic description of
            // the link at the time of the failed lookup.
            let target = match fs::read_link(link) {
                Ok(target) => json!({"path": target}),
                Err(error) => {
                    json!({"errno": error.raw_os_error(), "kind": format!("{:?}", error.kind())})
                }
            };
            println!(
                "\nETAS_ISOLATION_SYMLINK_FAILURE {}",
                json!({
                    "iteration": iteration, "stage": failure.stage, "errno": errno,
                    "post_error_link_target": target,
                })
            );
        }
        assert_ne!(
            class,
            FailureClass::Unexpected,
            "unexpected errno {errno:?} at {:?} iteration {iteration}",
            failure.stage
        );
    }

    pub fn print(&self) {
        println!("\nETAS_ISOLATION_SYMLINK_OPERATIONS {}", json!(self));
    }
}

pub(super) fn print_environment(inside: &Path, outside: &Path) {
    let filesystems: Vec<_> = [("inside", inside), ("outside", outside)]
        .into_iter()
        .map(|(name, path)| {
            let file = File::open(path).unwrap();
            // statfs is a plain C output structure; the live descriptor pins its filesystem.
            let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
            assert_eq!(
                unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) },
                0,
                "{}",
                io::Error::last_os_error()
            );
            json!({"name": name, "filesystem_magic": format!("{:#x}", stat.f_type),
                   "directory": identity(&file).unwrap()})
        })
        .collect();
    println!(
        "ETAS_ISOLATION_SYMLINK_ENVIRONMENT {}",
        json!({"filesystems": filesystems})
    );
}

#[test]
fn read_failure_preserves_the_open_stage_and_kernel_errno() {
    let fixture = etas_host::TestWorkspace::create("symlink-observation-missing").unwrap();
    let failure = read_through(fixture.path()).unwrap_err();
    assert_eq!(failure.stage, Stage::ReadOpen);
    assert_eq!(failure.error.raw_os_error(), Some(libc::ENOENT));
}

#[test]
fn lookup_unavailability_is_not_a_permission_denial_or_a_post_open_error() {
    for stage in [
        Stage::ReadOpen,
        Stage::ReadStat,
        Stage::ReadContent,
        Stage::WriteOpen,
        Stage::WriteStat,
        Stage::WriteContent,
    ] {
        let open = matches!(stage, Stage::ReadOpen | Stage::WriteOpen);
        for errno in [
            libc::ENOENT,
            libc::EACCES,
            libc::EIO,
            libc::EAGAIN,
            libc::EBADF,
        ] {
            let failure = Failure {
                stage,
                error: io::Error::from_raw_os_error(errno),
            };
            let expected = match (open, errno) {
                (true, libc::ENOENT) => FailureClass::LookupUnavailable,
                (true, libc::EACCES) => FailureClass::AccessDenied,
                _ => FailureClass::Unexpected,
            };
            assert_eq!(failure.classify(), expected, "{stage:?}: {errno}");
        }
    }
}

#[test]
fn successful_operations_keep_the_opened_descriptor_identity() {
    let fixture = etas_host::TestWorkspace::create("symlink-observation-identity").unwrap();
    fs::write(fixture.path().join("readable"), "inside").unwrap();
    fs::write(fixture.path().join("writable"), "old").unwrap();
    let (read_identity, text) = read_through(fixture.path()).unwrap();
    assert_eq!(text, "inside");
    assert_eq!(
        read_identity,
        identity(&File::open(fixture.path().join("readable")).unwrap()).unwrap()
    );
    let write_identity = write_through(fixture.path()).unwrap();
    assert_eq!(
        write_identity,
        identity(&File::open(fixture.path().join("writable")).unwrap()).unwrap()
    );
    assert_eq!(
        fs::read_to_string(fixture.path().join("writable")).unwrap(),
        "inside"
    );
}
