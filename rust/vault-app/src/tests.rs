use super::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};
use vault_core::{
    crypto::{KeyProvider, SecretKey},
    repository::{Group, GroupId, VaultRepository},
    Credential, CredentialField, CredentialId,
};
use vault_storage::SqliteRepository;

#[derive(Default)]
struct Faults {
    save: Cell<bool>,
    groups: Cell<bool>,
    credentials: Cell<bool>,
    remove_group: Cell<bool>,
    remove_credential: Cell<bool>,
    save_calls: Cell<usize>,
    dropped: Cell<usize>,
}
#[derive(Debug)]
enum RepoError {
    Injected,
    Storage(vault_storage::Error),
}
struct FaultRepository {
    inner: SqliteRepository,
    faults: Rc<Faults>,
}
impl Drop for FaultRepository {
    fn drop(&mut self) {
        self.faults.dropped.set(self.faults.dropped.get() + 1);
    }
}
impl VaultRepository for FaultRepository {
    type Error = RepoError;
    fn groups(&self) -> Result<Vec<Group>, Self::Error> {
        if self.faults.groups.get() {
            return Err(RepoError::Injected);
        }
        self.inner.groups().map_err(RepoError::Storage)
    }
    fn add_group(&mut self, id: GroupId, name: &str) -> Result<(), Self::Error> {
        self.inner.add_group(id, name).map_err(RepoError::Storage)
    }
    fn rename_group(&mut self, id: GroupId, name: &str) -> Result<(), Self::Error> {
        self.inner
            .rename_group(id, name)
            .map_err(RepoError::Storage)
    }
    fn remove_group(&mut self, id: GroupId) -> Result<(), Self::Error> {
        if self.faults.remove_group.get() {
            return Err(RepoError::Injected);
        }
        self.inner.remove_group(id).map_err(RepoError::Storage)
    }
    fn credentials(&self, group: GroupId) -> Result<Vec<Credential>, Self::Error> {
        if self.faults.credentials.get() {
            return Err(RepoError::Injected);
        }
        self.inner.credentials(group).map_err(RepoError::Storage)
    }
    fn save_credentials(&mut self, group: GroupId, rows: &[Credential]) -> Result<(), Self::Error> {
        self.faults.save_calls.set(self.faults.save_calls.get() + 1);
        if self.faults.save.get() {
            return Err(RepoError::Injected);
        }
        self.inner
            .save_credentials(group, rows)
            .map_err(RepoError::Storage)
    }
    fn remove_credential(&mut self, group: GroupId, id: CredentialId) -> Result<(), Self::Error> {
        if self.faults.remove_credential.get() {
            return Err(RepoError::Injected);
        }
        self.inner
            .remove_credential(group, id)
            .map_err(RepoError::Storage)
    }
}
struct Factory {
    path: PathBuf,
    faults: Rc<Faults>,
}
impl VaultFactory for Factory {
    type Repository = FaultRepository;
    type Error = vault_storage::Error;
    fn open(&self, key: SecretKey) -> Result<Self::Repository, Self::Error> {
        Ok(FaultRepository {
            inner: SqliteRepository::open_existing(&self.path, key)?,
            faults: self.faults.clone(),
        })
    }
}
struct Provider {
    byte: Option<u8>,
    calls: Cell<usize>,
}
impl Provider {
    fn new(byte: Option<u8>) -> Self {
        Self {
            byte,
            calls: Cell::new(0),
        }
    }
}
impl KeyProvider for Provider {
    type Error = &'static str;
    fn load_key(&self) -> Result<SecretKey, Self::Error> {
        self.calls.set(self.calls.get() + 1);
        SecretKey::from_bytes(&[self.byte.ok_or("key unavailable")?; 32])
            .map_err(|_| "invalid fixture key")
    }
}
struct Harness {
    app: VaultApplication<Factory>,
    provider: Provider,
    faults: Rc<Faults>,
    dir: tempfile::TempDir,
}
impl Harness {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.sqlite");
        let provider = Provider::new(Some(7));
        drop(SqliteRepository::open(&path, provider.load_key().unwrap()).unwrap());
        let faults = Rc::new(Faults::default());
        let mut app = VaultApplication::new(Factory {
            path,
            faults: faults.clone(),
        });
        app.unlock(&provider).unwrap();
        Self {
            app,
            provider,
            faults,
            dir,
        }
    }
    fn controller(&mut self) -> &mut VaultController<FaultRepository> {
        self.app.controller_mut().unwrap()
    }
    fn selected_group(&mut self) -> GroupId {
        let controller = self.controller();
        let id = controller.create_group("My Vault").unwrap();
        controller.select_group(Some(id)).unwrap();
        id
    }
    fn saved_row(&mut self, password: &str) -> CredentialId {
        let controller = self.controller();
        let id = controller.add_credential().unwrap();
        controller
            .update_edit(CredentialField::Password, password.into())
            .unwrap();
        assert_eq!(controller.submit_edit().unwrap(), SubmitOutcome::Saved);
        id
    }
    fn disk(&self) -> SqliteRepository {
        SqliteRepository::open_existing(
            self.dir.path().join("vault.sqlite"),
            self.provider.load_key().unwrap(),
        )
        .unwrap()
    }
}

#[test]
fn startup_has_no_mock_data_and_group_commands_use_stable_ids() {
    let mut h = Harness::new();
    let c = h.controller();
    assert!(c.groups().unwrap().is_empty());
    assert!(c.credentials().unwrap().is_empty());
    assert_eq!(c.selected_group(), None);
    assert!(matches!(
        c.add_credential(),
        Err(ControllerError::NoGroupSelected)
    ));
    assert!(matches!(
        c.create_group(" \n "),
        Err(ControllerError::EmptyGroupName)
    ));
    let a = c.create_group("  A  ").unwrap();
    let b = c.create_group("B").unwrap();
    assert_ne!(a, b);
    assert_eq!(uuid::Uuid::from_bytes(a).get_version_num(), 7);
    c.rename_group(a, " renamed ").unwrap();
    assert!(c
        .groups()
        .unwrap()
        .iter()
        .any(|g| g.id == a && g.name == "renamed"));
    c.select_group(Some(b)).unwrap();
    c.remove_group(a).unwrap();
    assert_eq!(c.selected_group(), Some(b));
    assert_eq!(c.groups().unwrap().len(), 1);
}

#[test]
fn blank_provisional_rows_are_deduplicated_and_finish_without_writes() {
    let mut h = Harness::new();
    h.selected_group();
    let id = h.controller().add_credential().unwrap();
    assert_eq!(h.controller().add_credential().unwrap(), id);
    assert_eq!(h.controller().credentials().unwrap().len(), 1);
    assert_eq!(
        h.controller().submit_edit().unwrap(),
        SubmitOutcome::DiscardedEmptyNewRow
    );
    assert!(!h.controller().has_edit());
    assert_eq!(h.controller().selected_credential(), None);
    assert!(h.controller().credentials().unwrap().is_empty());
    assert_eq!(h.faults.save_calls.get(), 0);
    assert_eq!(h.controller().submit_edit().unwrap(), SubmitOutcome::NoEdit);
}

#[test]
fn populated_new_row_saves_before_next_and_cancel_clears_provisional_state() {
    let mut h = Harness::new();
    let group = h.selected_group();
    let first = h.controller().add_credential().unwrap();
    h.controller()
        .update_edit_role("org_name", "Example".into())
        .unwrap();
    let next = h.controller().add_credential().unwrap();
    assert_ne!(first, next);
    let persisted = h.disk().credentials(group).unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].content_id, first);
    assert_eq!(persisted[0].password, "");
    h.controller().cancel_edit();
    assert_eq!(h.controller().selected_credential(), None);
    assert!(!h.controller().has_edit());
    assert_eq!(h.controller().credentials().unwrap().len(), 1);
}

#[test]
fn existing_clean_reverted_and_empty_edits_finish_correctly() {
    let mut h = Harness::new();
    let group = h.selected_group();
    let id = h.saved_row("original");
    h.controller().begin_edit(id).unwrap();
    assert_eq!(
        h.controller().submit_edit().unwrap(),
        SubmitOutcome::Unchanged
    );
    assert!(!h.controller().has_edit());
    h.controller().begin_edit(id).unwrap();
    h.controller()
        .update_edit(CredentialField::Password, "changed".into())
        .unwrap();
    h.controller()
        .update_edit(CredentialField::Password, "original".into())
        .unwrap();
    assert!(!h.controller().is_dirty());
    assert_eq!(
        h.controller().submit_edit().unwrap(),
        SubmitOutcome::Unchanged
    );
    assert_eq!(h.faults.save_calls.get(), 1);
    h.controller().begin_edit(id).unwrap();
    h.controller()
        .update_edit(CredentialField::Password, String::new())
        .unwrap();
    assert_eq!(h.controller().submit_edit().unwrap(), SubmitOutcome::Saved);
    assert!(!h.controller().has_edit());
    let rows = h.disk().credentials(group).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].password, "");
    h.controller().begin_edit(id).unwrap();
    assert_eq!(
        h.controller().submit_edit().unwrap(),
        SubmitOutcome::Unchanged
    );
}

#[test]
fn failed_save_retains_draft_and_blocks_all_navigation_then_can_retry() {
    let mut h = Harness::new();
    let group = h.selected_group();
    let first = h.saved_row("first");
    let second = h.saved_row("second");
    let target = h.controller().create_group("other").unwrap();
    h.controller().begin_edit(first).unwrap();
    h.controller()
        .update_edit(CredentialField::Password, "unsaved".into())
        .unwrap();
    h.faults.save.set(true);
    assert!(h.controller().submit_edit().is_err());
    assert!(h.controller().select_group(Some(target)).is_err());
    assert!(h.controller().select_group(None).is_err());
    assert!(h.controller().select_credential(Some(second)).is_err());
    assert!(h.controller().begin_edit(second).is_err());
    assert!(h.controller().add_credential().is_err());
    assert_eq!(h.controller().selected_group(), Some(group));
    assert_eq!(h.controller().selected_credential(), Some(first));
    assert_eq!(h.controller().draft().unwrap().password, "unsaved");
    assert!(h.controller().is_dirty());
    assert_eq!(
        h.disk()
            .credentials(group)
            .unwrap()
            .iter()
            .find(|r| r.content_id == first)
            .unwrap()
            .password,
        "first"
    );
    h.faults.save.set(false);
    h.controller().select_group(Some(target)).unwrap();
    assert!(!h.controller().has_edit());
    assert_eq!(h.controller().selected_group(), Some(target));
    assert_eq!(h.controller().selected_credential(), None);
    assert_eq!(
        h.disk()
            .credentials(group)
            .unwrap()
            .iter()
            .find(|r| r.content_id == first)
            .unwrap()
            .password,
        "unsaved"
    );
}

#[test]
fn failed_target_reads_and_invalid_ids_do_not_save_or_change_selection() {
    let mut h = Harness::new();
    let group = h.selected_group();
    let id = h.saved_row("old");
    let target = h.controller().create_group("other").unwrap();
    h.controller().begin_edit(id).unwrap();
    h.controller()
        .update_edit_role("note", "pending".into())
        .unwrap();
    let saves = h.faults.save_calls.get();
    assert!(matches!(
        h.controller().select_group(Some([0; 16])),
        Err(ControllerError::Repository(RepoError::Storage(
            vault_storage::Error::NotFound
        )))
    ));
    assert!(matches!(
        h.controller().begin_edit([0; 16]),
        Err(ControllerError::CredentialNotFound)
    ));
    assert!(matches!(
        h.controller().select_credential(Some([0; 16])),
        Err(ControllerError::CredentialNotFound)
    ));
    h.faults.credentials.set(true);
    assert!(h.controller().select_group(Some(target)).is_err());
    assert!(h.controller().credentials().is_err());
    assert_eq!(h.faults.save_calls.get(), saves);
    assert_eq!(h.controller().selected_group(), Some(group));
    assert_eq!(h.controller().draft().unwrap().note, "pending");
    h.controller().select_group(Some(group)).unwrap(); // same target is a no-op
    h.controller().begin_edit(id).unwrap();
    assert!(h.controller().is_dirty());
}

#[test]
fn selection_switch_commits_and_scopes_credentials_to_current_group() {
    let mut h = Harness::new();
    let a = h.selected_group();
    let first = h.saved_row("first");
    let second = h.saved_row("second");
    h.controller().begin_edit(first).unwrap();
    h.controller()
        .update_edit_role("username", "alice".into())
        .unwrap();
    h.controller().select_credential(Some(second)).unwrap();
    assert!(!h.controller().has_edit());
    assert_eq!(h.controller().selected_credential(), Some(second));
    let b = h.controller().create_group("other").unwrap();
    h.controller().select_group(Some(b)).unwrap();
    assert!(matches!(
        h.controller().begin_edit(first),
        Err(ControllerError::CredentialNotFound)
    ));
    assert!(matches!(
        h.controller().reveal_password(first),
        Err(ControllerError::CredentialNotFound)
    ));
    h.controller().select_group(Some(a)).unwrap();
    assert_eq!(
        h.controller()
            .credentials()
            .unwrap()
            .iter()
            .find(|r| r.id == first)
            .unwrap()
            .username,
        "alice"
    );
}

#[test]
fn deleting_selected_group_updates_state_only_after_success() {
    let mut h = Harness::new();
    let group = h.selected_group();
    let id = h.saved_row("old");
    h.controller().begin_edit(id).unwrap();
    h.controller()
        .update_edit_role("password", "pending".into())
        .unwrap();
    h.faults.remove_group.set(true);
    assert!(h.controller().remove_group(group).is_err());
    assert_eq!(h.controller().selected_group(), Some(group));
    assert_eq!(h.controller().draft().unwrap().password, "pending");
    h.faults.remove_group.set(false);
    let saves = h.faults.save_calls.get();
    h.controller().remove_group(group).unwrap();
    assert_eq!(h.faults.save_calls.get(), saves);
    assert_eq!(h.controller().selected_group(), None);
    assert_eq!(h.controller().selected_credential(), None);
    assert!(!h.controller().has_edit());
    assert!(h.disk().groups().unwrap().is_empty());
}

#[test]
fn deleting_other_groups_or_rows_preserves_the_active_draft() {
    let mut h = Harness::new();
    let group = h.selected_group();
    let id = h.saved_row("first");
    let other = h.saved_row("second");
    let other_group = h.controller().create_group("other").unwrap();
    h.controller().begin_edit(id).unwrap();
    h.controller()
        .update_edit_role("password", "pending".into())
        .unwrap();
    h.controller().remove_group(other_group).unwrap();
    h.controller().remove_credential(other).unwrap();
    assert_eq!(h.controller().selected_group(), Some(group));
    assert_eq!(h.controller().draft().unwrap().password, "pending");
    h.faults.remove_credential.set(true);
    assert!(h.controller().remove_credential(id).is_err());
    assert_eq!(h.controller().selected_credential(), Some(id));
    assert!(h.controller().is_dirty());
    h.faults.remove_credential.set(false);
    h.controller().remove_credential(id).unwrap();
    assert!(!h.controller().has_edit());
    assert_eq!(h.controller().selected_credential(), None);
    assert!(h.disk().credentials(group).unwrap().is_empty());
}

#[test]
fn provisional_deletion_cancel_and_password_reveal_are_explicit() {
    let mut h = Harness::new();
    h.selected_group();
    let saved = h.saved_row("original");
    h.controller().begin_edit(saved).unwrap();
    h.controller()
        .update_edit_role("password", "draft".into())
        .unwrap();
    assert_eq!(
        h.controller().reveal_password(saved).unwrap().as_str(),
        "draft"
    );
    h.controller().cancel_edit();
    assert_eq!(
        h.controller().reveal_password(saved).unwrap().as_str(),
        "original"
    );
    assert!(matches!(
        h.controller().update_edit_role("password", "x".into()),
        Err(ControllerError::NoActiveEdit)
    ));
    let new = h.controller().add_credential().unwrap();
    assert!(matches!(
        h.controller().update_edit_role("content_id", "x".into()),
        Err(ControllerError::UnknownField)
    ));
    h.controller()
        .update_edit_role("username", "provisional".into())
        .unwrap();
    assert!(h
        .controller()
        .credentials()
        .unwrap()
        .iter()
        .any(|r| r.id == new && r.username == "provisional"));
    h.faults.remove_credential.set(true);
    h.controller().remove_credential(new).unwrap(); // no persistence for provisional rows
    assert!(!h.controller().has_edit());
    assert_eq!(h.controller().credentials().unwrap().len(), 1);
}

#[test]
fn failed_key_open_and_initial_read_leave_session_locked() {
    let mut h = Harness::new();
    h.app.lock(DraftResolution::Discard).unwrap();
    assert!(matches!(h.app.controller(), Err(ControllerError::Locked)));
    assert!(matches!(
        h.app.unlock(&Provider::new(None)),
        Err(UnlockError::Key(_))
    ));
    assert_eq!(h.app.state(), SessionState::Locked);
    assert!(matches!(
        h.app.unlock(&Provider::new(Some(8))),
        Err(UnlockError::Open(vault_storage::Error::Crypto))
    ));
    assert_eq!(h.app.state(), SessionState::Locked);
    h.faults.groups.set(true);
    let dropped = h.faults.dropped.get();
    assert!(matches!(
        h.app.unlock(&h.provider),
        Err(UnlockError::Repository(_))
    ));
    assert_eq!(h.faults.dropped.get(), dropped + 1);
    assert_eq!(h.app.state(), SessionState::Locked);
    h.faults.groups.set(false);
    h.app.unlock(&h.provider).unwrap();
    let provider = Provider::new(Some(7));
    assert!(matches!(
        h.app.unlock(&provider),
        Err(UnlockError::InvalidState)
    ));
    assert_eq!(provider.calls.get(), 0);
}

#[test]
fn lock_save_failure_and_cancel_preserve_session_discard_releases_repository() {
    let mut h = Harness::new();
    let group = h.selected_group();
    let id = h.saved_row("old");
    h.controller().begin_edit(id).unwrap();
    h.controller()
        .update_edit_role("password", "pending".into())
        .unwrap();
    let dropped = h.faults.dropped.get();
    h.faults.save.set(true);
    assert!(h.app.lock(DraftResolution::Save).is_err());
    assert_eq!(h.app.state(), SessionState::Unlocked);
    assert_eq!(
        h.app.lock(DraftResolution::Cancel).unwrap(),
        TransitionOutcome::Cancelled
    );
    assert_eq!(h.controller().draft().unwrap().password, "pending");
    assert_eq!(h.faults.dropped.get(), dropped);
    h.app.lock(DraftResolution::Discard).unwrap();
    assert_eq!(h.faults.dropped.get(), dropped + 1);
    assert_eq!(h.app.state(), SessionState::Locked);
    assert_eq!(h.disk().credentials(group).unwrap()[0].password, "old");
    h.app.unlock(&h.provider).unwrap();
    assert!(!h.controller().has_edit());
    assert_eq!(h.controller().selected_group(), None);
}

#[test]
fn save_and_lock_persists_and_shutdown_can_be_cancelled_or_retried() {
    let mut h = Harness::new();
    let group = h.selected_group();
    h.controller().add_credential().unwrap();
    h.controller()
        .update_edit_role("password", "saved on lock".into())
        .unwrap();
    h.app.lock(DraftResolution::Save).unwrap();
    assert_eq!(
        h.disk().credentials(group).unwrap()[0].password,
        "saved on lock"
    );
    h.app.unlock(&h.provider).unwrap();
    h.controller().select_group(Some(group)).unwrap();
    h.controller().add_credential().unwrap();
    h.controller()
        .update_edit_role("note", "saved on shutdown".into())
        .unwrap();
    assert_eq!(
        h.app.shutdown(DraftResolution::Cancel).unwrap(),
        TransitionOutcome::Cancelled
    );
    h.faults.save.set(true);
    assert!(h.app.shutdown(DraftResolution::Save).is_err());
    assert_eq!(h.app.state(), SessionState::Unlocked);
    assert_eq!(h.controller().draft().unwrap().note, "saved on shutdown");
    h.faults.save.set(false);
    h.app.shutdown(DraftResolution::Save).unwrap();
    assert_eq!(h.app.state(), SessionState::Closed);
    assert_eq!(h.disk().credentials(group).unwrap().len(), 2);
    assert!(matches!(
        h.app.controller_mut(),
        Err(ControllerError::Closed)
    ));
    assert!(matches!(
        h.app.unlock(&h.provider),
        Err(UnlockError::InvalidState)
    ));
    assert_eq!(
        h.app.shutdown(DraftResolution::Save).unwrap(),
        TransitionOutcome::Completed
    );
}

#[test]
fn discard_shutdown_and_drop_do_not_implicitly_save() {
    for explicit in [true, false] {
        let mut h = Harness::new();
        let group = h.selected_group();
        h.controller().add_credential().unwrap();
        h.controller()
            .update_edit_role("password", "pending".into())
            .unwrap();
        if explicit {
            h.app.shutdown(DraftResolution::Discard).unwrap();
        }
        let path = h.dir.path().join("vault.sqlite");
        drop(h.app);
        assert_eq!(h.faults.save_calls.get(), 0);
        assert_eq!(h.faults.dropped.get(), 1);
        let disk = SqliteRepository::open_existing(&path, h.provider.load_key().unwrap()).unwrap();
        assert!(disk.credentials(group).unwrap().is_empty());
    }
}

#[test]
fn production_factory_opens_existing_vault_and_never_initializes_on_unlock() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.sqlite");
    let provider = Provider::new(Some(7));
    let mut app = VaultApplication::new(SqliteVault::new(&path));
    assert!(matches!(app.unlock(&provider), Err(UnlockError::Open(_))));
    assert!(!path.exists());
    assert_eq!(app.state(), SessionState::Locked);
    std::fs::write(&path, []).unwrap();
    assert!(matches!(
        app.unlock(&provider),
        Err(UnlockError::Open(vault_storage::Error::MigrationRequired))
    ));
    assert_eq!(app.state(), SessionState::Locked);
    drop(SqliteRepository::open(&path, provider.load_key().unwrap()).unwrap());
    app.unlock(&provider).unwrap();
    assert!(app.controller().unwrap().groups().unwrap().is_empty());
    app.shutdown(DraftResolution::Save).unwrap();
    assert_eq!(app.state(), SessionState::Closed);
}

#[test]
fn failed_new_row_save_keeps_one_provisional_draft_until_retry() {
    let mut h = Harness::new();
    let group = h.selected_group();
    let id = h.controller().add_credential().unwrap();
    h.controller()
        .update_edit_role("username", "pending user".into())
        .unwrap();
    h.faults.save.set(true);
    assert!(h.controller().submit_edit().is_err());
    assert!(h.controller().add_credential().is_err());
    assert!(h.disk().credentials(group).unwrap().is_empty());
    assert_eq!(h.controller().credentials().unwrap().len(), 1);
    assert_eq!(h.controller().selected_credential(), Some(id));
    assert_eq!(h.controller().draft().unwrap().username, "pending user");
    h.faults.save.set(false);
    assert_eq!(h.controller().submit_edit().unwrap(), SubmitOutcome::Saved);
    assert_eq!(h.controller().credentials().unwrap().len(), 1);
    assert_eq!(h.disk().credentials(group).unwrap()[0].content_id, id);
    assert!(!h.controller().has_edit());
}
