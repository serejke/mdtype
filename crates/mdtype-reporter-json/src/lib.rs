//! Structured JSON reporter. The emitted shape is a stable public contract — see
//! `docs/json-schema.md`. Breaking changes bump `CONTRACT_VERSION`.

#![forbid(unsafe_code)]

use std::io;

use mdtype_core::{Diagnostic, Reporter, Summary};
use serde::Serialize;

/// Current JSON contract version. Consumers should assert against this.
pub const CONTRACT_VERSION: &str = "1";

/// Pretty-prints JSON when `pretty` is set, compact otherwise.
///
/// The CLI sets `pretty` based on tty detection so machine consumers (CI, hooks) get a single
/// compact line per run while interactive users get an indented payload.
pub struct JsonReporter {
    /// When `true`, emit pretty-printed JSON (2-space indentation).
    pub pretty: bool,
}

impl JsonReporter {
    /// Construct a reporter. `pretty` is set by the CLI based on tty detection.
    #[must_use]
    pub const fn new(pretty: bool) -> Self {
        Self { pretty }
    }
}

/// Wire payload emitted by [`JsonReporter`]. Field order, names, and types are the public
/// contract; do not reorder or rename without bumping [`CONTRACT_VERSION`].
#[derive(Serialize)]
struct Payload<'a> {
    version: &'static str,
    summary: &'a Summary,
    diagnostics: &'a [Diagnostic],
}

impl Reporter for JsonReporter {
    fn report(
        &self,
        diagnostics: &[Diagnostic],
        summary: &Summary,
        out: &mut dyn io::Write,
    ) -> io::Result<()> {
        let payload = Payload {
            version: CONTRACT_VERSION,
            summary,
            diagnostics,
        };
        let result = if self.pretty {
            serde_json::to_writer_pretty(&mut *out, &payload)
        } else {
            serde_json::to_writer(&mut *out, &payload)
        };
        result.map_err(io::Error::other)?;
        writeln!(out)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mdtype_core::{Diagnostic, Fixit, Reporter, Severity, Summary};

    use super::JsonReporter;

    fn fixture_diagnostics() -> Vec<Diagnostic> {
        vec![
            Diagnostic {
                file: PathBuf::from("content/posts/hello.md"),
                line: None,
                rule: "frontmatter.schema",
                severity: Severity::Error,
                message: "missing required field 'author'".into(),
                fixit: Some(Fixit::AddFrontmatterField {
                    field: "author".into(),
                    hint: Some("string".into()),
                }),
            },
            Diagnostic {
                file: PathBuf::from("content/posts/stray.md"),
                line: Some(7),
                rule: "body.forbid_h1",
                severity: Severity::Error,
                message: "top-level H1 is not allowed".into(),
                fixit: Some(Fixit::DeleteLine { line: 7 }),
            },
        ]
    }

    fn fixture_summary() -> Summary {
        Summary {
            files_scanned: 4,
            files_with_errors: 2,
            errors: 2,
            warnings: 0,
        }
    }

    fn render(reporter: &JsonReporter, diags: &[Diagnostic], summary: &Summary) -> String {
        let mut buf: Vec<u8> = Vec::new();
        reporter.report(diags, summary, &mut buf).expect("report");
        String::from_utf8(buf).expect("utf-8")
    }

    #[test]
    fn pretty_output_snapshot() {
        let out = render(
            &JsonReporter::new(true),
            &fixture_diagnostics(),
            &fixture_summary(),
        );
        insta::assert_snapshot!("payload_pretty", out);
    }

    #[test]
    fn compact_output_snapshot() {
        let out = render(
            &JsonReporter::new(false),
            &fixture_diagnostics(),
            &fixture_summary(),
        );
        insta::assert_snapshot!("payload_compact", out);
    }

    #[test]
    fn empty_run_still_emits_version_and_summary() {
        let summary = Summary {
            files_scanned: 7,
            files_with_errors: 0,
            errors: 0,
            warnings: 0,
        };
        let out = render(&JsonReporter::new(true), &[], &summary);
        insta::assert_snapshot!("payload_clean_pretty", out);
    }
}
