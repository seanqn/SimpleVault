//! Opt-in disposable vault, compiled only with dev-vault. Never reads user data.
use crate::{Bridge, ClientError, Commands, Request, Response};
use vault_core::crypto::{KeyProvider, SecretKey};
use zeroize::Zeroizing;

struct TemporaryKeys(Zeroizing<[u8; 32]>);
impl KeyProvider for TemporaryKeys {
    type Error = ();
    fn load_key(&self) -> Result<SecretKey, ()> {
        SecretKey::from_bytes(self.0.as_ref()).map_err(|_| ())
    }
}
pub struct TemporaryVault {
    bridge: Option<Bridge<TemporaryKeys>>,
    // Dropped after the bridge/connection to support Windows file semantics too.
    directory: Option<tempfile::TempDir>,
}
impl TemporaryVault {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let mut bytes = Zeroizing::new([0; 32]);
        getrandom::fill(bytes.as_mut())
            .map_err(|_| std::io::Error::other("random key generation failed"))?;
        let keys = TemporaryKeys(bytes);
        let key = keys
            .load_key()
            .map_err(|_| std::io::Error::other("invalid generated key"))?;
        let path = directory.path().join("vault.sqlite");
        drop(vault_storage::SqliteRepository::open(&path, key)?);
        Ok(Self {
            bridge: Some(Bridge::new(
                path,
                keys,
                true,
                Some("temporary-session".into()),
            )),
            directory: Some(directory),
        })
    }
}
impl Commands for TemporaryVault {
    fn dispatch(&mut self, request: Request) -> Result<Response, ClientError> {
        let bridge = self.bridge.as_mut().ok_or(ClientError {
            code: "closed",
            message: "The temporary vault is closed.",
        })?;
        let response = bridge.dispatch(request)?;
        if response
            .snapshot
            .as_ref()
            .is_some_and(|s| s.session == "closed")
        {
            self.bridge = None;
            self.directory = None;
        }
        Ok(response)
    }
}
