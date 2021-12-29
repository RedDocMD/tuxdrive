use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TuxDriveError {
    #[error("{0} is not a directory")]
    NotDirectory(String),

    #[error("Home directory not found")]
    HomeDirNotFound,

    #[error("Config directory not found")]
    ConfigDirNotFound,

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to parse config: {0}")]
    DeserializeFailed(#[from] serde_json::Error),

    #[error("Failed to find config file")]
    ConfigFileNotFound,

    #[error("{0} is not an absolute path")]
    PathNotAbs(String),

    #[error("Platform error: {0}")]
    NixError(#[from] nix::errno::Errno),

    #[error("Thread pool build error: {0}")]
    ThreadPoolBuildError(#[from] rayon::ThreadPoolBuildError),
}

pub type TuxDriveResult<T> = Result<T, TuxDriveError>;
