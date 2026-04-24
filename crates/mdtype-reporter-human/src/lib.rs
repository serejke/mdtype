//! Human-readable reporter: diagnostics grouped by file, optional color.

#![forbid(unsafe_code)]

use std::io;

use mdtype_core::{Diagnostic, Reporter, Summary};

/// Pretty-printing reporter. Construct with `HumanReporter::new()`; the color policy follows
/// `--no-color` plus tty detection wired in at the CLI layer.
pub struct HumanReporter {
    /// Whether to emit ANSI color escapes. Ignored when stdout is not a tty.
    pub color: bool,
}

impl HumanReporter {
    /// Construct a reporter with color enabled.
    #[must_use]
    pub fn new() -> Self {
        Self { color: true }
    }
}

impl Default for HumanReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Reporter for HumanReporter {
    fn report(
        &self,
        _diagnostics: &[Diagnostic],
        _summary: &Summary,
        _out: &mut dyn io::Write,
    ) -> io::Result<()> {
        todo!("implemented in Phase 2.2")
    }
}
