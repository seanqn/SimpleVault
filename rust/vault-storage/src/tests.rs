use super::*;
use tempfile::TempDir;

fn key() -> SecretKey {
    SecretKey::from_bytes(&[7; 32]).unwrap()
}
fn credential(id: u8, password: &str) -> Credential {
    Credential {
        content_id: [id; 16],
        org_name: "Example ' org".into(),
        username: "alice".into(),
        password: password.into(),
        email: "alice@example.test".into(),
        note: "Unicode 秘密🔐\nnotes".into(),
    }
}
fn repository() -> (TempDir, SqliteRepository) {
    let dir = TempDir::new().unwrap();
    let repo = SqliteRepository::open(dir.path().join("vault.sqlite"), key()).unwrap();
    (dir, repo)
}
fn legacy(path: &Path) -> Connection {
    let connection = Connection::open(path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection
        .execute_batch(include_str!("legacy_fixture.sql"))
        .unwrap();
    connection.execute("INSERT INTO groups (id, name, created_at) VALUES (?1, 'Legacy group', '2020-01-02 03:04:05')", [[1u8;16].as_slice()]).unwrap();
    for (id, password) in [
        (2u8, Some("old secret🔐")),
        (3, Some("")),
        (4, Some("x")),
        (5, None),
    ] {
        connection
            .execute(
                "INSERT INTO vault_content (group_id,content_id,password) VALUES (?1,?2,?3)",
                params![[1u8; 16].as_slice(), [id; 16].as_slice(), password],
            )
            .unwrap();
    }
    connection
}

#[test]
fn groups_crud_targets_ids_and_reports_missing_records() {
    let (_dir, mut repo) = repository();
    assert!(repo.groups().unwrap().is_empty());
    repo.add_group([1; 16], "first").unwrap();
    repo.add_group([2; 16], "second").unwrap();
    assert!(repo.add_group([1; 16], "duplicate").is_err());
    repo.rename_group([1; 16], "renamed ' ; DROP TABLE groups; --")
        .unwrap();
    let groups = repo.groups().unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "renamed ' ; DROP TABLE groups; --");
    assert_eq!(groups[1].name, "second");
    assert!(!groups[0].created_at.is_empty());
    assert!(matches!(
        repo.rename_group([3; 16], "absent"),
        Err(Error::NotFound)
    ));
    repo.remove_group([1; 16]).unwrap();
    assert_eq!(repo.groups().unwrap()[0].id, [2; 16]);
    assert!(matches!(repo.remove_group([1; 16]), Err(Error::NotFound)));
}

#[test]
fn credential_crud_is_encrypted_persistent_and_group_scoped() {
    let (dir, mut repo) = repository();
    repo.add_group([1; 16], "one").unwrap();
    repo.add_group([2; 16], "two").unwrap();
    let mut row = credential(8, "unique-secret-marker-🔐");
    repo.save_credentials([1; 16], std::slice::from_ref(&row))
        .unwrap();
    let first_nonce: Vec<u8> = repo
        .connection
        .query_row("SELECT password_nonce FROM vault_content", [], |r| r.get(0))
        .unwrap();
    let storage: (String, Vec<u8>, usize) = repo
        .connection
        .query_row(
            "SELECT typeof(content_id),content_id,length(password_ciphertext) FROM vault_content",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(storage.0, "blob");
    assert_eq!(storage.1, [8; 16]);
    assert_eq!(storage.2, row.password.len() + 16);
    assert!(repo.credentials([1; 16]).unwrap()[0] == row);
    assert!(repo.credentials([2; 16]).unwrap().is_empty());
    row.note = "edited".into();
    row.password = "x".into();
    repo.save_credentials([1; 16], std::slice::from_ref(&row))
        .unwrap();
    let second_nonce: Vec<u8> = repo
        .connection
        .query_row("SELECT password_nonce FROM vault_content", [], |r| r.get(0))
        .unwrap();
    assert_ne!(first_nonce, second_nonce);
    drop(repo);
    let bytes = std::fs::read(dir.path().join("vault.sqlite")).unwrap();
    assert!(!bytes
        .windows("unique-secret-marker-🔐".len())
        .any(|w| w == "unique-secret-marker-🔐".as_bytes()));
    let mut repo = SqliteRepository::open(dir.path().join("vault.sqlite"), key()).unwrap();
    assert!(repo.credentials([1; 16]).unwrap()[0] == row);
    assert!(matches!(
        repo.remove_credential([2; 16], [8; 16]),
        Err(Error::NotFound)
    ));
    repo.remove_credential([1; 16], [8; 16]).unwrap();
    assert!(repo.credentials([1; 16]).unwrap().is_empty());
    assert!(matches!(repo.credentials([9; 16]), Err(Error::NotFound)));
}

#[test]
fn batch_conflict_rolls_back_prior_inserts_and_updates() {
    let (_dir, mut repo) = repository();
    repo.add_group([1; 16], "one").unwrap();
    repo.add_group([2; 16], "two").unwrap();
    let original = credential(3, "original");
    repo.save_credentials([1; 16], std::slice::from_ref(&original))
        .unwrap();
    repo.save_credentials([2; 16], &[credential(4, "other")])
        .unwrap();
    assert!(matches!(
        repo.save_credentials(
            [1; 16],
            &[
                credential(3, "changed"),
                credential(5, "new"),
                credential(4, "conflict")
            ]
        ),
        Err(Error::GroupConflict)
    ));
    let rows = repo.credentials([1; 16]).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0] == original);
    assert_eq!(repo.credentials([2; 16]).unwrap()[0].password, "other");
    assert!(matches!(
        repo.save_credentials([9; 16], &[credential(9, "orphan")]),
        Err(Error::NotFound)
    ));
}

#[test]
fn delete_cascades_and_foreign_keys_are_enabled_after_reopen() {
    let (dir, mut repo) = repository();
    repo.add_group([1; 16], "one").unwrap();
    repo.add_group([2; 16], "two").unwrap();
    repo.save_credentials([1; 16], &[credential(3, "one")])
        .unwrap();
    repo.save_credentials([2; 16], &[credential(4, "two")])
        .unwrap();
    drop(repo);
    let mut repo = SqliteRepository::open(dir.path().join("vault.sqlite"), key()).unwrap();
    repo.remove_group([1; 16]).unwrap();
    let count: i64 = repo
        .connection
        .query_row("SELECT count(*) FROM vault_content", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(repo.credentials([2; 16]).unwrap()[0].content_id, [4; 16]);
    assert!(repo
        .connection
        .execute(
            "UPDATE vault_content SET group_id=?1",
            [[9u8; 16].as_slice()]
        )
        .is_err());
}

#[test]
fn wrong_key_and_tampered_password_fail_closed() {
    let (dir, mut repo) = repository();
    repo.add_group([1; 16], "one").unwrap();
    repo.save_credentials([1; 16], &[credential(2, "secret")])
        .unwrap();
    drop(repo);
    assert!(matches!(
        SqliteRepository::open(
            dir.path().join("vault.sqlite"),
            SecretKey::from_bytes(&[8; 32]).unwrap()
        ),
        Err(Error::Crypto)
    ));
    let repo = SqliteRepository::open(dir.path().join("vault.sqlite"), key()).unwrap();
    repo.connection
        .execute(
            "UPDATE vault_content SET password_ciphertext=zeroblob(22)",
            [],
        )
        .unwrap();
    assert!(matches!(repo.credentials([1; 16]), Err(Error::Crypto)));
}

#[test]
fn legacy_migration_preserves_values_and_backup_including_wal() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("old.sqlite");
    let backup = dir.path().join("backup.sqlite");
    let legacy = legacy(&path);
    legacy.pragma_update(None, "journal_mode", "WAL").unwrap();
    legacy
        .execute("UPDATE groups SET name='committed in WAL'", [])
        .unwrap();
    assert!(matches!(
        SqliteRepository::open(&path, key()),
        Err(Error::MigrationRequired)
    ));
    let mut repo = SqliteRepository::migrate_legacy(&path, &backup, key()).unwrap();
    assert_eq!(repo.groups().unwrap()[0].name, "committed in WAL");
    assert_eq!(repo.groups().unwrap()[0].created_at, "2020-01-02 03:04:05");
    let rows = repo.credentials([1; 16]).unwrap();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].password, "old secret🔐");
    assert_eq!(rows[1].password, "");
    assert_eq!(rows[2].password, "x");
    assert_eq!(rows[3].password, "");
    assert!(rows
        .iter()
        .all(|r| r.org_name.is_empty() && r.note.is_empty()));
    let saved = Connection::open(&backup).unwrap();
    assert_eq!(migration::version(&saved).unwrap(), 0);
    assert_eq!(
        saved
            .query_row("SELECT name FROM groups", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "committed in WAL"
    );
    assert_eq!(
        saved
            .query_row(
                "SELECT password FROM vault_content WHERE content_id=?1",
                [[2u8; 16].as_slice()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "old secret🔐"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&backup).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    drop(legacy);
    drop(repo);
    repo = SqliteRepository::open(&path, key()).unwrap();
    assert_eq!(repo.credentials([1; 16]).unwrap().len(), 4);
    repo.remove_group([1; 16]).unwrap();
    assert_eq!(
        repo.connection
            .query_row("SELECT count(*) FROM vault_content", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(matches!(
        SqliteRepository::migrate_legacy(&path, dir.path().join("again"), key()),
        Err(Error::UnsupportedSchema)
    ));
    assert!(!dir.path().join("again").exists());
}

#[test]
fn migration_failure_rolls_back_and_retains_recoverable_backup() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("old.sqlite");
    let backup = dir.path().join("backup.sqlite");
    let source = legacy(&path);
    source
        .execute(
            "UPDATE vault_content SET group_id=?1 WHERE content_id=?2",
            params![[9u8; 16].as_slice(), [5u8; 16].as_slice()],
        )
        .unwrap();
    assert!(SqliteRepository::migrate_legacy(&path, &backup, key()).is_err());
    assert!(backup.exists());
    assert_eq!(migration::version(&source).unwrap(), 0);
    assert_eq!(
        source
            .query_row("SELECT count(*) FROM vault_content", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        source
            .query_row(
                "SELECT password FROM vault_content WHERE content_id=?1",
                [[2u8; 16].as_slice()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "old secret🔐"
    );
    let restored = SqliteRepository::open(&backup, key());
    assert!(matches!(restored, Err(Error::MigrationRequired)));
    assert!(source.prepare("SELECT * FROM vault_metadata").is_err());
}

#[test]
fn refuses_backup_overwrite_and_unknown_schema_without_changes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("old.sqlite");
    let backup = dir.path().join("backup.sqlite");
    let source = legacy(&path);
    std::fs::write(&backup, b"keep me").unwrap();
    assert!(matches!(
        SqliteRepository::migrate_legacy(&path, &backup, key()),
        Err(Error::Io(_))
    ));
    assert_eq!(std::fs::read(&backup).unwrap(), b"keep me");
    assert_eq!(migration::version(&source).unwrap(), 0);
    assert!(SqliteRepository::migrate_legacy(&path, &path, key()).is_err());
    source.pragma_update(None, "user_version", 99).unwrap();
    assert!(matches!(
        SqliteRepository::open(&path, key()),
        Err(Error::UnsupportedSchema)
    ));
    assert!(matches!(
        SqliteRepository::migrate_legacy(&path, dir.path().join("unknown"), key()),
        Err(Error::UnsupportedSchema)
    ));
    assert!(!dir.path().join("unknown").exists());
    assert_eq!(migration::version(&source).unwrap(), 99);
}

#[test]
fn invalid_legacy_uuid_aborts_migration_without_discarding_rows() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("legacy.sqlite");
    let source = legacy(&path);
    source
        .execute(
            "UPDATE vault_content SET content_id = x'01' WHERE content_id = ?1",
            [[5u8; 16].as_slice()],
        )
        .unwrap();
    assert!(
        SqliteRepository::migrate_legacy(&path, dir.path().join("backup.sqlite"), key()).is_err()
    );
    assert_eq!(migration::version(&source).unwrap(), 0);
    assert_eq!(
        source
            .query_row("SELECT count(*) FROM vault_content", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        source
            .query_row(
                "SELECT count(*) FROM vault_content WHERE content_id=x'01'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn unknown_legacy_layout_is_not_modified_or_backed_up_as_a_valid_vault() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("legacy.sqlite");
    let source = legacy(&path);
    source
        .execute("ALTER TABLE vault_content ADD COLUMN extra TEXT", [])
        .unwrap();
    let backup = dir.path().join("backup.sqlite");
    assert!(matches!(
        SqliteRepository::migrate_legacy(&path, &backup, key()),
        Err(Error::UnsupportedSchema)
    ));
    assert!(!backup.exists());
    assert_eq!(migration::version(&source).unwrap(), 0);
    assert_eq!(
        source
            .query_row("SELECT count(*) FROM vault_content", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        4
    );
}
