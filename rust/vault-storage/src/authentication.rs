//! Version 1 password envelope: Argon2id (64 MiB, 3 passes, 1 lane), XChaCha20-Poly1305.
use crate::{Error, Result, SqliteRepository};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::{
    aead::{Aead, AeadCore, OsRng, Payload},
    KeyInit, XChaCha20Poly1305,
};
use rusqlite::{params, Connection, OpenFlags};
use std::path::Path;
use vault_core::crypto::SecretKey;
use zeroize::Zeroizing;
fn derive(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0; 32]);
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(65536, 3, 1, Some(32)).map_err(|_| Error::Crypto)?,
    )
    .hash_password_into(password.as_bytes(), salt, key.as_mut())
    .map_err(|_| Error::Crypto)?;
    Ok(key)
}
fn connection(path: &Path) -> Result<Connection> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?)
}
pub fn identity(path: &Path) -> Result<Option<String>> {
    if !path.try_exists()? {
        return Ok(None);
    }
    Ok(Some(connection(path)?.query_row(
        "SELECT uid FROM password_envelope WHERE id=1",
        [],
        |r| r.get(0),
    )?))
}
pub fn create(path: &Path, password: &str) -> Result<String> {
    if password.is_empty() || password.len() > 1024 {
        return Err(Error::InvalidData);
    }
    let parent = path.parent().ok_or(Error::InvalidData)?;
    std::fs::create_dir_all(parent)?;
    let file = tempfile::NamedTempFile::new_in(parent)?;
    let uid = uuid::Uuid::new_v4().to_string();
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let mut key = Zeroizing::new([0u8; 32]);
    OsRng.fill_bytes(key.as_mut());
    let wrapping = derive(password, &salt)?;
    let cipher = XChaCha20Poly1305::new_from_slice(wrapping.as_ref()).map_err(|_| Error::Crypto)?;
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: key.as_ref(),
                aad: uid.as_bytes(),
            },
        )
        .map_err(|_| Error::Crypto)?;
    {
        let repository = SqliteRepository::open(
            file.path(),
            SecretKey::from_bytes(key.as_ref()).map_err(|_| Error::Crypto)?,
        )?;
        repository.connection.execute_batch("CREATE TABLE password_envelope (id INTEGER PRIMARY KEY CHECK(id=1), version INTEGER NOT NULL, uid TEXT UNIQUE NOT NULL, salt BLOB NOT NULL, nonce BLOB NOT NULL, ciphertext BLOB NOT NULL)")?;
        repository.connection.execute(
            "INSERT INTO password_envelope VALUES(1,1,?1,?2,?3,?4)",
            params![uid, salt.as_slice(), nonce.as_slice(), ciphertext],
        )?;
    }
    file.as_file().sync_all()?;
    file.persist_noclobber(path)
        .map_err(|e| Error::Io(e.error))?;
    Ok(uid)
}
pub fn unlock(path: &Path, uid: &str, password: &str) -> Result<SecretKey> {
    if password.len() > 1024 {
        return Err(Error::InvalidData);
    }
    let (version, stored, salt, nonce, ciphertext): (i64, String, Vec<u8>, Vec<u8>, Vec<u8>) =
        connection(path)?.query_row(
            "SELECT version,uid,salt,nonce,ciphertext FROM password_envelope WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )?;
    if version != 1
        || stored != uid
        || salt.len() != 16
        || nonce.len() != 24
        || ciphertext.len() != 48
    {
        return Err(Error::Crypto);
    }
    let wrapping = derive(password, &salt)?;
    let cipher = XChaCha20Poly1305::new_from_slice(wrapping.as_ref()).map_err(|_| Error::Crypto)?;
    let key = Zeroizing::new(
        cipher
            .decrypt(
                nonce.as_slice().into(),
                Payload {
                    msg: &ciphertext,
                    aad: uid.as_bytes(),
                },
            )
            .map_err(|_| Error::Crypto)?,
    );
    SecretKey::from_bytes(&key).map_err(|_| Error::Crypto)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persistence_and_rejection() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.sqlite");
        let uid = create(&path, "correct horse battery staple").unwrap();
        assert_eq!(identity(&path).unwrap(), Some(uid.clone()));
        assert!(unlock(&path, &uid, "wrong").is_err());
        assert!(unlock(&path, "other", "correct horse battery staple").is_err());
        assert!(create(&path, "replacement").is_err());
        SqliteRepository::open_existing(
            &path,
            unlock(&path, &uid, "correct horse battery staple").unwrap(),
        )
        .unwrap();
        let c = Connection::open(&path).unwrap();
        c.execute("UPDATE password_envelope SET ciphertext=zeroblob(48)", [])
            .unwrap();
        assert!(unlock(&path, &uid, "correct horse battery staple").is_err());
    }
}
