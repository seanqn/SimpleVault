//! Persistence boundary for the application layer; no SQLite dependency.
use crate::{Credential, CredentialId};

pub type GroupId = [u8; 16];

#[derive(Clone, PartialEq, Eq)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
    /// SQLite UTC timestamp text, retained verbatim when importing legacy data.
    pub created_at: String,
}

pub trait VaultRepository {
    type Error;
    fn groups(&self) -> Result<Vec<Group>, Self::Error>;
    fn add_group(&mut self, id: GroupId, name: &str) -> Result<(), Self::Error>;
    fn rename_group(&mut self, id: GroupId, name: &str) -> Result<(), Self::Error>;
    fn remove_group(&mut self, id: GroupId) -> Result<(), Self::Error>;
    fn credentials(&self, group: GroupId) -> Result<Vec<Credential>, Self::Error>;
    /// Upsert the whole batch atomically. IDs cannot move between groups implicitly.
    fn save_credentials(
        &mut self,
        group: GroupId,
        credentials: &[Credential],
    ) -> Result<(), Self::Error>;
    fn remove_credential(&mut self, group: GroupId, id: CredentialId) -> Result<(), Self::Error>;
}
