use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};

fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

fn isolated(test: &str) -> bool {
    let test = test.strip_prefix("etas_host::").unwrap();
    if std::env::var("ETAS_ATOMIC_MODE_CHILD").as_deref() == Ok(test) {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("ETAS_ATOMIC_MODE_CHILD", test)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("running 1 test"),
        "child did not run {test}: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.status.success(),
        "{}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

#[test]
fn atomic_replace_preserves_existing_unix_rwx_under_different_umasks() {
    if isolated(concat!(
        module_path!(),
        "::atomic_replace_preserves_existing_unix_rwx_under_different_umasks"
    )) {
        return;
    }
    let fixture = TestWorkspace::create("atomic-mode-matrix").unwrap();
    let root = fixture.root().unwrap();
    // Only this isolated child changes its process-global umask.
    for mask in [0o022, 0o077, 0o777] {
        unsafe {
            libc::umask(mask);
        }
        for expected in [0o600, 0o700, 0o444, 0o640, 0o755, 0o000] {
            let path = fixture.path().join("data");
            set_mode(fixture.path(), 0o700);
            if path.exists() {
                fs::remove_file(&path).unwrap();
            }
            fs::write(&path, b"old").unwrap();
            set_mode(&path, expected);
            let policy = FilesystemPolicy {
                read_roots: vec![],
                write_roots: vec![root.clone()],
                delete_roots: vec![],
            };
            FilesystemSandbox::new(policy)
                .atomic_write(&root, Path::new("data"), b"new")
                .unwrap();
            assert_eq!(mode(&path), expected, "umask={mask:o}, mode={expected:o}");
            set_mode(&path, 0o600);
            assert_eq!(fs::read(&path).unwrap(), b"new");
            assert_eq!(fs::read_dir(fixture.path()).unwrap().count(), 1);
        }
    }
}

#[test]
fn atomic_replace_new_file_keeps_creation_umask_semantics() {
    if isolated(concat!(
        module_path!(),
        "::atomic_replace_new_file_keeps_creation_umask_semantics"
    )) {
        return;
    }
    let fixture = TestWorkspace::create("atomic-mode-create").unwrap();
    let root = fixture.root().unwrap();
    let sandbox = FilesystemSandbox::new(FilesystemPolicy::allow_workspace(root.clone()));
    for (mask, expected) in [(0o022, 0o644), (0o077, 0o600), (0o777, 0)] {
        unsafe {
            libc::umask(mask);
        }
        let name = format!("new-{mask:o}");
        sandbox
            .atomic_write(&root, Path::new(&name), b"new")
            .unwrap();
        assert_eq!(mode(&fixture.path().join(name)), expected);
    }
}

#[test]
fn atomic_replace_sets_final_mode_before_publication_and_keeps_temp_private() {
    let fixture = TestWorkspace::create("atomic-mode-publish").unwrap();
    let path = fixture.path().join("data");
    fs::write(&path, b"old").unwrap();
    set_mode(&path, 0o700);
    let root = fixture.root().unwrap();
    let directory = fixture.path().to_path_buf();
    ATOMIC_OBSERVER.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move |stage| {
            if stage != "pre-write" && stage != "pre-rename" {
                return;
            }
            let temp = fs::read_dir(&directory)
                .unwrap()
                .map(Result::unwrap)
                .find(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".etas-tmp-")
                })
                .unwrap()
                .path();
            let actual = mode(&temp);
            if stage == "pre-write" {
                assert_eq!(actual & !0o600, 0, "temporary mode {actual:o}");
            } else {
                assert_eq!(actual, 0o700);
            }
            assert_eq!(fs::read(directory.join("data")).unwrap(), b"old");
        }))
    });
    FilesystemSandbox::new(FilesystemPolicy::allow_workspace(root.clone()))
        .atomic_write(&root, Path::new("data"), b"new")
        .unwrap();
    ATOMIC_OBSERVER.with(|slot| slot.borrow_mut().take());
    assert_eq!(mode(&path), 0o700);
    assert_eq!(fs::read(&path).unwrap(), b"new");
}

#[test]
fn atomic_replace_does_not_copy_special_unix_bits() {
    let fixture = TestWorkspace::create("atomic-mode-special").unwrap();
    let path = fixture.path().join("data");
    fs::write(&path, b"old").unwrap();
    set_mode(&path, 0o6751);
    let root = fixture.root().unwrap();
    FilesystemSandbox::new(FilesystemPolicy::allow_workspace(root.clone()))
        .atomic_write(&root, Path::new("data"), b"new")
        .unwrap();
    assert_eq!(mode(&path), 0o751);
}

#[test]
fn atomic_replace_rejects_symlink_and_directory_without_changing_the_target() {
    let fixture = TestWorkspace::create("atomic-mode-symlink").unwrap();
    let outside = TestWorkspace::create("atomic-mode-outside").unwrap();
    fs::write(outside.path().join("data"), b"outside").unwrap();
    symlink(outside.path().join("data"), fixture.path().join("link")).unwrap();
    fs::create_dir(fixture.path().join("dir")).unwrap();
    let root = fixture.root().unwrap();
    let sandbox = FilesystemSandbox::new(FilesystemPolicy::allow_workspace(root.clone()));
    for name in ["link", "dir"] {
        assert!(
            sandbox
                .atomic_write(&root, Path::new(name), b"new")
                .is_err()
        );
    }
    assert!(
        fs::symlink_metadata(fixture.path().join("link"))
            .unwrap()
            .is_symlink()
    );
    assert_eq!(fs::read(outside.path().join("data")).unwrap(), b"outside");
    assert_eq!(fs::read_dir(fixture.path()).unwrap().count(), 2);
}

#[test]
fn atomic_replace_permission_failure_preserves_original_and_removes_temp() {
    let fixture = TestWorkspace::create("atomic-mode-failure").unwrap();
    let path = fixture.path().join("data");
    fs::write(&path, b"old").unwrap();
    set_mode(&path, 0o600);
    let root = fixture.root().unwrap();
    ATOMIC_FAILURE.with(|slot| *slot.borrow_mut() = Some("pre-permissions"));
    let error = FilesystemSandbox::new(FilesystemPolicy::allow_workspace(root.clone()))
        .atomic_write(&root, Path::new("data"), b"new")
        .unwrap_err();
    assert!(
        error
            .details
            .iter()
            .any(|d| d.key == "publication" && d.value == "unchanged")
    );
    assert_eq!(mode(&path), 0o600);
    assert_eq!(fs::read(&path).unwrap(), b"old");
    assert_eq!(fs::read_dir(fixture.path()).unwrap().count(), 1);
}

#[test]
fn atomic_replace_preserves_mode_in_the_retained_parent_not_an_outside_replacement() {
    let fixture = TestWorkspace::create("atomic-mode-parent-race").unwrap();
    let outside = TestWorkspace::create("atomic-mode-parent-outside").unwrap();
    fs::create_dir(fixture.path().join("parent")).unwrap();
    fs::write(fixture.path().join("parent/data"), b"old").unwrap();
    set_mode(&fixture.path().join("parent/data"), 0o700);
    fs::write(outside.path().join("data"), b"outside").unwrap();
    set_mode(&outside.path().join("data"), 0o600);
    let root = fixture.root().unwrap();
    super::replace_parent_after_resolution(&fixture, &outside);
    FilesystemSandbox::new(FilesystemPolicy::allow_workspace(root.clone()))
        .atomic_write(&root, Path::new("parent/data"), b"new")
        .unwrap();
    assert_eq!(mode(&fixture.path().join("retained/data")), 0o700);
    assert_eq!(
        fs::read(fixture.path().join("retained/data")).unwrap(),
        b"new"
    );
    assert_eq!(mode(&outside.path().join("data")), 0o600);
    assert_eq!(fs::read(outside.path().join("data")).unwrap(), b"outside");
}
