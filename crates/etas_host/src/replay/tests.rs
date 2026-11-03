use super::*;
use crate::{HostTraceFieldSensitivity, HostTracePayload, HostValue};
use std::num::NonZeroUsize;

fn payload(reason: &str) -> HostTracePayload {
    HostTracePayload::new("approval", "Approval.request").with_field(
        "reason",
        HostValue::String(reason.into()),
        HostTraceFieldSensitivity::Sensitive,
    )
}

#[test]
fn replay_keys_bind_canonical_requests_without_exporting_material() {
    let key = ReplayKey::generate().unwrap();
    let other = ReplayKey::generate().unwrap();
    let request = payload("sensitive-reason-marker");
    let fingerprint = key.fingerprint(&request).unwrap();
    assert_eq!(fingerprint, key.fingerprint(&request).unwrap());
    assert_ne!(
        fingerprint,
        key.fingerprint(&payload("another reason")).unwrap()
    );
    assert_ne!(fingerprint, other.fingerprint(&request).unwrap());
    assert_ne!(key.id(), other.id());
    assert_eq!(key.id(), key.id().to_hex().parse().unwrap());
    assert_eq!(fingerprint, fingerprint.to_hex().parse().unwrap());
    for public in [
        format!("{key:?}"),
        format!("{fingerprint:?}"),
        key.id().to_hex(),
    ] {
        assert!(!public.contains("sensitive-reason-marker"));
        assert!(!public.contains(&blake3::Hash::from(*key.material).to_hex().to_string()));
    }
    for bad in ["", "key-1", "../key", &"A".repeat(64), &"0".repeat(63)] {
        assert!(bad.parse::<ReplayKeyId>().is_err());
        assert!(bad.parse::<ReplayFingerprint>().is_err());
    }
}

#[test]
fn memory_key_provider_does_not_replace_missing_or_removed_keys() {
    let provider = MemoryReplayKeyProvider::new(NonZeroUsize::MIN);
    let key = provider.create().unwrap();
    assert!(provider.resolve(key.id()).is_err());
    provider.retain(&key).unwrap();
    provider.retain(&key).unwrap();
    assert_eq!(provider.resolve(key.id()).unwrap().id(), key.id());
    assert_eq!(
        provider
            .retain(&provider.create().unwrap())
            .unwrap_err()
            .kind,
        ReplayKeyErrorKind::Capacity
    );
    assert_eq!(provider.resolve(key.id()).unwrap().id(), key.id());
    assert!(provider.remove(key.id()).unwrap());
    assert_eq!(
        provider.resolve(key.id()).unwrap_err().kind,
        ReplayKeyErrorKind::Missing
    );
    let replacement = provider.create().unwrap();
    assert_ne!(replacement.id(), key.id());
    assert_eq!(
        provider.resolve(key.id()).unwrap_err().kind,
        ReplayKeyErrorKind::Missing
    );
}

#[test]
fn replay_request_validation_does_not_turn_bad_metadata_into_an_empty_digest() {
    let key = ReplayKey::generate().unwrap();
    let duplicate = payload("one").with_field(
        "reason",
        HostValue::Bool(true),
        HostTraceFieldSensitivity::Public,
    );
    assert_eq!(
        key.fingerprint(&duplicate).unwrap_err().kind,
        ReplayKeyErrorKind::Invalid
    );
    assert!(key.fingerprint(&HostTracePayload::new("", "")).is_err());
}

#[cfg(unix)]
#[test]
fn private_directory_keys_roundtrip_and_reject_corruption_and_loose_permissions() {
    use std::{fs, os::unix::fs::PermissionsExt};
    let fixture = crate::TestWorkspace::create("private-replay-keys").unwrap();
    let path = fixture.path().join("keys");
    let provider = DirectoryReplayKeyProvider::open(&path).unwrap();
    let key = provider.create().unwrap();
    assert_eq!(fs::read_dir(&path).unwrap().count(), 0);
    provider.retain(&key).unwrap();
    provider.retain(&key).unwrap();
    let expected = key.fingerprint(&payload("private reason")).unwrap();
    drop(provider);
    let provider = DirectoryReplayKeyProvider::open(&path).unwrap();
    assert_eq!(
        provider
            .resolve(key.id())
            .unwrap()
            .fingerprint(&payload("private reason"))
            .unwrap(),
        expected
    );
    let file = path.join(key.id().to_hex());
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
    assert_eq!(fs::metadata(&file).unwrap().permissions().mode() & 0o077, 0);
    fs::set_permissions(&file, fs::Permissions::from_mode(0o400)).unwrap();
    assert_eq!(provider.resolve(key.id()).unwrap().id(), key.id());
    fs::set_permissions(&file, fs::Permissions::from_mode(0o200)).unwrap();
    assert!(provider.resolve(key.id()).is_err());
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(provider.resolve(key.id()).is_err());
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&file, b"invalid").unwrap();
    assert!(provider.resolve(key.id()).is_err());
    fs::remove_file(&file).unwrap();
    assert_eq!(
        provider.resolve(key.id()).unwrap_err().kind,
        ReplayKeyErrorKind::Missing
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(DirectoryReplayKeyProvider::open(&path).is_err());
}

#[cfg(unix)]
#[test]
fn private_directory_store_rejects_symlinks_and_stays_on_its_opened_root() {
    use std::{fs, os::unix::fs::symlink};
    let fixture = crate::TestWorkspace::create("replay-key-root").unwrap();
    let path = fixture.path().join("keys");
    let retained = fixture.path().join("retained");
    let provider = DirectoryReplayKeyProvider::open(&path).unwrap();
    let key = provider.create().unwrap();
    provider.retain(&key).unwrap();
    fs::rename(&path, &retained).unwrap();
    let replacement = DirectoryReplayKeyProvider::open(&path).unwrap();
    assert_eq!(provider.resolve(key.id()).unwrap().id(), key.id());
    assert_eq!(
        replacement.resolve(key.id()).unwrap_err().kind,
        ReplayKeyErrorKind::Missing
    );
    let alias = fixture.path().join("alias");
    symlink(&retained, &alias).unwrap();
    assert!(DirectoryReplayKeyProvider::open(&alias).is_err());
    symlink(
        retained.join(key.id().to_hex()),
        path.join(key.id().to_hex()),
    )
    .unwrap();
    assert!(replacement.resolve(key.id()).is_err());
}

#[cfg(unix)]
#[test]
fn private_key_store_rejects_program_authority_by_directory_identity() {
    let fixture = crate::TestWorkspace::create("replay-key-authority").unwrap();
    let path = fixture.path().join("keys");
    let provider = DirectoryReplayKeyProvider::open(&path).unwrap();
    let mut authority = crate::AuthorityContext::deny_all();
    provider.validate_authority(&authority).unwrap();
    authority
        .sandbox
        .filesystem
        .read_roots
        .push(crate::WorkspaceRoot::new(fixture.path()).unwrap());
    assert!(provider.validate_authority(&authority).is_err());
    authority.sandbox.filesystem.read_roots.clear();
    authority
        .sandbox
        .filesystem
        .write_roots
        .push(crate::WorkspaceRoot::new(&path).unwrap());
    assert!(provider.validate_authority(&authority).is_err());
}
