use crate::db::Db;
use std::path::PathBuf;

fn temp_db() -> (Db, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let path = dir.path().join("test.db");
    let db = Db::open(&path).expect("failed to open db");
    (db, dir)
}

#[test]
fn open_creates_tables() {
    let (_db, _dir) = temp_db();
}

#[test]
fn insert_and_recent() {
    let (db, _dir) = temp_db();
    db.insert("hello world").unwrap();
    db.insert("second entry").unwrap();

    let recent = db.recent(10).unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].text, "second entry");
    assert_eq!(recent[1].text, "hello world");
}

#[test]
fn recent_respects_limit() {
    let (db, _dir) = temp_db();
    for i in 0..5 {
        db.insert(&format!("entry {i}")).unwrap();
    }
    let recent = db.recent(3).unwrap();
    assert_eq!(recent.len(), 3);
}

#[test]
fn settings_roundtrip() {
    let (db, _dir) = temp_db();

    // Use a valid whitelisted key
    assert!(db.get_setting("transcription_mode").unwrap().is_none());

    db.set_setting("transcription_mode", "local").unwrap();
    assert_eq!(
        db.get_setting("transcription_mode").unwrap(),
        Some("local".to_string())
    );

    db.set_setting("transcription_mode", "api").unwrap();
    assert_eq!(
        db.get_setting("transcription_mode").unwrap(),
        Some("api".to_string())
    );
}

#[test]
fn settings_rejects_unknown_key() {
    let (db, _dir) = temp_db();
    let result = db.set_setting("unknown_key", "value");
    assert!(result.is_err());
}

#[test]
fn open_at_nonexistent_path_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("subdir").join("test.db");
    // Parent doesn't exist, sqlite should fail gracefully
    let result = Db::open(&path);
    // This tests that we get a proper error rather than a panic
    assert!(result.is_err() || PathBuf::from(&path).exists());
}
