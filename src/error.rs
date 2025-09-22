use std::fmt;

/// Errors that can occur when working with cloned streams

#[derive(Debug, Clone, PartialEq)]
pub enum CloneStreamError {
    /// The maximum number of clones has been exceeded
    MaxClonesExceeded {
        max_allowed: usize,
        current_count: usize,
    },
}

impl fmt::Display for CloneStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CloneStreamError::MaxClonesExceeded {
                max_allowed,
                current_count,
            } => write!(
                f,
                "Maximum number of clones exceeded: {current_count} >= {max_allowed}"
            ),
        }
    }
}

impl std::error::Error for CloneStreamError {}

pub type Result<T> = std::result::Result<T, CloneStreamError>;
