//! Error types for the Uxie library
//!
//! This module provides a unified error type [`UxieError`] that covers all
//! error conditions that can occur during ROM parsing, project loading,
//! and symbol resolution.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for Uxie operations
pub type Result<T> = std::result::Result<T, UxieError>;

/// Unified error type for all Uxie operations
///
/// This enum covers all error conditions that can occur during:
/// - File I/O operations
/// - Binary format parsing (NARC, ARM9, ROM headers)
/// - JSON/YAML configuration parsing
/// - Symbol resolution
/// - Project detection and loading
#[derive(Debug, Error)]
pub enum UxieError {
    /// I/O error (file not found, permission denied, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing/serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML parsing/serialization error
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Dialogue text formatting error from chatot (wrapping, measuring, etc.)
    #[error("Text formatting error: {0}")]
    TextFormat(#[from] chatot::FormatError),

    /// Invalid binary format (wrong magic bytes, corrupted data, etc.)
    #[error("Invalid format: {message}")]
    InvalidFormat {
        /// Description of what was expected vs found
        message: String,
    },

    /// Resource not found (map header, script, text bank, etc.)
    #[error("{resource_type} not found: {id}")]
    NotFound {
        /// Type of resource (e.g., "Map header", "Script file")
        resource_type: &'static str,
        /// Identifier that was searched for
        id: String,
    },

    /// Index out of bounds
    #[error("{resource_type} ID {id} out of range (max: {max})")]
    OutOfBounds {
        /// Type of resource
        resource_type: &'static str,
        /// The requested ID
        id: u32,
        /// Maximum valid ID
        max: u32,
    },

    /// Project detection failed
    #[error("Could not detect project type at {}", path.display())]
    ProjectDetectionFailed {
        /// Path that was examined
        path: PathBuf,
    },

    /// Game detection failed
    #[error("Could not detect game from ROM header")]
    GameDetectionFailed,

    /// Symbol resolution failed
    #[error("Failed to resolve symbol: {name}")]
    SymbolResolutionFailed {
        /// Symbol name that couldn't be resolved
        name: String,
    },

    /// Missing required file or directory
    #[error("Missing required {item_type}: {}", path.display())]
    MissingRequired {
        /// What kind of item is missing (e.g., "file", "directory")
        item_type: &'static str,
        /// Path to the missing item
        path: PathBuf,
    },
}

impl UxieError {
    /// Create an InvalidFormat error with a message
    pub fn invalid_format(message: impl Into<String>) -> Self {
        Self::InvalidFormat {
            message: message.into(),
        }
    }

    /// Create a NotFound error
    pub fn not_found(resource_type: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            resource_type,
            id: id.into(),
        }
    }

    /// Create an OutOfBounds error
    pub fn out_of_bounds(resource_type: &'static str, id: u32, max: u32) -> Self {
        Self::OutOfBounds {
            resource_type,
            id,
            max,
        }
    }

    /// Create a MissingRequired error
    pub fn missing_file(path: impl Into<PathBuf>) -> Self {
        Self::MissingRequired {
            item_type: "file",
            path: path.into(),
        }
    }

    /// Create a MissingRequired error for directories
    pub fn missing_directory(path: impl Into<PathBuf>) -> Self {
        Self::MissingRequired {
            item_type: "directory",
            path: path.into(),
        }
    }
}

impl From<UxieError> for std::io::Error {
    fn from(err: UxieError) -> Self {
        match err {
            UxieError::Io(e) => e,
            UxieError::Json(e) => std::io::Error::new(std::io::ErrorKind::InvalidData, e),
            UxieError::Yaml(e) => std::io::Error::new(std::io::ErrorKind::InvalidData, e),
            UxieError::TextFormat(e) => std::io::Error::new(std::io::ErrorKind::InvalidData, e),
            UxieError::InvalidFormat { message } => {
                std::io::Error::new(std::io::ErrorKind::InvalidData, message)
            }
            UxieError::NotFound { resource_type, id } => std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{} not found: {}", resource_type, id),
            ),
            UxieError::OutOfBounds {
                resource_type,
                id,
                max,
            } => std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{} ID {} out of range (max: {})", resource_type, id, max),
            ),
            UxieError::ProjectDetectionFailed { path } => std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Could not detect project type at {}", path.display()),
            ),
            UxieError::GameDetectionFailed => std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Could not detect game from ROM header",
            ),
            UxieError::SymbolResolutionFailed { name } => std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Failed to resolve symbol: {}", name),
            ),
            UxieError::MissingRequired { item_type, path } => std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Missing required {}: {}", item_type, path.display()),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let uxie_err: UxieError = io_err.into();
        assert!(matches!(uxie_err, UxieError::Io(_)));
        assert!(uxie_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_invalid_format() {
        let err = UxieError::invalid_format("Not a NARC file");
        assert!(matches!(err, UxieError::InvalidFormat { .. }));
        assert_eq!(err.to_string(), "Invalid format: Not a NARC file");
    }

    #[test]
    fn test_not_found() {
        let err = UxieError::not_found("Map header", "42");
        assert!(matches!(err, UxieError::NotFound { .. }));
        assert_eq!(err.to_string(), "Map header not found: 42");
    }

    #[test]
    fn test_out_of_bounds() {
        let err = UxieError::out_of_bounds("Map header", 600, 558);
        assert!(matches!(err, UxieError::OutOfBounds { .. }));
        assert_eq!(err.to_string(), "Map header ID 600 out of range (max: 558)");
    }

    #[test]
    fn test_missing_file() {
        let err = UxieError::missing_file("/path/to/arm9.bin");
        assert!(matches!(err, UxieError::MissingRequired { .. }));
        assert!(err.to_string().contains("arm9.bin"));
    }

    #[test]
    fn test_backward_compat_io_conversion() {
        let uxie_err = UxieError::not_found("Script", "main");
        let io_err: std::io::Error = uxie_err.into();
        assert_eq!(io_err.kind(), std::io::ErrorKind::NotFound);
    }
}
