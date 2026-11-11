//! SQLite adapter. Only passwords are encrypted; other fields remain queryable text.
mod migration;

use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use std::{path::Path, time::Duration};
use vault_core::{
    crypto::{self, EncryptedCredential, SecretKey},
    repository::{Group, GroupId, VaultRepository},
    Credential, CredentialId,
};
use zeroize::Zeroizing;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database operation failed")]
    Database(#[from] rusqlite::Error),
    #[error("filesystem operation failed")]
    Io(#[from] std::io::Error),
    #[error("legacy database needs explicit migration with a backup destination")]
    MigrationRequired,
    #[error("unsupported database schema or version")]
    UnsupportedSchema,
    #[error("database contains invalid data")]
    InvalidData,
    #[error("record not found")]
    NotFound,
    #[error("credential belongs to another group")]
    GroupConflict,
    #[error("password encryption or authentication failed")]
    Crypto,
}
pub type Result<T> = std::result::Result<T, Error>;

/// Holds an owned zeroizing key. Drop this repository when locking the vault.
/// Open never migrates an existing legacy database implicitly.
pub struct SqliteRepository {
    connection: Connection,
    key: SecretKey,
}

impl SqliteRepository {
    pub fn open(path: impl AsRef<Path>, key: SecretKey) -> Result<Self> {
        Self::open_with_creation(path.as_ref(), key, true)
    }

    /// Open an existing vault without silently creating one after a path typo.
    pub fn open_existing(path: impl AsRef<Path>, key: SecretKey) -> Result<Self> {
        Self::open_with_creation(path.as_ref(), key, false)
    }

    fn open_with_creation(path: &Path, key: SecretKey, create: bool) -> Result<Self> {
        let mut connection = connect(path, create)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        match migration::version(&tx)? {
            0 if create && migration::is_empty(&tx)? => {
                tx.execute_batch(include_str!("schema.sql"))?;
                write_key_check(&tx, &key)?;
            }
            0 => return Err(Error::MigrationRequired),
            1 => migration::validate_current(&tx)?,
            _ => return Err(Error::UnsupportedSchema),
        }
        verify_key(&tx, &key)?;
        tx.commit()?;
        Ok(Self { connection, key })
    }

    /// Migrate only the recognized Qt schema. The backup destination must not exist.
    /// Close the legacy application first. Backup includes committed WAL contents.
    pub fn migrate_legacy(
        path: impl AsRef<Path>,
        backup: impl AsRef<Path>,
        key: SecretKey,
    ) -> Result<Self> {
        let mut connection = connect(path.as_ref(), false)?;
        migration::migrate(&mut connection, path.as_ref(), backup.as_ref(), &key)?;
        Ok(Self { connection, key })
    }
}

fn connect(path: &Path, create: bool) -> Result<Connection> {
    let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if create {
        flags |= OpenFlags::SQLITE_OPEN_CREATE;
    }
    let connection = Connection::open_with_flags(path, flags)?;
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "secure_delete", true)?;
    Ok(connection)
}

const KEY_CHECK: &str = "SimpleVault database key check v1";
fn write_key_check(connection: &Connection, key: &SecretKey) -> Result<()> {
    let encrypted = crypto::encrypt_stored_password(Zeroizing::new(KEY_CHECK.into()), key)
        .map_err(|_| Error::Crypto)?;
    connection.execute(
        "INSERT INTO vault_metadata (id, nonce, ciphertext) VALUES (1, ?1, ?2)",
        params![encrypted.nonce.as_slice(), encrypted.secret],
    )?;
    Ok(())
}
fn verify_key(connection: &Connection, key: &SecretKey) -> Result<()> {
    let (nonce, secret): (Vec<u8>, Vec<u8>) = connection.query_row(
        "SELECT nonce, ciphertext FROM vault_metadata WHERE id = 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let encrypted = EncryptedCredential {
        nonce: nonce.try_into().map_err(|_| Error::InvalidData)?,
        secret,
    };
    let text = crypto::decrypt(&encrypted, key).map_err(|_| Error::Crypto)?;
    if text.as_str() != KEY_CHECK {
        return Err(Error::Crypto);
    }
    Ok(())
}
fn require_group(connection: &Connection, id: GroupId) -> Result<()> {
    if !connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM groups WHERE id = ?1)",
        [id.as_slice()],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(Error::NotFound);
    }
    Ok(())
}
fn changed(count: usize) -> Result<()> {
    if count == 0 {
        Err(Error::NotFound)
    } else {
        Ok(())
    }
}
fn id_from_row(row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<[u8; 16]> {
    let bytes: Vec<u8> = row.get(column)?;
    bytes.try_into().map_err(|_| rusqlite::Error::InvalidQuery)
}

impl VaultRepository for SqliteRepository {
    type Error = Error;
    fn groups(&self) -> Result<Vec<Group>> {
        let mut query = self
            .connection
            .prepare("SELECT id, name, created_at FROM groups ORDER BY created_at, id")?;
        let rows = query.query_map([], |row| {
            Ok(Group {
                id: id_from_row(row, 0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
    fn add_group(&mut self, id: GroupId, name: &str) -> Result<()> {
        self.connection.execute(
            "INSERT INTO groups (id, name) VALUES (?1, ?2)",
            params![id.as_slice(), name],
        )?;
        Ok(())
    }
    fn rename_group(&mut self, id: GroupId, name: &str) -> Result<()> {
        changed(self.connection.execute(
            "UPDATE groups SET name = ?1 WHERE id = ?2",
            params![name, id.as_slice()],
        )?)
    }
    fn remove_group(&mut self, id: GroupId) -> Result<()> {
        changed(
            self.connection
                .execute("DELETE FROM groups WHERE id = ?1", [id.as_slice()])?,
        )
    }
    fn credentials(&self, group: GroupId) -> Result<Vec<Credential>> {
        require_group(&self.connection, group)?;
        let mut query = self.connection.prepare("SELECT content_id, org_name, username, password_nonce, password_ciphertext, email, note FROM vault_content WHERE group_id = ?1 ORDER BY content_id")?;
        let mut rows = query.query([group.as_slice()])?;
        let mut credentials = Vec::new();
        while let Some(row) = rows.next()? {
            let nonce: Vec<u8> = row.get(3)?;
            let encrypted = EncryptedCredential {
                nonce: nonce.try_into().map_err(|_| Error::InvalidData)?,
                secret: row.get(4)?,
            };
            let password = crypto::decrypt(&encrypted, &self.key).map_err(|_| Error::Crypto)?;
            credentials.push(Credential {
                content_id: id_from_row(row, 0)?,
                org_name: row.get(1)?,
                username: row.get(2)?,
                password: password.to_string(),
                email: row.get(5)?,
                note: row.get(6)?,
            });
        }
        Ok(credentials)
    }
    fn save_credentials(&mut self, group: GroupId, credentials: &[Credential]) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_group(&tx, group)?;
        for credential in credentials {
            let owner: Option<Vec<u8>> = tx
                .query_row(
                    "SELECT group_id FROM vault_content WHERE content_id = ?1",
                    [credential.content_id.as_slice()],
                    |r| r.get(0),
                )
                .optional()?;
            if owner.is_some_and(|id| id != group) {
                return Err(Error::GroupConflict);
            }
            insert_credential(&tx, group, credential, &self.key)?;
        }
        tx.commit()?;
        Ok(())
    }
    fn remove_credential(&mut self, group: GroupId, id: CredentialId) -> Result<()> {
        changed(self.connection.execute(
            "DELETE FROM vault_content WHERE group_id = ?1 AND content_id = ?2",
            params![group.as_slice(), id.as_slice()],
        )?)
    }
}

fn insert_credential(
    connection: &Connection,
    group: GroupId,
    credential: &Credential,
    key: &SecretKey,
) -> Result<()> {
    let encrypted =
        crypto::encrypt_stored_password(Zeroizing::new(credential.password.clone()), key)
            .map_err(|_| Error::Crypto)?;
    connection.execute("INSERT INTO vault_content (content_id, group_id, org_name, username, password_nonce, password_ciphertext, email, note)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(content_id) DO UPDATE SET org_name=excluded.org_name, username=excluded.username,
        password_nonce=excluded.password_nonce, password_ciphertext=excluded.password_ciphertext, email=excluded.email, note=excluded.note",
        params![credential.content_id.as_slice(), group.as_slice(), credential.org_name, credential.username, encrypted.nonce.as_slice(), encrypted.secret, credential.email, credential.note])?;
    Ok(())
}

#[cfg(test)]
mod tests;

pub mod authentication;
