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

pub mod repository;

// Used by application-owned temporary rows and the edit cache. Callers retaining
// ordinary Credential copies must explicitly clear those copies themselves.
impl zeroize::Zeroize for Credential {
    fn zeroize(&mut self) {
        self.content_id.zeroize();
        self.org_name.zeroize();
        self.username.zeroize();
        self.password.zeroize();
        self.email.zeroize();
        self.note.zeroize();
    }
}
