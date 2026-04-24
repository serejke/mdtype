//! Helpers for `mdtype-tests` integration tests.
//!
//! The crate intentionally has no production code — its unit of work lives in `tests/`.
//! Helpers exposed here are pure test utilities: workspace discovery and binary location.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Absolute path to the workspace root (the directory containing the top-level `Cargo.toml`).
///
/// # Panics
///
/// Panics if `CARGO_MANIFEST_DIR` does not have two parent components — i.e. if this crate
/// is moved out from under `crates/mdtype-tests/`.
#[must_use]
pub fn workspace_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root above crates/mdtype-tests")
            .to_path_buf()
    })
}

/// Build (if needed) and return the absolute path to the `mdtype` binary.
///
/// Uses `escargot` so `cargo test -p mdtype-tests` works standalone — the binary is
/// rebuilt on demand and cached for the duration of the test process.
///
/// # Panics
///
/// Panics if `escargot` cannot build or locate the binary (e.g., the workspace member
/// `crates/mdtype` is missing or fails to compile).
#[must_use]
pub fn mdtype_bin() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let manifest = workspace_root().join("crates/mdtype/Cargo.toml");
        escargot::CargoBuild::new()
            .bin("mdtype")
            .manifest_path(&manifest)
            .run()
            .expect("build mdtype")
            .path()
            .to_path_buf()
    })
}
