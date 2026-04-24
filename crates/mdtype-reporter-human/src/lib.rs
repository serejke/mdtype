//! Human-readable reporter: diagnostics grouped by file, optional color.

#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::path::Path;

use mdtype_core::{Diagnostic, Reporter, Severity, Summary};
use owo_colors::OwoColorize;

/// Pretty-printing reporter.
///
/// Diagnostics are emitted grouped by file with a bold header, one indented line per
/// diagnostic, and a trailing summary. Color follows `color`; the CLI also clears it when
/// stdout is not a tty or `--no-color` is set.
pub struct HumanReporter {
    /// Whether to emit ANSI color escapes.
    pub color: bool,
    /// When `true`, the trailing summary line is suppressed (mirrors `mdtype --quiet`).
    pub quiet: bool,
}

impl HumanReporter {
    /// Construct a reporter with the given color policy and the summary line enabled.
    #[must_use]
    pub const fn new(color: bool) -> Self {
        Self {
            color,
            quiet: false,
        }
    }

    /// Enable or disable the summary line.
    #[must_use]
    pub const fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }
}

impl Default for HumanReporter {
    fn default() -> Self {
        Self::new(true)
    }
}

impl Reporter for HumanReporter {
    fn report(
        &self,
        diagnostics: &[Diagnostic],
        summary: &Summary,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        let mut current_file: Option<&Path> = None;
        for d in diagnostics {
            let file = d.file.as_path();
            if current_file != Some(file) {
                if current_file.is_some() {
                    writeln!(out)?;
                }
                self.write_file_header(out, file)?;
                current_file = Some(file);
            }
            self.write_diagnostic(out, d)?;
        }

        if !self.quiet {
            if !diagnostics.is_empty() {
                writeln!(out)?;
            }
            Self::write_summary(out, summary)?;
        }
        Ok(())
    }
}

impl HumanReporter {
    fn write_file_header(&self, out: &mut dyn Write, file: &Path) -> io::Result<()> {
        if self.color {
            writeln!(out, "{}", file.display().bold())
        } else {
            writeln!(out, "{}", file.display())
        }
    }

    fn write_diagnostic(&self, out: &mut dyn Write, d: &Diagnostic) -> io::Result<()> {
        let location = d
            .line
            .map(|l| format!("line {l}: "))
            .unwrap_or_default();
        if self.color {
            let label = match d.severity {
                Severity::Error => severity_label(d.severity).red().bold().to_string(),
                Severity::Warning => severity_label(d.severity).yellow().bold().to_string(),
            };
            writeln!(
                out,
                "  {label}[{}] {location}{}",
                d.rule.dimmed(),
                d.message
            )
        } else {
            writeln!(
                out,
                "  {}[{}] {location}{}",
                severity_label(d.severity),
                d.rule,
                d.message
            )
        }
    }

    fn write_summary(out: &mut dyn Write, summary: &Summary) -> io::Result<()> {
        let files_word = pluralize(summary.files_scanned, "file", "files");
        if summary.errors == 0 && summary.warnings == 0 {
            writeln!(
                out,
                "mdtype: clean ({} {files_word} scanned)",
                summary.files_scanned
            )
        } else {
            writeln!(
                out,
                "mdtype: {} {} across {} {} ({} {files_word} scanned)",
                summary.errors,
                pluralize(summary.errors, "error", "errors"),
                summary.files_with_errors,
                pluralize(summary.files_with_errors, "file", "files"),
                summary.files_scanned,
            )
        }
    }
}

const fn severity_label(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

const fn pluralize(n: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if n == 1 {
        singular
    } else {
        plural
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mdtype_core::{Diagnostic, Reporter, Severity, Summary};

    use super::HumanReporter;

    fn fixture_diagnostics() -> Vec<Diagnostic> {
        vec![
            Diagnostic {
                file: PathBuf::from("content/posts/2026-02-missing-author.md"),
                line: None,
                rule: "frontmatter.schema",
                severity: Severity::Error,
                message: "missing required property 'author'".into(),
                fixit: None,
            },
            Diagnostic {
                file: PathBuf::from("content/posts/2026-03-stray-h1.md"),
                line: Some(7),
                rule: "body.forbid_h1",
                severity: Severity::Error,
                message: "top-level H1 is not allowed".into(),
                fixit: None,
            },
            Diagnostic {
                file: PathBuf::from("content/posts/2026-03-stray-h1.md"),
                line: Some(12),
                rule: "body.required_sections",
                severity: Severity::Error,
                message: "missing required section 'Summary'".into(),
                fixit: None,
            },
        ]
    }

    fn fixture_summary() -> Summary {
        Summary {
            files_scanned: 4,
            files_with_errors: 2,
            errors: 3,
            warnings: 0,
        }
    }

    fn render(reporter: &HumanReporter) -> String {
        let mut buf: Vec<u8> = Vec::new();
        reporter
            .report(&fixture_diagnostics(), &fixture_summary(), &mut buf)
            .expect("report");
        String::from_utf8(buf).expect("utf-8")
    }

    #[test]
    fn no_color_snapshot() {
        insta::assert_snapshot!(render(&HumanReporter::new(false)));
    }

    #[test]
    fn no_color_quiet_omits_summary() {
        let reporter = HumanReporter::new(false).with_quiet(true);
        insta::assert_snapshot!(render(&reporter));
    }

    #[test]
    fn clean_run_summary() {
        let reporter = HumanReporter::new(false);
        let mut buf: Vec<u8> = Vec::new();
        let summary = Summary {
            files_scanned: 7,
            files_with_errors: 0,
            errors: 0,
            warnings: 0,
        };
        reporter.report(&[], &summary, &mut buf).expect("report");
        insta::assert_snapshot!(String::from_utf8(buf).expect("utf-8"));
    }
}
