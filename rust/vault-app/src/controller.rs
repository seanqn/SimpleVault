use vault_core::{
    repository::{Group, GroupId, VaultRepository},
    Credential, CredentialEditCache, CredentialField, CredentialId, EditError,
};
use zeroize::{Zeroize, Zeroizing};

#[derive(Debug, thiserror::Error)]
pub enum ControllerError<E> {
    #[error("repository operation failed")]
    Repository(E),
    #[error("vault is locked")]
    Locked,
    #[error("application is closed")]
    Closed,
    #[error("select a group first")]
    NoGroupSelected,
    #[error("credential not found in selected group")]
    CredentialNotFound,
    #[error("group name cannot be empty")]
    EmptyGroupName,
    #[error("no active edit")]
    NoActiveEdit,
    #[error("unknown editable field")]
    UnknownField,
}

/// List responses deliberately exclude passwords. No Debug/serialization of secrets.
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialSummary {
    pub id: CredentialId,
    pub org_name: String,
    pub username: String,
    pub email: String,
    pub note: String,
}
impl From<&Credential> for CredentialSummary {
    fn from(row: &Credential) -> Self {
        Self {
            id: row.content_id,
            org_name: row.org_name.clone(),
            username: row.username.clone(),
            email: row.email.clone(),
            note: row.note.clone(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitOutcome {
    NoEdit,
    Unchanged,
    DiscardedEmptyNewRow,
    Saved,
}

/// All commands are synchronous and serialized through mutable access. No implicit
/// destructor saves: use the application's explicit lock/shutdown commands.
pub struct VaultController<R: VaultRepository> {
    repository: R,
    selected_group: Option<GroupId>,
    selected_credential: Option<CredentialId>,
    edit: CredentialEditCache,
}
impl<R: VaultRepository> VaultController<R> {
    pub fn new(repository: R) -> Result<Self, ControllerError<R::Error>> {
        // Validate initial access before an authentication transition is published.
        repository.groups().map_err(ControllerError::Repository)?;
        Ok(Self {
            repository,
            selected_group: None,
            selected_credential: None,
            edit: CredentialEditCache::default(),
        })
    }
    pub fn selected_group(&self) -> Option<GroupId> {
        self.selected_group
    }
    pub fn selected_credential(&self) -> Option<CredentialId> {
        self.selected_credential
    }
    pub fn has_edit(&self) -> bool {
        self.edit.is_active()
    }
    pub fn is_dirty(&self) -> bool {
        self.edit.is_active() && !self.edit.is_clean()
    }
    /// Backend-only access to the active draft. Do not serialize it as a list row.
    pub fn draft(&self) -> Option<&Credential> {
        self.edit.draft()
    }
    pub fn groups(&self) -> Result<Vec<Group>, ControllerError<R::Error>> {
        self.repository
            .groups()
            .map_err(ControllerError::Repository)
    }
    pub fn create_group(&mut self, name: &str) -> Result<GroupId, ControllerError<R::Error>> {
        let name = group_name(name)?;
        let id = *uuid::Uuid::now_v7().as_bytes();
        self.repository
            .add_group(id, name)
            .map_err(ControllerError::Repository)?;
        Ok(id)
    }
    pub fn rename_group(
        &mut self,
        id: GroupId,
        name: &str,
    ) -> Result<(), ControllerError<R::Error>> {
        self.repository
            .rename_group(id, group_name(name)?)
            .map_err(ControllerError::Repository)
    }
    /// An explicit successful deletion discards a draft belonging to this group.
    /// Failure preserves all local state. Deleting another group leaves it intact.
    pub fn remove_group(&mut self, id: GroupId) -> Result<(), ControllerError<R::Error>> {
        self.repository
            .remove_group(id)
            .map_err(ControllerError::Repository)?;
        if self.selected_group == Some(id) {
            self.edit.reset();
            self.selected_group = None;
            self.selected_credential = None;
        }
        Ok(())
    }
    pub fn select_group(&mut self, id: Option<GroupId>) -> Result<(), ControllerError<R::Error>> {
        if self.selected_group == id {
            return Ok(());
        }
        if let Some(id) = id {
            // Preflight target access before saving or changing current state.
            let _rows = Zeroizing::new(
                self.repository
                    .credentials(id)
                    .map_err(ControllerError::Repository)?,
            );
        }
        self.submit_edit()?;
        self.selected_group = id;
        self.selected_credential = None;
        Ok(())
    }
    pub fn credentials(&self) -> Result<Vec<CredentialSummary>, ControllerError<R::Error>> {
        let Some(group) = self.selected_group else {
            return Ok(Vec::new());
        };
        let rows = Zeroizing::new(
            self.repository
                .credentials(group)
                .map_err(ControllerError::Repository)?,
        );
        let mut summaries: Vec<_> = rows.iter().map(CredentialSummary::from).collect();
        if let Some(draft) = self.edit.draft() {
            let summary = CredentialSummary::from(draft);
            if let Some(row) = summaries.iter_mut().find(|row| row.id == draft.content_id) {
                *row = summary;
            } else if self.edit.default_row_exists() {
                summaries.push(summary);
            }
        }
        Ok(summaries)
    }
    fn stored_credential(
        &self,
        id: CredentialId,
    ) -> Result<Zeroizing<Credential>, ControllerError<R::Error>> {
        let group = self
            .selected_group
            .ok_or(ControllerError::NoGroupSelected)?;
        let rows = Zeroizing::new(
            self.repository
                .credentials(group)
                .map_err(ControllerError::Repository)?,
        );
        let row = rows
            .iter()
            .find(|row| row.content_id == id)
            .ok_or(ControllerError::CredentialNotFound)?;
        Ok(Zeroizing::new(row.clone()))
    }
    pub fn reveal_password(
        &self,
        id: CredentialId,
    ) -> Result<Zeroizing<String>, ControllerError<R::Error>> {
        if let Some(draft) = self.edit.draft().filter(|draft| draft.content_id == id) {
            return Ok(Zeroizing::new(draft.password.clone()));
        }
        Ok(Zeroizing::new(self.stored_credential(id)?.password.clone()))
    }
    pub fn select_credential(
        &mut self,
        id: Option<CredentialId>,
    ) -> Result<(), ControllerError<R::Error>> {
        if self.selected_credential == id {
            return Ok(());
        }
        if let Some(id) = id {
            self.stored_credential(id)?;
        }
        self.submit_edit()?;
        self.selected_credential = id;
        Ok(())
    }
    pub fn begin_edit(&mut self, id: CredentialId) -> Result<(), ControllerError<R::Error>> {
        if self
            .edit
            .draft()
            .is_some_and(|draft| draft.content_id == id)
        {
            return Ok(());
        }
        let row = self.stored_credential(id)?;
        self.submit_edit()?;
        self.edit.start_edit((*row).clone());
        self.selected_credential = Some(id);
        Ok(())
    }
    /// Keep one blank provisional row. A filled provisional row is saved before
    /// creating another; failed saves retain the existing draft and selection.
    pub fn add_credential(&mut self) -> Result<CredentialId, ControllerError<R::Error>> {
        self.selected_group
            .ok_or(ControllerError::NoGroupSelected)?;
        if self.edit.default_row_exists() && self.edit.is_empty() {
            return Ok(self
                .edit
                .draft()
                .expect("provisional row has a draft")
                .content_id);
        }
        self.submit_edit()?;
        let id = *uuid::Uuid::now_v7().as_bytes();
        self.edit.start_edit(Credential {
            content_id: id,
            ..Credential::default()
        });
        self.edit.set_default_row_exists(true);
        self.selected_credential = Some(id);
        Ok(id)
    }
    pub fn update_edit(
        &mut self,
        field: CredentialField,
        value: String,
    ) -> Result<(), ControllerError<R::Error>> {
        self.edit.update(field, value).map_err(|error| match error {
            EditError::NoActiveEdit => ControllerError::NoActiveEdit,
            EditError::UnknownField => ControllerError::UnknownField,
        })
    }
    pub fn update_edit_role(
        &mut self,
        role: &str,
        value: String,
    ) -> Result<(), ControllerError<R::Error>> {
        let mut value = value;
        let Ok(field) = CredentialField::try_from(role) else {
            value.zeroize();
            return Err(ControllerError::UnknownField);
        };
        self.update_edit(field, value)
    }
    pub fn submit_edit(&mut self) -> Result<SubmitOutcome, ControllerError<R::Error>> {
        let Some(draft) = self.edit.draft() else {
            return Ok(SubmitOutcome::NoEdit);
        };
        let outcome = if self.edit.default_row_exists() && self.edit.is_empty() {
            self.selected_credential = None;
            SubmitOutcome::DiscardedEmptyNewRow
        } else if !self.edit.default_row_exists() && self.edit.is_clean() {
            SubmitOutcome::Unchanged
        } else {
            let group = self
                .selected_group
                .ok_or(ControllerError::NoGroupSelected)?;
            self.repository
                .save_credentials(group, std::slice::from_ref(draft))
                .map_err(ControllerError::Repository)?;
            SubmitOutcome::Saved
        };
        self.edit.reset();
        Ok(outcome)
    }
    pub fn cancel_edit(&mut self) {
        if self.edit.default_row_exists() {
            self.selected_credential = None;
        }
        self.edit.reset();
    }
    pub fn remove_credential(&mut self, id: CredentialId) -> Result<(), ControllerError<R::Error>> {
        let group = self
            .selected_group
            .ok_or(ControllerError::NoGroupSelected)?;
        let editing_target = self
            .edit
            .draft()
            .is_some_and(|draft| draft.content_id == id);
        if !(editing_target && self.edit.default_row_exists()) {
            self.repository
                .remove_credential(group, id)
                .map_err(ControllerError::Repository)?;
        }
        if editing_target {
            self.edit.reset();
        }
        if self.selected_credential == Some(id) {
            self.selected_credential = None;
        }
        Ok(())
    }
}
fn group_name<E>(name: &str) -> Result<&str, ControllerError<E>> {
    let name = name.trim();
    if name.is_empty() {
        Err(ControllerError::EmptyGroupName)
    } else {
        Ok(name)
    }
}
