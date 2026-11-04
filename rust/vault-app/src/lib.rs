//! Application operations and session lifecycle, independent of Qt and a web host.
mod controller;
mod session;
mod sqlite;

pub use controller::{ControllerError, CredentialSummary, SubmitOutcome, VaultController};
pub use session::{
    DraftResolution, SessionState, TransitionOutcome, UnlockError, UnlockResult, VaultApplication,
    VaultFactory,
};
pub use sqlite::SqliteVault;

#[cfg(test)]
mod tests;
