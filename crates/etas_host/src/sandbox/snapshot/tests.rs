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
    let stage = WorkspaceStage::create(&fixture.root().unwrap()).unwrap();
    let original = stage.root().display_path().to_path_buf();
    fs::write(original.join("data"), b"before").unwrap();
    fs::write(outside.path().join("data"), b"outside").unwrap();
    let snapshot = stage.snapshot().unwrap();
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
    let stage = WorkspaceStage::create(&fixture.root().unwrap()).unwrap();
    let stage_path = stage.root().display_path().to_path_buf();
    let parent = stage_path.join("parent");
    fs::create_dir(&parent).unwrap();
    fs::write(parent.join("data"), b"before").unwrap();
    fs::write(outside.path().join("data"), b"outside").unwrap();
    let snapshot = stage.snapshot().unwrap();
    fs::write(parent.join("data"), b"after").unwrap();
    let retained = stage_path.join("retained");
    let target = outside.path().to_path_buf();
    AFTER_PARENT.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            fs::rename(&parent, retained).unwrap();
            std::os::unix::fs::symlink(target, parent).unwrap();
        }))
    });
    snapshot.rollback().unwrap();
    assert_eq!(
        fs::read(stage_path.join("retained/data")).unwrap(),
        b"before"
    );
    assert_eq!(fs::read(outside.path().join("data")).unwrap(), b"outside");
}

#[test]
fn rollback_restores_deleted_nested_directories_before_files() {
    let fixture = TestWorkspace::create("rollback-nested").unwrap();
    let stage = WorkspaceStage::create(&fixture.root().unwrap()).unwrap();
    let path = stage.root().display_path();
    fs::create_dir_all(path.join("one/two")).unwrap();
    fs::write(path.join("one/two/data"), b"before").unwrap();
    let snapshot = stage.snapshot().unwrap();
    fs::remove_dir_all(path.join("one")).unwrap();
    snapshot.rollback().unwrap();
    assert_eq!(fs::read(path.join("one/two/data")).unwrap(), b"before");
}

#[test]
fn rollback_only_changes_staging_and_keeps_external_live_edits() {
    let live = TestWorkspace::create("live-not-a-snapshot").unwrap();
    let staging = TestWorkspace::create("staging-parent").unwrap();
    fs::write(live.path().join("data"), "initial").unwrap();
    let observed = WorkspaceSnapshot::capture(live.root().unwrap()).unwrap();
    let stage = WorkspaceStage::create(&staging.root().unwrap()).unwrap();
    let snapshot = stage.snapshot().unwrap();
    fs::write(
        stage.root().display_path().join("unpublished"),
        "stage edit",
    )
    .unwrap();
    fs::write(live.path().join("data"), "external edit").unwrap();
    snapshot.rollback().unwrap();
    assert_eq!(
        fs::read_to_string(live.path().join("data")).unwrap(),
        "external edit"
    );
    assert_eq!(observed.diff_current().unwrap().entries.len(), 1);
    assert!(stage.root().directory().entries().unwrap().next().is_none());
}
