//! Structured JSON reporter. The emitted shape is a stable public contract — see
//! `docs/json-schema.md`. Breaking changes bump `CONTRACT_VERSION`.

#![forbid(unsafe_code)]

use std::io;

use mdtype_core::{Diagnostic, Reporter, Summary};
use serde::Serialize;

/// Current JSON contract version. Consumers should assert against this.
pub const CONTRACT_VERSION: &str = "1";

/// Pretty-prints JSON when stdout is a tty, compact otherwise.
pub struct JsonReporter {
    /// When `true`, emit pretty-printed JSON.
    pub pretty: bool,
}

impl JsonReporter {
    /// Construct a reporter. `pretty` is set by the CLI based on tty detection.
    #[must_use]
    pub const fn new(pretty: bool) -> Self {
        Self { pretty }
    }
}

/// Wire payload emitted by `JsonReporter`. Field shape is the public contract.
#[derive(Serialize)]
#[allow(dead_code, reason = "populated in Phase 4.1")]
struct Payload<'a> {
    version: &'static str,
    summary: &'a Summary,
    diagnostics: &'a [Diagnostic],
}

impl Reporter for JsonReporter {
    fn report(
        &self,
        _diagnostics: &[Diagnostic],
        _summary: &Summary,
        _out: &mut dyn io::Write,
    ) -> io::Result<()> {
        todo!("implemented in Phase 4.1")
    }
}
