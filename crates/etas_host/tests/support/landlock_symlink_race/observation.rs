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
    pub stage: &'static str,
    pub error: io::Error,
}

fn stage<T>(name: &'static str, result: io::Result<T>) -> Result<T, Failure> {
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
    let mut file = stage("read.open", File::open(link.join("readable")))?;
    let identity = stage("read.fstat", identity(&file))?;
    let mut text = String::new();
    stage("read.content", file.read_to_string(&mut text))?;
    Ok((identity, text))
}

pub(super) fn write_through(link: &Path) -> Result<Identity, Failure> {
    let mut file = stage(
        "write.open",
        OpenOptions::new().write(true).open(link.join("writable")),
    )?;
    let identity = stage("write.fstat", identity(&file))?;
    stage("write.content", file.write_all(b"inside"))?;
    Ok(identity)
}

#[derive(Default, Serialize)]
pub(super) struct Observations {
    pub reads: usize,
    pub writes: usize,
    errors: Vec<ErrorCount>,
}

#[derive(Serialize)]
struct ErrorCount {
    stage: &'static str,
    errno: Option<i32>,
    count: usize,
}

impl Observations {
    pub fn denied(&mut self, iteration: usize, link: &Path, failure: Failure) {
        let errno = failure.error.raw_os_error();
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
        if errno != Some(libc::EACCES) {
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
        assert_eq!(
            errno,
            Some(libc::EACCES),
            "unexpected error at {} iteration {iteration}",
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
    assert_eq!(failure.stage, "read.open");
    assert_eq!(failure.error.raw_os_error(), Some(libc::ENOENT));
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
