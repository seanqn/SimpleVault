use crate::{
    id_from_row, insert_credential, write_key_check, Credential, Error, Result, SecretKey,
};
use rusqlite::{backup::Backup, Connection, OpenFlags, TransactionBehavior};
use std::{path::Path, time::Duration};
use zeroize::Zeroize;

pub(crate) fn version(connection: &Connection) -> Result<i64> {
    Ok(connection.pragma_query_value(None, "user_version", |r| r.get(0))?)
}
fn tables(connection: &Connection) -> Result<Vec<String>> {
    let mut query = connection.prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")?;
    let rows = query.query_map([], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}
pub(crate) fn is_empty(connection: &Connection) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT count(*) = 0 FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
        [],
        |r| r.get(0),
    )?)
}
fn columns(connection: &Connection, table: &str, expected: &[(&str, &str)]) -> Result<()> {
    let mut query =
        connection.prepare("SELECT name, upper(type) FROM pragma_table_info(?1) ORDER BY cid")?;
    let actual: Vec<(String, String)> = query
        .query_map([table], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if actual
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect::<Vec<_>>()
        != expected
    {
        return Err(Error::UnsupportedSchema);
    }
    Ok(())
}
fn no_custom_code(connection: &Connection) -> Result<()> {
    let count: i64 = connection.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE type IN ('trigger','view')",
        [],
        |r| r.get(0),
    )?;
    if count != 0 {
        return Err(Error::UnsupportedSchema);
    }
    Ok(())
}
pub(crate) fn validate_current(connection: &Connection) -> Result<()> {
    let names = tables(connection)?;
    if names != ["groups", "vault_content", "vault_metadata"]
        && names
            != [
                "groups",
                "password_envelope",
                "vault_content",
                "vault_metadata",
            ]
    {
        return Err(Error::UnsupportedSchema);
    }
    no_custom_code(connection)?;
    columns(
        connection,
        "groups",
        &[("id", "BLOB"), ("name", "TEXT"), ("created_at", "TEXT")],
    )?;
    columns(
        connection,
        "vault_content",
        &[
            ("content_id", "BLOB"),
            ("group_id", "BLOB"),
            ("org_name", "TEXT"),
            ("username", "TEXT"),
            ("password_nonce", "BLOB"),
            ("password_ciphertext", "BLOB"),
            ("email", "TEXT"),
            ("note", "TEXT"),
        ],
    )?;
    columns(
        connection,
        "vault_metadata",
        &[("id", "INTEGER"), ("nonce", "BLOB"), ("ciphertext", "BLOB")],
    )?;
    let cascade: bool = connection.query_row("SELECT count(*) = 1 FROM pragma_foreign_key_list('vault_content') WHERE \"table\"='groups' AND \"from\"='group_id' AND \"to\"='id' AND on_delete='CASCADE'", [], |r| r.get(0))?;
    if !cascade {
        return Err(Error::UnsupportedSchema);
    }
    let bad_links: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check)",
        [],
        |r| r.get(0),
    )?;
    if bad_links {
        return Err(Error::InvalidData);
    }
    let mut query = connection.prepare("PRAGMA quick_check")?;
    for row in query.query_map([], |r| r.get::<_, String>(0))? {
        if row? != "ok" {
            return Err(Error::InvalidData);
        }
    }
    Ok(())
}
fn validate_legacy(connection: &Connection) -> Result<()> {
    if version(connection)? != 0 || tables(connection)? != ["groups", "vault_content"] {
        return Err(Error::UnsupportedSchema);
    }
    no_custom_code(connection)?;
    columns(
        connection,
        "groups",
        &[("id", "BLOB"), ("name", "TEXT"), ("created_at", "DATETIME")],
    )?;
    columns(
        connection,
        "vault_content",
        &[
            ("group_id", "BLOB"),
            ("content_id", "BLOB"),
            ("org_name", "TEXT"),
            ("username", "TEXT"),
            ("password", "TEXT"),
            ("email", "TEXT"),
            ("note", "TEXT"),
        ],
    )
}

pub(crate) fn migrate(
    connection: &mut Connection,
    source: &Path,
    backup_path: &Path,
    key: &SecretKey,
) -> Result<()> {
    // Reserve the writer before snapshotting: another connection cannot change the
    // source between the backup and migration. The backup reads through a separate
    // connection because SQLite's backup API cannot read its own write transaction.
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    validate_legacy(&tx)?;
    let source_reader = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let parent = backup_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temporary = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut destination = Connection::open(temporary.path())?;
        let backup = Backup::new(&source_reader, &mut destination)?;
        backup.run_to_completion(128, Duration::from_millis(10), None)?;
    }
    temporary.as_file().sync_all()?;
    // Never replace a previous backup (including a symlink or the source itself).
    let persisted = temporary
        .persist_noclobber(backup_path)
        .map_err(|e| Error::Io(e.error))?;
    persisted.sync_all()?;
    #[cfg(unix)]
    std::fs::File::open(parent)?.sync_all()?;

    tx.execute_batch(
        "ALTER TABLE vault_content RENAME TO legacy_content;
        ALTER TABLE groups RENAME TO legacy_groups;",
    )?;
    tx.execute_batch(include_str!("schema.sql"))?;
    tx.execute("INSERT INTO groups (id, name, created_at) SELECT id, name, coalesce(created_at, CURRENT_TIMESTAMP) FROM legacy_groups", [])?;
    {
        let mut query = tx.prepare("SELECT group_id, content_id, coalesce(org_name,''), coalesce(username,''), coalesce(password,''), coalesce(email,''), coalesce(note,'') FROM legacy_content ORDER BY content_id")?;
        let mut rows = query.query([])?;
        while let Some(row) = rows.next()? {
            // Read plaintext last so earlier conversion errors cannot leave a
            // successfully read password in an unguarded local allocation.
            let group = id_from_row(row, 0)?;
            let mut credential = Credential {
                content_id: id_from_row(row, 1)?,
                org_name: row.get(2)?,
                username: row.get(3)?,
                email: row.get(5)?,
                note: row.get(6)?,
                password: row.get(4)?,
            };
            let result = insert_credential(&tx, group, &credential, key);
            credential.password.zeroize();
            result?;
        }
    }
    tx.execute_batch("DROP TABLE legacy_content; DROP TABLE legacy_groups;")?;
    write_key_check(&tx, key)?;
    validate_current(&tx)?;
    tx.commit()?;
    Ok(())
}
