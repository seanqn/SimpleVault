use super::*;
use std::path::PathBuf;
use vault_storage::authentication;
pub struct PasswordVault {
    path: PathBuf,
    bridge: Bridge<UnavailableKeys>,
}
impl PasswordVault {
    pub fn new(path: PathBuf) -> Self {
        Self {
            bridge: Bridge::new(path.clone(), UnavailableKeys, false, None),
            path,
        }
    }
}
fn failure() -> ClientError {
    ClientError {
        code: "authentication_failed",
        message: "The vault could not be opened. Check your password and vault.",
    }
}
impl Commands for PasswordVault {
    fn dispatch(&mut self, request: Request) -> Result<Response, ClientError> {
        match request {
            Request::CreateVault { password } => {
                let password = Zeroizing::new(password);
                if self.bridge.app.state() != SessionState::Locked {
                    return Err(failure());
                }
                let uid = authentication::create(&self.path, &password).map_err(|_| failure())?;
                self.bridge.uid = Some(uid);
            }
            Request::UnlockVault { uid, password } => {
                let password = Zeroizing::new(password);
                if self.bridge.app.state() != SessionState::Locked {
                    return Err(failure());
                }
                let key =
                    authentication::unlock(&self.path, &uid, &password).map_err(|_| failure())?;
                self.bridge
                    .app
                    .unlock_key::<()>(key)
                    .map_err(|_| failure())?;
                self.bridge.uid = Some(uid);
            }
            Request::Snapshot {} => {
                self.bridge.uid = authentication::identity(&self.path).map_err(|_| failure())?;
            }
            other => return self.bridge.dispatch(other),
        }
        self.bridge.dispatch(Request::Snapshot {})
    }
}
#[cfg(all(test, feature = "dev-vault"))]
mod tests {
    use super::*;
    #[test]
    fn create_restart_unlock_and_lock() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.sqlite");
        let mut vault = PasswordVault::new(path.clone());
        assert!(vault
            .dispatch(Request::Snapshot {})
            .unwrap()
            .snapshot
            .as_ref()
            .unwrap()
            .uid
            .is_none());
        let response = vault
            .dispatch(Request::CreateVault {
                password: "a long master password".into(),
            })
            .unwrap();
        let uid = response.snapshot.as_ref().unwrap().uid.clone().unwrap();
        drop(vault);
        let mut vault = PasswordVault::new(path);
        assert!(vault
            .dispatch(Request::UnlockVault {
                uid: uid.clone(),
                password: "wrong".into()
            })
            .is_err());
        let response = vault
            .dispatch(Request::UnlockVault {
                uid,
                password: "a long master password".into(),
            })
            .unwrap();
        assert_eq!(response.snapshot.as_ref().unwrap().session, "unlocked");
        vault
            .dispatch(Request::CreateGroup {
                name: "Personal".into(),
            })
            .unwrap();
        let response = vault
            .dispatch(Request::Lock {
                resolution: Resolution::Discard,
            })
            .unwrap();
        assert_eq!(response.snapshot.as_ref().unwrap().session, "locked");
    }
}
