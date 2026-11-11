use crate::{ControllerError, VaultController};
use vault_core::{
    crypto::{KeyProvider, SecretKey},
    repository::VaultRepository,
};

/// Opens a repository with an owned key; the factory must not retain that key.
pub trait VaultFactory {
    type Repository: VaultRepository;
    type Error;
    fn open(&self, key: SecretKey) -> Result<Self::Repository, Self::Error>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Locked,
    Unlocked,
    Closed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftResolution {
    Save,
    Discard,
    Cancel,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionOutcome {
    Completed,
    Cancelled,
}
#[derive(Debug, thiserror::Error)]
pub enum UnlockError<K, O, R> {
    #[error("unlock requires a locked session")]
    InvalidState,
    #[error("key acquisition failed")]
    Key(K),
    #[error("vault open failed")]
    Open(O),
    #[error("initial repository read failed")]
    Repository(ControllerError<R>),
}

type RepositoryError<F> = <<F as VaultFactory>::Repository as VaultRepository>::Error;
pub type UnlockResult<F, P> = Result<
    (),
    UnlockError<<P as KeyProvider>::Error, <F as VaultFactory>::Error, RepositoryError<F>>,
>;

/// Starts locked. Only successful key acquisition, opening, and initial reads
/// publish an unlocked controller. No destructor I/O and no cached provider keys.
pub struct VaultApplication<F: VaultFactory> {
    factory: F,
    controller: Option<VaultController<F::Repository>>,
    closed: bool,
}
impl<F: VaultFactory> VaultApplication<F> {
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            controller: None,
            closed: false,
        }
    }
    pub fn state(&self) -> SessionState {
        if self.closed {
            SessionState::Closed
        } else if self.controller.is_some() {
            SessionState::Unlocked
        } else {
            SessionState::Locked
        }
    }
    pub fn controller(
        &self,
    ) -> Result<&VaultController<F::Repository>, ControllerError<RepositoryError<F>>> {
        self.controller.as_ref().ok_or(if self.closed {
            ControllerError::Closed
        } else {
            ControllerError::Locked
        })
    }
    pub fn controller_mut(
        &mut self,
    ) -> Result<&mut VaultController<F::Repository>, ControllerError<RepositoryError<F>>> {
        self.controller.as_mut().ok_or(if self.closed {
            ControllerError::Closed
        } else {
            ControllerError::Locked
        })
    }
    pub fn unlock<P: KeyProvider>(&mut self, provider: &P) -> UnlockResult<F, P> {
        if self.state() != SessionState::Locked {
            return Err(UnlockError::InvalidState);
        }
        let key = provider.load_key().map_err(UnlockError::Key)?;
        self.unlock_key(key)
    }
    pub fn unlock_key<K>(
        &mut self,
        key: SecretKey,
    ) -> Result<(), UnlockError<K, F::Error, RepositoryError<F>>> {
        if self.state() != SessionState::Locked {
            return Err(UnlockError::InvalidState);
        }
        let repository = self.factory.open(key).map_err(UnlockError::Open)?;
        let controller = VaultController::new(repository).map_err(UnlockError::Repository)?;
        self.controller = Some(controller);
        Ok(())
    }
    /// Save failure leaves the session unlocked with its draft available for retry.
    /// For an immediate security lock, the host must explicitly choose Discard.
    pub fn lock(
        &mut self,
        resolution: DraftResolution,
    ) -> Result<TransitionOutcome, ControllerError<RepositoryError<F>>> {
        if self.closed {
            return Err(ControllerError::Closed);
        }
        self.finish_session(resolution)
    }
    pub fn shutdown(
        &mut self,
        resolution: DraftResolution,
    ) -> Result<TransitionOutcome, ControllerError<RepositoryError<F>>> {
        if self.closed {
            return Ok(TransitionOutcome::Completed);
        }
        let outcome = self.finish_session(resolution)?;
        if outcome == TransitionOutcome::Completed {
            self.closed = true;
        }
        Ok(outcome)
    }
    fn finish_session(
        &mut self,
        resolution: DraftResolution,
    ) -> Result<TransitionOutcome, ControllerError<RepositoryError<F>>> {
        if resolution == DraftResolution::Cancel {
            return Ok(TransitionOutcome::Cancelled);
        }
        if let Some(controller) = &mut self.controller {
            match resolution {
                DraftResolution::Save => {
                    controller.submit_edit()?;
                }
                DraftResolution::Discard => controller.cancel_edit(),
                DraftResolution::Cancel => unreachable!(),
            }
        }
        // Dropping releases the repository/key and zeroizes owned edit buffers.
        self.controller = None;
        Ok(TransitionOutcome::Completed)
    }
}
