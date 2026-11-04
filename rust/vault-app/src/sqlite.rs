use crate::VaultFactory;
use std::path::PathBuf;
use vault_core::crypto::SecretKey;
use vault_storage::{Error, SqliteRepository};

/// Factory for a previously initialized/migrated vault. Provisioning and migration
/// remain explicit storage operations; unlocking never creates a missing vault.
pub struct SqliteVault {
    path: PathBuf,
}
impl SqliteVault {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}
impl VaultFactory for SqliteVault {
    type Repository = SqliteRepository;
    type Error = Error;
    fn open(&self, key: SecretKey) -> Result<Self::Repository, Self::Error> {
        SqliteRepository::open_existing(&self.path, key)
    }
}
