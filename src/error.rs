use std::fmt::Display;

use std::env::VarError;
use std::io::Error as IoError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    GitError(git2::Error),
    MissingCommand,
    EnvError(VarError),
    IoError(IoError),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::GitError(err) => write!(f, "Git Error: {}", err.message()),
            Error::MissingCommand => write!(f, "Missing command"),
            Error::EnvError(err) => write!(f, "Missing env var: {}", err),
            Error::IoError(err) => write!(f, "io error occured: {}", err),
        }
    }
}

impl From<git2::Error> for Error {
    fn from(err: git2::Error) -> Self {
        Error::GitError(err)
    }
}

impl From<VarError> for Error {
    fn from(err: VarError) -> Self {
        Error::EnvError(err)
    }
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Self {
        Error::IoError(err)
    }
}
