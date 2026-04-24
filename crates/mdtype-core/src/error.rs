//! Shared error type for the core crate.

use std::path::PathBuf;

/// Errors produced by parsing, schema loading, and validation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to read a file from disk.
    #[error("failed to read {path}: {source}")]
    Io {
        /// File that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Frontmatter YAML block was malformed.
    #[error("malformed frontmatter in {path}: {message}")]
    Frontmatter {
        /// File that contains the malformed frontmatter.
        path: PathBuf,
        /// Human-readable error message from the YAML parser.
        message: String,
    },

    /// Schema file or config is invalid.
    #[error("invalid schema: {0}")]
    Schema(String),

    /// Catch-all for integration errors reported by downstream crates.
    #[error("{0}")]
    Other(String),
}
