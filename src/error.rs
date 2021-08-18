use std::{error::Error, fmt};

#[derive(Debug)]
pub enum OutdatedError {
    CannotElaborateWorkspace,
    EmptyPath,
    NoWorkspace,
    NoMatchingDependency,
}

impl fmt::Display for OutdatedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutdatedError::CannotElaborateWorkspace => write!(f, "Cannot elaborate the workspace"),
            OutdatedError::EmptyPath => write!(f, "Empty path cannot get last"),
            OutdatedError::NoWorkspace => write!(f, "No workspace"),
            OutdatedError::NoMatchingDependency => write!(f, "No matching dependency"),
        }
    }
}

impl std::error::Error for OutdatedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            OutdatedError::CannotElaborateWorkspace => None,
            OutdatedError::EmptyPath => None,
            OutdatedError::NoWorkspace => None,
            OutdatedError::NoMatchingDependency => None,
        }
    }
}
