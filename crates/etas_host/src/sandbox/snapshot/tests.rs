use super::*;
use crate::TestWorkspace;
use std::{cell::RefCell, fs};

thread_local! {
    static AFTER_RESOLUTION: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
    static AFTER_PARENT: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}

pub(super) fn after_parent_resolution() {
    let hook = AFTER_PARENT.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

pub(super) fn after_resolution() {
    let hook = AFTER_RESOLUTION.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(unix)]
#[test]
fn snapshot_rejects_file_replaced_by_symlink_after_entry_inspection() {
    let fixture = TestWorkspace::create("snapshot-file-race").unwrap();
    let outside = TestWorkspace::create("snapshot-file-outside").unwrap();
    let path = fixture.path().join("data");
    fs::write(&path, b"authorized").unwrap();
    let target = outside.path().join("secret");
    fs::write(&target, b"outside").unwrap();
    AFTER_RESOLUTION.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            fs::remove_file(&path).unwrap();
            std::os::unix::fs::symlink(target, path).unwrap();
        }))
    });
    assert!(WorkspaceSnapshot::capture(fixture.root().unwrap()).is_err());
}

#[cfg(unix)]
#[test]
fn rollback_keeps_root_capability_after_root_path_replacement() {
    let fixture = TestWorkspace::create("rollback-root-race").unwrap();
    let outside = TestWorkspace::create("rollback-root-outside").unwrap();
    let original = fixture.path().join("root");
    fs::create_dir(&original).unwrap();
    fs::write(original.join("data"), b"before").unwrap();
    fs::write(outside.path().join("data"), b"outside").unwrap();
    let snapshot = WorkspaceSnapshot::capture(WorkspaceRoot::new(&original).unwrap()).unwrap();
    fs::write(original.join("data"), b"after").unwrap();
    fs::rename(&original, fixture.path().join("retained")).unwrap();
    std::os::unix::fs::symlink(outside.path(), &original).unwrap();
    snapshot.rollback().unwrap();
    assert_eq!(
        fs::read(fixture.path().join("retained/data")).unwrap(),
        b"before"
    );
    assert_eq!(fs::read(outside.path().join("data")).unwrap(), b"outside");
}

#[cfg(unix)]
#[test]
fn rollback_does_not_write_outside_after_parent_replacement() {
    let fixture = TestWorkspace::create("rollback-parent-race").unwrap();
    let outside = TestWorkspace::create("rollback-parent-outside").unwrap();
    let parent = fixture.path().join("parent");
    fs::create_dir(&parent).unwrap();
    fs::write(parent.join("data"), b"before").unwrap();
    fs::write(outside.path().join("data"), b"outside").unwrap();
    let snapshot = WorkspaceSnapshot::capture(fixture.root().unwrap()).unwrap();
    fs::write(parent.join("data"), b"after").unwrap();
    let retained = fixture.path().join("retained");
    let target = outside.path().to_path_buf();
    AFTER_PARENT.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            fs::rename(&parent, retained).unwrap();
            std::os::unix::fs::symlink(target, parent).unwrap();
        }))
    });
    snapshot.rollback().unwrap();
    assert_eq!(
        fs::read(fixture.path().join("retained/data")).unwrap(),
        b"before"
    );
    assert_eq!(fs::read(outside.path().join("data")).unwrap(), b"outside");
}

#[test]
fn rollback_restores_deleted_nested_directories_before_files() {
    let fixture = TestWorkspace::create("rollback-nested").unwrap();
    fs::create_dir_all(fixture.path().join("one/two")).unwrap();
    fs::write(fixture.path().join("one/two/data"), b"before").unwrap();
    let snapshot = WorkspaceSnapshot::capture(fixture.root().unwrap()).unwrap();
    fs::remove_dir_all(fixture.path().join("one")).unwrap();
    snapshot.rollback().unwrap();
    assert_eq!(
        fs::read(fixture.path().join("one/two/data")).unwrap(),
        b"before"
    );
}
