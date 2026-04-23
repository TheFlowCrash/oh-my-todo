use crate::domain::{IdError, ReferenceError, ValidationError};
use crate::storage::StorageError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Id(#[from] IdError),
    #[error(transparent)]
    Reference(#[from] ReferenceError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
}
