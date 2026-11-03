//! Platform-independent vault domain. No database, UI, or OS dependencies.
mod edit_cache;
pub use edit_cache::{CredentialEditCache, CredentialField, EditError};

/// RFC 4122 bytes, matching the existing SQLite BLOB identifiers.
pub type CredentialId = [u8; 16];

/// Plaintext editing state; never log or persist this directly.
/// Deliberately does not implement Debug to avoid accidental credential logging.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Credential {
    pub content_id: CredentialId,
    pub org_name: String,
    pub username: String,
    pub password: String,
    pub email: String,
    pub note: String,
}
pub mod crypto;
