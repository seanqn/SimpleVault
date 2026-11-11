//! Typed desktop protocol. Keys and repository handles never cross this boundary.
use serde::{Deserialize, Serialize};
use vault_app::{
    ControllerError, DraftResolution, SessionState, SqliteVault, UnlockError, VaultApplication,
};
use vault_core::{crypto::KeyProvider, Credential};
use zeroize::{Zeroize, Zeroizing};

#[cfg(feature = "dev-vault")]
pub mod development;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Snapshot {},
    Unlock {},
    CreateVault { password: String },
    UnlockVault { uid: String, password: String },
    CreateGroup { name: String },
    RenameGroup { id: String, name: String },
    RemoveGroup { id: String },
    SelectGroup { id: Option<String> },
    AddCredential {},
    BeginEdit { id: String },
    UpdateDraft { fields: EditFields },
    SubmitEdit {},
    CancelEdit {},
    RemoveCredential { id: String },
    RevealPassword { id: String },
    Lock { resolution: Resolution },
    Shutdown { resolution: Resolution },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditFields {
    pub org_name: String,
    pub username: String,
    pub email: String,
    pub note: String,
    /// None preserves the stored password; Some("") explicitly clears it.
    pub password: Option<String>,
}
impl Drop for EditFields {
    fn drop(&mut self) {
        self.org_name.zeroize();
        self.username.zeroize();
        self.email.zeroize();
        self.note.zeroize();
        if let Some(password) = &mut self.password {
            password.zeroize();
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    Save,
    Discard,
    Cancel,
}
impl From<Resolution> for DraftResolution {
    fn from(value: Resolution) -> Self {
        match value {
            Resolution::Save => Self::Save,
            Resolution::Discard => Self::Discard,
            Resolution::Cancel => Self::Cancel,
        }
    }
}
#[derive(Serialize)]
pub struct GroupView {
    pub id: String,
    pub name: String,
}
#[derive(Serialize)]
pub struct CredentialView {
    pub id: String,
    pub org_name: String,
    pub username: String,
    pub email: String,
    pub note: String,
}
#[derive(Serialize)]
pub struct Snapshot {
    pub session: &'static str,
    pub temporary: bool,
    pub uid: Option<String>,
    pub groups: Vec<GroupView>,
    pub credentials: Vec<CredentialView>,
    pub selected_group: Option<String>,
    pub selected_credential: Option<String>,
    pub draft: Option<CredentialView>,
    pub dirty: bool,
}
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ClientError {
    pub code: &'static str,
    pub message: &'static str,
}
#[derive(Serialize)]
pub struct Response {
    pub snapshot: Option<Snapshot>,
    /// A mutation succeeded but refreshing display data failed. Do not retry the mutation.
    pub refresh_error: Option<ClientError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}
impl Drop for Response {
    fn drop(&mut self) {
        if let Some(secret) = &mut self.secret {
            secret.zeroize();
        }
    }
}
pub trait Commands: Send {
    fn dispatch(&mut self, request: Request) -> Result<Response, ClientError>;
}
pub struct Bridge<P: KeyProvider> {
    app: VaultApplication<SqliteVault>,
    provider: P,
    temporary: bool,
    uid: Option<String>,
}
impl<P: KeyProvider> Bridge<P> {
    pub fn new(
        path: impl Into<std::path::PathBuf>,
        provider: P,
        temporary: bool,
        uid: Option<String>,
    ) -> Self {
        Self {
            app: VaultApplication::new(SqliteVault::new(path)),
            provider,
            temporary,
            uid,
        }
    }
    fn snapshot(&self) -> Result<Snapshot, ClientError> {
        let state = self.app.state();
        let mut snapshot = Snapshot {
            session: match state {
                SessionState::Locked => "locked",
                SessionState::Unlocked => "unlocked",
                SessionState::Closed => "closed",
            },
            temporary: self.temporary,
            uid: self.uid.clone(),
            groups: vec![],
            credentials: vec![],
            selected_group: None,
            selected_credential: None,
            draft: None,
            dirty: false,
        };
        if state == SessionState::Unlocked {
            let controller = self.app.controller().map_err(controller_error)?;
            snapshot.groups = controller
                .groups()
                .map_err(controller_error)?
                .into_iter()
                .map(|g| GroupView {
                    id: uuid_string(g.id),
                    name: g.name,
                })
                .collect();
            snapshot.credentials = controller
                .credentials()
                .map_err(controller_error)?
                .into_iter()
                .map(|r| CredentialView {
                    id: uuid_string(r.id),
                    org_name: r.org_name,
                    username: r.username,
                    email: r.email,
                    note: r.note,
                })
                .collect();
            snapshot.selected_group = controller.selected_group().map(uuid_string);
            snapshot.selected_credential = controller.selected_credential().map(uuid_string);
            snapshot.draft = controller.draft().map(draft_view);
            snapshot.dirty = controller.is_dirty();
        }
        Ok(snapshot)
    }
    fn execute(&mut self, request: Request) -> Result<Option<Zeroizing<String>>, ClientError> {
        match request {
            Request::CreateVault { password } | Request::UnlockVault { password, .. } => {
                let _password = Zeroizing::new(password);
                return Err(ClientError {
                    code: "unsupported",
                    message: "This vault does not support password provisioning.",
                });
            }
            Request::Snapshot {} => {}
            Request::Unlock {} => self
                .app
                .unlock(&self.provider)
                .map_err(|error| match error {
                    UnlockError::Key(_) => ClientError {
                        code: "authentication_unavailable",
                        message: "Device authentication is unavailable. Your vault remains locked.",
                    },
                    UnlockError::Open(vault_storage::Error::Crypto) => ClientError {
                        code: "authentication_failed",
                        message: "The vault could not be unlocked.",
                    },
                    UnlockError::Open(vault_storage::Error::MigrationRequired) => ClientError {
                        code: "migration_required",
                        message: "This vault needs to be migrated before it can be opened.",
                    },
                    _ => ClientError {
                        code: "unlock_failed",
                        message: "The vault could not be opened. Try again.",
                    },
                })?,
            Request::Lock { resolution } => {
                self.app.lock(resolution.into()).map_err(controller_error)?;
            }
            Request::Shutdown { resolution } => {
                self.app
                    .shutdown(resolution.into())
                    .map_err(controller_error)?;
            }
            request => {
                let c = self.app.controller_mut().map_err(controller_error)?;
                match request {
                    Request::CreateGroup { name } => {
                        c.create_group(&name).map_err(controller_error)?;
                    }
                    Request::RenameGroup { id, name } => c
                        .rename_group(parse_id(&id)?, &name)
                        .map_err(controller_error)?,
                    Request::RemoveGroup { id } => {
                        c.remove_group(parse_id(&id)?).map_err(controller_error)?
                    }
                    Request::SelectGroup { id } => c
                        .select_group(id.as_deref().map(parse_id).transpose()?)
                        .map_err(controller_error)?,
                    Request::AddCredential {} => {
                        c.add_credential().map_err(controller_error)?;
                    }
                    Request::BeginEdit { id } => {
                        c.begin_edit(parse_id(&id)?).map_err(controller_error)?
                    }
                    Request::UpdateDraft { mut fields } => {
                        if !c.has_edit() {
                            return Err(ClientError {
                                code: "no_edit",
                                message: "Select a credential to edit first.",
                            });
                        }
                        use vault_core::CredentialField as F;
                        for (field, value) in [
                            (F::Organization, &mut fields.org_name),
                            (F::Username, &mut fields.username),
                            (F::Email, &mut fields.email),
                            (F::Note, &mut fields.note),
                        ] {
                            c.update_edit(field, std::mem::take(value))
                                .map_err(controller_error)?;
                        }
                        if let Some(password) = fields.password.take() {
                            c.update_edit(F::Password, password)
                                .map_err(controller_error)?;
                        }
                    }
                    Request::SubmitEdit {} => {
                        c.submit_edit().map_err(controller_error)?;
                    }
                    Request::CancelEdit {} => c.cancel_edit(),
                    Request::RemoveCredential { id } => c
                        .remove_credential(parse_id(&id)?)
                        .map_err(controller_error)?,
                    Request::RevealPassword { id } => {
                        return Ok(Some(
                            c.reveal_password(parse_id(&id)?)
                                .map_err(controller_error)?,
                        ))
                    }
                    _ => unreachable!(),
                }
            }
        }
        Ok(None)
    }
}
impl<P: KeyProvider + Send> Commands for Bridge<P> {
    fn dispatch(&mut self, request: Request) -> Result<Response, ClientError> {
        if matches!(request, Request::Snapshot {}) {
            return Ok(Response {
                snapshot: Some(self.snapshot()?),
                refresh_error: None,
                secret: None,
            });
        }
        let secret = self.execute(request)?;
        // Read failures after a committed mutation must not be reported as a failed save.
        let (snapshot, refresh_error) = match self.snapshot() {
            Ok(s) => (Some(s), None),
            Err(e) => (None, Some(e)),
        };
        Ok(Response {
            snapshot,
            refresh_error,
            secret: secret.map(|s| s.to_string()),
        })
    }
}
fn uuid_string(id: [u8; 16]) -> String {
    uuid::Uuid::from_bytes(id).to_string()
}
fn parse_id(value: &str) -> Result<[u8; 16], ClientError> {
    uuid::Uuid::parse_str(value)
        .map(|id| *id.as_bytes())
        .map_err(|_| ClientError {
            code: "invalid_id",
            message: "That item is no longer available. Refresh the vault.",
        })
}
fn draft_view(row: &Credential) -> CredentialView {
    CredentialView {
        id: uuid_string(row.content_id),
        org_name: row.org_name.clone(),
        username: row.username.clone(),
        email: row.email.clone(),
        note: row.note.clone(),
    }
}
fn controller_error(error: ControllerError<vault_storage::Error>) -> ClientError {
    match error {
        ControllerError::Locked | ControllerError::Closed => ClientError {
            code: "locked",
            message: "Unlock the vault to continue.",
        },
        ControllerError::EmptyGroupName => ClientError {
            code: "invalid_name",
            message: "Enter a name for this group.",
        },
        ControllerError::NoGroupSelected => ClientError {
            code: "no_group",
            message: "Select a group first.",
        },
        ControllerError::CredentialNotFound
        | ControllerError::Repository(vault_storage::Error::NotFound) => ClientError {
            code: "not_found",
            message: "This item is no longer available. Your draft has been kept.",
        },
        _ => ClientError {
            code: "operation_failed",
            message: "The change could not be completed. Your draft has been kept; try again.",
        },
    }
}

/// Default host remains locked until a native key provider is installed.
pub struct UnavailableKeys;
impl KeyProvider for UnavailableKeys {
    type Error = ();
    fn load_key(&self) -> Result<vault_core::crypto::SecretKey, ()> {
        Err(())
    }
}

#[cfg(all(test, feature = "dev-vault"))]
mod tests {
    use super::*;
    #[test]
    fn protocol_roundtrip_uses_sqlite_without_serializing_passwords_or_keys() {
        let mut service = development::TemporaryVault::new().unwrap();
        let response = service.dispatch(Request::Snapshot {}).unwrap();
        assert_eq!(response.snapshot.as_ref().unwrap().session, "locked");
        assert!(service.dispatch(Request::AddCredential {}).is_err());
        service.dispatch(Request::Unlock {}).unwrap();
        let response = service
            .dispatch(Request::CreateGroup {
                name: "Personal".into(),
            })
            .unwrap();
        let id = response.snapshot.as_ref().unwrap().groups[0].id.clone();
        service
            .dispatch(Request::SelectGroup { id: Some(id) })
            .unwrap();
        service.dispatch(Request::AddCredential {}).unwrap();
        let response = service
            .dispatch(Request::UpdateDraft {
                fields: EditFields {
                    org_name: "Example".into(),
                    username: "alice".into(),
                    password: Some("not-in-snapshots".into()),
                    email: String::new(),
                    note: String::new(),
                },
            })
            .unwrap();
        assert!(!serde_json::to_string(&response)
            .unwrap()
            .contains("not-in-snapshots"));
        let response = service.dispatch(Request::SubmitEdit {}).unwrap();
        let id = response.snapshot.as_ref().unwrap().credentials[0]
            .id
            .clone();
        assert!(!serde_json::to_string(&response)
            .unwrap()
            .contains("password"));
        let reveal = service
            .dispatch(Request::RevealPassword { id: id.clone() })
            .unwrap();
        assert_eq!(reveal.secret.as_deref(), Some("not-in-snapshots"));
        service
            .dispatch(Request::BeginEdit { id: id.clone() })
            .unwrap();
        service
            .dispatch(Request::UpdateDraft {
                fields: EditFields {
                    org_name: "Edited".into(),
                    username: "alice".into(),
                    password: None,
                    email: String::new(),
                    note: String::new(),
                },
            })
            .unwrap();
        service.dispatch(Request::SubmitEdit {}).unwrap();
        assert_eq!(
            service
                .dispatch(Request::RevealPassword { id })
                .unwrap()
                .secret
                .as_deref(),
            Some("not-in-snapshots")
        );
        let response = service
            .dispatch(Request::Lock {
                resolution: Resolution::Save,
            })
            .unwrap();
        assert!(response.snapshot.as_ref().unwrap().credentials.is_empty());
        service.dispatch(Request::Unlock {}).unwrap();
        service
            .dispatch(Request::Shutdown {
                resolution: Resolution::Discard,
            })
            .unwrap();
        assert!(service.dispatch(Request::Unlock {}).is_err());
    }
    #[test]
    fn malformed_ids_and_unavailable_authentication_are_safe_errors() {
        let mut service = development::TemporaryVault::new().unwrap();
        service.dispatch(Request::Unlock {}).unwrap();
        let error = match service.dispatch(Request::SelectGroup {
            id: Some("bad-id".into()),
        }) {
            Err(e) => e,
            _ => panic!("expected invalid id"),
        };
        assert_eq!(error.code, "invalid_id");
        assert!(
            serde_json::from_str::<Request>(r#"{"type":"unlock","key":"do-not-accept"}"#).is_err()
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.sqlite");
        let mut service = Bridge::new(&path, UnavailableKeys, false, None);
        assert!(service.dispatch(Request::Unlock {}).is_err());
        assert!(!path.exists());
    }
}

pub mod authentication;
