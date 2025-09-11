use std::fmt;

/// Errors that can occur when working with cloned streams
#[derive(Debug, Clone, PartialEq)]
pub enum CloneStreamError {
    /// The maximum number of clones has been exceeded
    MaxClonesExceeded {
        max_allowed: usize,
        current_count: usize,
    },
    /// The maximum queue size has been exceeded
    MaxQueueSizeExceeded {
        max_allowed: usize,
        current_size: usize,
    },
    /// Attempted to access a clone that doesn't exist
    CloneNotFound { clone_id: usize },
    /// The fork is in an invalid state
    InvalidForkState { message: String },
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
            CloneStreamError::MaxQueueSizeExceeded {
                max_allowed,
                current_size,
            } => write!(
                f,
                "Maximum queue size exceeded: {current_size} >= {max_allowed}"
            ),
            CloneStreamError::CloneNotFound { clone_id } => {
                write!(f, "Clone with ID {clone_id} not found")
            }
            CloneStreamError::InvalidForkState { message } => {
                write!(f, "Invalid fork state: {message}")
            }
        }
    }
}

impl std::error::Error for CloneStreamError {}

/// Result type for clone stream operations
pub type Result<T> = std::result::Result<T, CloneStreamError>;
