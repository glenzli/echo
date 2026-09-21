use super::*;
use std::{fs, path::PathBuf};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("echo-recovery-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn database(&self) -> Connection {
        Connection::open(self.0.join("catalog.sqlite")).unwrap()
    }
    fn legacy_private(&self) {
        self.database()
            .execute_batch(
                "CREATE TABLE catalog_meta(key TEXT PRIMARY KEY,value TEXT);
             INSERT INTO catalog_meta VALUES('session_kind','independent-editor-v1');
             INSERT INTO catalog_meta VALUES('schema_version','legacy');
             CREATE TABLE assets(id TEXT,path TEXT,imported_at_millis INTEGER);",
            )
            .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn empty_private_sessions_are_hidden_and_foreign_or_missing_stores_are_untouched() {
    let fixture = Fixture::new();
    assert!(summary_json(&fixture.0).is_err());
    assert!(!fixture.0.join("catalog.sqlite").exists());
    fixture.legacy_private();
    assert_eq!(summary_json(&fixture.0).unwrap(), "");
    fixture
        .database()
        .execute("DELETE FROM catalog_meta WHERE key='session_kind'", [])
        .unwrap();
    let before = fs::read(fixture.0.join("catalog.sqlite")).unwrap();
    assert!(summary_json(&fixture.0).is_err());
    assert_eq!(before, fs::read(fixture.0.join("catalog.sqlite")).unwrap());
}

#[test]
fn legacy_projects_are_described_without_migration_or_mutation() {
    let fixture = Fixture::new();
    fixture.legacy_private();
    fixture
        .database()
        .execute_batch(
            "INSERT INTO assets VALUES('b','media/hash/第二段.wav',2);
         INSERT INTO assets VALUES('a','media/hash/雨声.wav',1);",
        )
        .unwrap();
    let before = fs::read(fixture.0.join("catalog.sqlite")).unwrap();
    for _ in 0..3 {
        let summary: serde_json::Value =
            serde_json::from_str(&summary_json(&fixture.0).unwrap()).unwrap();
        assert_eq!(summary["title"], "雨声.wav");
        assert_eq!(summary["sourceCount"], 2);
    }
    assert_eq!(before, fs::read(fixture.0.join("catalog.sqlite")).unwrap());
}

#[test]
fn latest_assembly_title_is_preferred_and_bounded() {
    let fixture = Fixture::new();
    fixture.legacy_private();
    let connection = fixture.database();
    connection
        .execute_batch(
            "INSERT INTO assets VALUES('a','media/hash/雨声.wav',1);
         CREATE TABLE sound_assembly_revisions(id INTEGER,name TEXT);
         INSERT INTO sound_assembly_revisions VALUES(1,'Old mix');",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO sound_assembly_revisions VALUES(2,?1)",
            ["旅行".repeat(200)],
        )
        .unwrap();
    drop(connection);
    let summary: serde_json::Value =
        serde_json::from_str(&summary_json(&fixture.0).unwrap()).unwrap();
    assert_eq!(summary["title"], "旅行".repeat(120));
}
