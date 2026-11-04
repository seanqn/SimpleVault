use crate::Credential;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialField {
    Organization,
    Username,
    Password,
    Email,
    Note,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditError {
    NoActiveEdit,
    UnknownField,
}

impl TryFrom<&str> for CredentialField {
    type Error = EditError;

    fn try_from(role: &str) -> Result<Self, Self::Error> {
        match role {
            "org_name" => Ok(Self::Organization),
            "username" => Ok(Self::Username),
            "password" => Ok(Self::Password),
            "email" => Ok(Self::Email),
            "note" => Ok(Self::Note),
            _ => Err(EditError::UnknownField),
        }
    }
}

/// Holds a draft until the application layer successfully commits it.
/// Reset explicitly on cancel, successful save, or vault lock; never on failed save.
/// Strings are ordinary allocations, not guaranteed zeroized secret storage.
#[derive(Default)]
pub struct CredentialEditCache {
    session: Option<EditSession>,
    default_row_exists: bool,
}

struct EditSession {
    original: Credential,
    draft: Credential,
}

impl CredentialEditCache {
    pub fn is_active(&self) -> bool {
        self.session.is_some()
    }

    pub fn default_row_exists(&self) -> bool {
        self.default_row_exists
    }

    pub fn set_default_row_exists(&mut self, exists: bool) {
        self.default_row_exists = exists;
    }

    /// Caller must resolve the previous draft before starting a different edit.
    pub fn start_edit(&mut self, credential: Credential) {
        self.session = Some(EditSession {
            original: credential.clone(),
            draft: credential,
        });
    }

    pub fn draft(&self) -> Option<&Credential> {
        self.session.as_ref().map(|session| &session.draft)
    }

    pub fn update(&mut self, field: CredentialField, value: String) -> Result<(), EditError> {
        let draft = &mut self.session.as_mut().ok_or(EditError::NoActiveEdit)?.draft;
        let column = match field {
            CredentialField::Organization => &mut draft.org_name,
            CredentialField::Username => &mut draft.username,
            CredentialField::Password => &mut draft.password,
            CredentialField::Email => &mut draft.email,
            CredentialField::Note => &mut draft.note,
        };
        *column = value;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.draft().is_none_or(|draft| {
            draft.org_name.is_empty()
                && draft.username.is_empty()
                && draft.password.is_empty()
                && draft.email.is_empty()
                && draft.note.is_empty()
        })
    }

    pub fn is_clean(&self) -> bool {
        self.session
            .as_ref()
            .is_none_or(|session| session.original == session.draft)
    }

    /// Clear both edit and provisional-row state, unlike the legacy partial reset.
    pub fn reset(&mut self) {
        self.session = None;
        self.default_row_exists = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_cache_rejects_updates() {
        let mut cache = CredentialEditCache::default();
        assert!(!cache.is_active());
        assert!(cache.is_empty());
        assert!(cache.is_clean());
        assert!(cache.draft().is_none());
        assert_eq!(
            cache.update(CredentialField::Username, "alice".into()),
            Err(EditError::NoActiveEdit)
        );
    }

    #[test]
    fn edits_all_columns_preserving_identity_and_supports_revert() {
        for (role, field) in [
            ("org_name", CredentialField::Organization),
            ("username", CredentialField::Username),
            ("password", CredentialField::Password),
            ("email", CredentialField::Email),
            ("note", CredentialField::Note),
        ] {
            assert_eq!(CredentialField::try_from(role), Ok(field));
            let mut cache = CredentialEditCache::default();
            cache.start_edit(Credential {
                content_id: [42; 16],
                ..Credential::default()
            });
            assert!(cache.is_clean());
            cache.update(field, "秘密🔐".into()).unwrap();
            let draft = cache.draft().unwrap();
            let actual = match field {
                CredentialField::Organization => &draft.org_name,
                CredentialField::Username => &draft.username,
                CredentialField::Password => &draft.password,
                CredentialField::Email => &draft.email,
                CredentialField::Note => &draft.note,
            };
            assert_eq!(actual, "秘密🔐");
            assert_eq!(draft.content_id, [42; 16]);
            assert!(!cache.is_empty());
            assert!(!cache.is_clean());
            cache.update(field, String::new()).unwrap();
            assert!(cache.is_clean());
            assert!(cache.is_empty());
        }
        assert_eq!(
            CredentialField::try_from("content_id"),
            Err(EditError::UnknownField)
        );
    }

    #[test]
    fn reset_clears_session_and_provisional_row() {
        let mut cache = CredentialEditCache::default();
        cache.start_edit(Credential {
            password: "secret".into(),
            ..Credential::default()
        });
        cache.set_default_row_exists(true);
        assert!(cache.default_row_exists());
        cache.reset();
        assert!(!cache.is_active());
        assert!(cache.draft().is_none());
        assert!(!cache.default_row_exists());
        cache.start_edit(Credential::default());
        assert!(cache.is_empty());
        assert!(cache.is_clean());
    }
}
