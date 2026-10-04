use cadiswave_runtime::migration::inspect_legacy;
use std::{fs, os::unix::fs::symlink};
#[test]
fn malformed_legacy_state_remains_unchanged() {
    let legacy = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let source = legacy.path().join("sources.json");
    fs::write(&source, b"[unterminated").unwrap();
    fs::write(destination.path().join("sources.json"), b"{\"keep\":true}").unwrap();
    assert!(inspect_legacy(legacy.path(), destination.path()).is_err());
    assert_eq!(fs::read(source).unwrap(), b"[unterminated");
    assert_eq!(
        fs::read(destination.path().join("sources.json")).unwrap(),
        b"{\"keep\":true}"
    );
}
#[test]
fn existing_destination_is_never_replaced() {
    let legacy = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    fs::write(legacy.path().join("sources.json"), b"{}").unwrap();
    let target = destination.path().join("sources.json");
    fs::write(&target, b"{\"keep\":true}").unwrap();
    inspect_legacy(legacy.path(), destination.path())
        .unwrap()
        .apply()
        .unwrap();
    assert_eq!(fs::read(target).unwrap(), b"{\"keep\":true}");
}
#[test]
fn linked_ancestry_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let legacy = root.path().join("legacy");
    fs::create_dir(&legacy).unwrap();
    let linked = root.path().join("linked");
    symlink(&legacy, &linked).unwrap();
    assert!(inspect_legacy(&linked, &root.path().join("new")).is_err());
}
#[test]
fn changed_source_and_destination_race_are_safe() {
    let root = tempfile::tempdir().unwrap();
    let legacy = root.path().join("legacy");
    fs::create_dir(&legacy).unwrap();
    let target = root.path().join("new");
    fs::create_dir(&target).unwrap();
    fs::write(legacy.join("sources.json"), b"{}").unwrap();
    let import = inspect_legacy(&legacy, &target).unwrap();
    fs::write(legacy.join("sources.json"), b"{\"changed\":true}").unwrap();
    assert!(import.apply().is_err());
    assert!(!target.join("sources.json").exists());
    fs::write(legacy.join("sources.json"), b"{}").unwrap();
    let import = inspect_legacy(&legacy, &target).unwrap();
    fs::write(target.join("sources.json"), b"keep").unwrap();
    import.apply().unwrap();
    assert_eq!(fs::read(target.join("sources.json")).unwrap(), b"keep");
}
#[test]
fn validated_import_keeps_source_bytes() {
    let root = tempfile::tempdir().unwrap();
    let legacy = root.path().join("legacy");
    fs::create_dir(&legacy).unwrap();
    let target = root.path().join("new");
    fs::write(legacy.join("ui-state.json"), b"{\"width\":1280}").unwrap();
    inspect_legacy(&legacy, &target).unwrap().apply().unwrap();
    assert_eq!(
        fs::read(target.join("ui-state.json")).unwrap(),
        fs::read(legacy.join("ui-state.json")).unwrap()
    );
}
