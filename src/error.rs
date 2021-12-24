use thiserror::Error;

#[derive(Debug, Error)]
pub enum TuxDriveError {
    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),

    #[error("{0} is not a directory")]
    NotDirectory(String),
}

pub type TuxDriveResult<T> = Result<T, TuxDriveError>;
