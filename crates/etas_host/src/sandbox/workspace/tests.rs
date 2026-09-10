use super::*;
use crate::{FilesystemPolicy, FilesystemSandbox, HostErrorCode, TestWorkspace};
use std::{fs, path::Path};

#[test]
fn reopening_same_directory_does_not_inherit_binding_authority() {
    let fixture = TestWorkspace::create("workspace-reopen").unwrap();
    fs::write(fixture.path().join("data"), b"original").unwrap();
    let root = fixture.root().unwrap();
    let reopened = fixture.root().unwrap();
    assert_eq!(root, root.clone());
    assert_ne!(root, reopened);
    let sandbox = FilesystemSandbox::new(FilesystemPolicy::allow_workspace(root.clone()));
    assert_eq!(
        sandbox.read_file(&root.clone(), Path::new("data")).unwrap(),
        b"original"
    );
    assert_eq!(
        sandbox
            .read_file(&reopened, Path::new("data"))
            .unwrap_err()
            .code,
        HostErrorCode::AuthorityDenied
    );
}

#[test]
fn duplicate_region_registration_preserves_original_root_and_grants() {
    let first = TestWorkspace::create("workspace-first").unwrap();
    let other = TestWorkspace::create("workspace-other").unwrap();
    fs::write(first.path().join("data"), b"first").unwrap();
    fs::write(other.path().join("data"), b"other").unwrap();
    let root = first.root().unwrap();
    let mut registry = WorkspaceRegionRegistry::default();
    let region = WorkspaceRegionId::new("app.Root").unwrap();
    registry.insert(region.clone(), root.clone()).unwrap();
    assert_eq!(
        registry
            .insert(region.clone(), other.root().unwrap())
            .unwrap_err()
            .code,
        HostErrorCode::InvalidRequest
    );
    let bound = registry
        .bind(&WorkspacePathRef::new(region, "data").unwrap())
        .unwrap();
    assert_eq!(bound.root, root);
    let sandbox = FilesystemSandbox::new(FilesystemPolicy {
        read_roots: vec![root],
        write_roots: vec![],
        delete_roots: vec![],
    });
    assert_eq!(
        sandbox.read_file(&bound.root, &bound.relative).unwrap(),
        b"first"
    );
    assert_eq!(
        sandbox
            .atomic_write(&bound.root, &bound.relative, b"no")
            .unwrap_err()
            .code,
        HostErrorCode::AuthorityDenied
    );
}

#[cfg(unix)]
#[test]
fn root_rename_replacement_retains_opened_object_and_reopen_requires_grant() {
    let fixture = TestWorkspace::create("workspace-rename").unwrap();
    let path = fixture.path().join("root");
    fs::create_dir(&path).unwrap();
    fs::write(path.join("data"), b"A").unwrap();
    let old = WorkspaceRoot::new(&path).unwrap();
    fs::rename(&path, fixture.path().join("retained")).unwrap();
    fs::create_dir(&path).unwrap();
    fs::write(path.join("data"), b"B").unwrap();
    let new = WorkspaceRoot::new(&path).unwrap();
    let sandbox = FilesystemSandbox::new(FilesystemPolicy::allow_workspace(old.clone()));
    assert_eq!(sandbox.read_file(&old, Path::new("data")).unwrap(), b"A");
    assert_eq!(
        sandbox.read_file(&new, Path::new("data")).unwrap_err().code,
        HostErrorCode::AuthorityDenied
    );
    let fresh_policy = FilesystemSandbox::new(FilesystemPolicy::allow_workspace(new.clone()));
    assert_eq!(
        fresh_policy.read_file(&new, Path::new("data")).unwrap(),
        b"B"
    );
    fs::write(fixture.path().join("retained/data"), b"changed").unwrap();
    assert_eq!(
        sandbox.read_file(&old, Path::new("data")).unwrap(),
        b"changed",
        "binding is not a content snapshot"
    );
}

#[test]
fn child_binding_requires_registration_and_does_not_depend_on_parent_path() {
    let fixture = TestWorkspace::create("workspace-child").unwrap();
    fs::create_dir(fixture.path().join("child")).unwrap();
    let parent = fixture.root().unwrap();
    let child = parent.open_child("child").unwrap();
    let mut registry = WorkspaceRegionRegistry::default();
    registry
        .insert(WorkspaceRegionId::new("app.Parent").unwrap(), parent)
        .unwrap();
    let reference =
        WorkspacePathRef::new(WorkspaceRegionId::new("app.Child").unwrap(), "data").unwrap();
    assert_eq!(
        registry.bind(&reference).unwrap_err().code,
        HostErrorCode::AuthorityDenied
    );
    registry
        .insert(reference.region.clone(), child.clone())
        .unwrap();
    drop(registry);
    let sandbox = FilesystemSandbox::new(FilesystemPolicy::allow_workspace(child.clone()));
    sandbox
        .atomic_write(&child, Path::new("data"), b"alive")
        .unwrap();
    assert_eq!(
        fs::read(fixture.path().join("child/data")).unwrap(),
        b"alive"
    );
}

#[test]
fn foreign_platform_path_spellings_are_rejected_on_every_platform() {
    for input in [
        r"C:\private\data",
        "C:data",
        r"\\server\share",
        r"..\data",
        "/etc/data",
        "../data",
        "",
        ".",
    ] {
        assert!(
            normalize_relative(Path::new(input)).is_err(),
            "accepted {input:?}"
        );
    }
}

#[test]
fn workspace_region_identity_uses_canonical_language_identifiers() {
    assert!(WorkspaceRegionId::new("app.workspace.ProjectRoot").is_ok());
    assert!(WorkspaceRegionId::new("app._private.Root2").is_ok());

    for invalid in ["", ".app.Root", "app..Root", "app.1Root", "app.Root-name"] {
        assert!(
            WorkspaceRegionId::new(invalid).is_err(),
            "`{invalid}` must not be accepted as a canonical region identity"
        );
    }
}

#[test]
fn workspace_path_ref_rejects_absolute_and_parent_paths() {
    let region = WorkspaceRegionId::new("app.workspace.ProjectRoot").expect("valid region");
    assert!(WorkspacePathRef::new(region.clone(), "src/main.es").is_ok());
    assert!(WorkspacePathRef::new(region.clone(), "../secret").is_err());
    assert!(WorkspacePathRef::new(region, "/tmp/secret").is_err());
}
