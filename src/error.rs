use std::fmt::Debug;

use thiserror::Error;

use crate::dic::header::HeaderError;

pub type SudachiResult<T> = Result<T, SudachiError>;

/// Sudachi error
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SudachiError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid header: {0}")]
    InvalidHeader(#[from] HeaderError),
}
