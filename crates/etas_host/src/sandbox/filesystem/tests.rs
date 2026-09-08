use super::*;
use crate::TestWorkspace;
use std::cell::RefCell;
use std::fs;

thread_local! {
    static AFTER_RESOLUTION: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}

pub(super) fn after_resolution() {
    let hook = AFTER_RESOLUTION.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(unix)]
#[test]
fn read_uses_opened_file_after_parent_replaced_by_outside_symlink() {
    let fixture = TestWorkspace::create("read-parent-race").unwrap();
    let outside = TestWorkspace::create("read-parent-outside").unwrap();
    fs::create_dir(fixture.path().join("parent")).unwrap();
    fs::write(fixture.path().join("parent/data"), b"authorized").unwrap();
    fs::write(outside.path().join("data"), b"outside").unwrap();
    let root = fixture.root().unwrap();
    let sandbox = FilesystemSandbox::new(FilesystemPolicy::allow_workspace(root.clone()));
    let parent = fixture.path().join("parent");
    let retained = fixture.path().join("retained");
    let target = outside.path().to_path_buf();
    AFTER_RESOLUTION.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            fs::rename(&parent, retained).unwrap();
            std::os::unix::fs::symlink(target, parent).unwrap();
        }));
    });
    assert_eq!(
        sandbox.read_file(&root, Path::new("parent/data")).unwrap(),
        b"authorized"
    );
}

#[cfg(unix)]
fn replace_parent_after_resolution(fixture: &TestWorkspace, outside: &TestWorkspace) {
    let parent = fixture.path().join("parent");
    let retained = fixture.path().join("retained");
    let target = outside.path().to_path_buf();
    AFTER_RESOLUTION.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            fs::rename(&parent, retained).unwrap();
            std::os::unix::fs::symlink(target, parent).unwrap();
        }))
    });
}

#[cfg(unix)]
#[test]
fn atomic_write_uses_retained_parent_after_replacement() {
    let fixture = TestWorkspace::create("write-parent-race").unwrap();
    let outside = TestWorkspace::create("write-parent-outside").unwrap();
    fs::create_dir(fixture.path().join("parent")).unwrap();
    fs::write(outside.path().join("data"), b"outside").unwrap();
    let root = fixture.root().unwrap();
    let sandbox = FilesystemSandbox::new(FilesystemPolicy::allow_workspace(root.clone()));
    replace_parent_after_resolution(&fixture, &outside);
    sandbox
        .atomic_write(&root, Path::new("parent/data"), b"authorized")
        .unwrap();
    assert_eq!(
        fs::read(fixture.path().join("retained/data")).unwrap(),
        b"authorized"
    );
    assert_eq!(fs::read(outside.path().join("data")).unwrap(), b"outside");
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 1);
    assert_eq!(
        fs::read_dir(fixture.path().join("retained"))
            .unwrap()
            .count(),
        1
    );
}

#[cfg(unix)]
#[test]
fn delete_uses_retained_parent_after_replacement() {
    let fixture = TestWorkspace::create("delete-parent-race").unwrap();
    let outside = TestWorkspace::create("delete-parent-outside").unwrap();
    fs::create_dir(fixture.path().join("parent")).unwrap();
    fs::write(fixture.path().join("parent/data"), b"authorized").unwrap();
    fs::write(outside.path().join("data"), b"outside").unwrap();
    let root = fixture.root().unwrap();
    let sandbox =
        FilesystemSandbox::new(FilesystemPolicy::allow_destructive_workspace(root.clone()));
    replace_parent_after_resolution(&fixture, &outside);
    sandbox
        .delete_file(&root, Path::new("parent/data"))
        .unwrap();
    assert!(!fixture.path().join("retained/data").exists());
    assert_eq!(fs::read(outside.path().join("data")).unwrap(), b"outside");
}

#[cfg(unix)]
#[test]
fn stat_uses_opened_object_after_leaf_replacement() {
    let fixture = TestWorkspace::create("stat-leaf-race").unwrap();
    let outside = TestWorkspace::create("stat-leaf-outside").unwrap();
    let path = fixture.path().join("data");
    let target = outside.path().join("data");
    fs::write(&path, b"authorized").unwrap();
    fs::write(&target, b"outside").unwrap();
    let root = fixture.root().unwrap();
    let sandbox = FilesystemSandbox::new(FilesystemPolicy::allow_workspace(root.clone()));
    AFTER_RESOLUTION.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            fs::remove_file(&path).unwrap();
            std::os::unix::fs::symlink(target, path).unwrap();
        }))
    });
    assert_eq!(sandbox.stat(&root, Path::new("data")).unwrap().len, 10);
}

#[cfg(unix)]
#[test]
fn list_uses_opened_directory_after_replacement() {
    let fixture = TestWorkspace::create("list-parent-race").unwrap();
    let outside = TestWorkspace::create("list-parent-outside").unwrap();
    fs::create_dir(fixture.path().join("parent")).unwrap();
    fs::write(fixture.path().join("parent/authorized"), b"ok").unwrap();
    fs::write(outside.path().join("secret"), b"outside").unwrap();
    let root = fixture.root().unwrap();
    let sandbox = FilesystemSandbox::new(FilesystemPolicy::allow_workspace(root.clone()));
    replace_parent_after_resolution(&fixture, &outside);
    assert_eq!(
        sandbox.read_dir(&root, Path::new("parent")).unwrap(),
        ["authorized"]
    );
}

#[cfg(unix)]
#[test]
fn mkdir_rejects_parent_replaced_between_create_and_open() {
    let fixture = TestWorkspace::create("mkdir-parent-race").unwrap();
    let outside = TestWorkspace::create("mkdir-parent-outside").unwrap();
    let root = fixture.root().unwrap();
    let sandbox = FilesystemSandbox::new(FilesystemPolicy::allow_workspace(root.clone()));
    replace_parent_after_resolution(&fixture, &outside);
    assert!(
        sandbox
            .create_dir_all(&root, Path::new("parent/new/nested"))
            .is_err()
    );
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn deleting_replaced_leaf_unlinks_symlink_not_its_target() {
    let fixture = TestWorkspace::create("delete-leaf-race").unwrap();
    let outside = TestWorkspace::create("delete-leaf-outside").unwrap();
    let path = fixture.path().join("data");
    let target = outside.path().join("data");
    fs::write(&path, b"authorized").unwrap();
    fs::write(&target, b"outside").unwrap();
    let root = fixture.root().unwrap();
    let sandbox =
        FilesystemSandbox::new(FilesystemPolicy::allow_destructive_workspace(root.clone()));
    AFTER_RESOLUTION.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            fs::remove_file(&path).unwrap();
            std::os::unix::fs::symlink(target, path).unwrap();
        }))
    });
    sandbox.delete_file(&root, Path::new("data")).unwrap();
    assert_eq!(fs::read(outside.path().join("data")).unwrap(), b"outside");
}
