//! Recoverable table operations.

use std::fmt;

/// Errors from table operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// `nil` cannot be used as a key.
    NilKey,
    /// The value could not be used as a key (for example a NaN number), or a
    /// key passed to [`Table::next`](crate::Table::next) is not in the table.
    InvalidKey,
    /// An operation expected a different dynamic type.
    TypeMismatch {
        /// What the caller needed.
        expected: &'static str,
        /// What was actually stored.
        found: &'static str,
    },
    /// Two values have no defined order.
    Incomparable {
        left: &'static str,
        right: &'static str,
    },
    /// A sequence position was outside the range the operation allows.
    PositionOutOfBounds {
        /// The 1-based position that was requested.
        position: i64,
        /// The sequence length at the time of the call.
        len: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NilKey => write!(f, "nil cannot be used as a table key"),
            Self::InvalidKey => write!(f, "value cannot be used as a table key"),
            Self::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected {expected}, found {found}")
            }
            Self::Incomparable { left, right } => {
                write!(f, "attempt to compare {left} with {right}")
            }
            Self::PositionOutOfBounds { position, len } => {
                write!(f, "position {position} out of bounds (length {len})")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result of a table operation.
pub type Result<T> = std::result::Result<T, Error>;
