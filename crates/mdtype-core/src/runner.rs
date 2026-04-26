//! Two-phase workspace runner.
//!
//! Builds a [`Workspace`] from every parsed file, runs every per-file body rule (current
//! [`Validator`] semantics, hoisted), then runs every workspace rule against its scope.
//! See `docs/proposals/0001-workspace-pipeline.md`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use comrak::Arena;

use crate::diagnostic::{Diagnostic, Severity};
use crate::error::Error;
use crate::extract;
use crate::parser::{parse_file_with_options, ParsedDocument};
use crate::schema::Schema;
use crate::validator::{CoreValidator, Validator};
use crate::workspace::{Requirements, Workspace};
use crate::WorkspaceRule;

/// Stable rule id for body-parse failures discovered inside the runner.
pub const RUNNER_PARSE_RULE_ID: &str = "mdtype.parse";

/// One workspace-rule invocation paired with the slice of files it judges.
struct RosterEntry<'a> {
    rule: &'a dyn WorkspaceRule,
    scope: Vec<PathBuf>,
}

/// Run every body and workspace rule across `files` against the schemas they resolve to.
///
/// `files` lists every Markdown file the run should consider. `schemas` is the
/// CLI-managed pool of [`Schema`]s. `schema_idx[i]` selects the schema for `files[i]`,
/// or `None` if the file matches no schema (it is still parsed and indexed so other
/// files may reference it as a link target).
///
/// Returns a sorted diagnostic list. Files whose body parse fails surface as
/// [`RUNNER_PARSE_RULE_ID`] diagnostics; pre-pass parse failures must be handled by the
/// caller and excluded from `files`.
///
/// # Errors
///
/// Currently this function only returns errors propagated from extractors that operate
/// on inputs the runner is responsible for; today none of those produce errors and
/// `Ok(_)` is always returned. The signature reserves the variant for future fact-kind
/// extensions.
///
/// # Panics
///
/// Panics if `files.len() != schema_idx.len()`. Callers are expected to keep these
/// vectors strictly parallel.
pub fn run_workspace(
    files: &[PathBuf],
    schemas: &[Schema],
    schema_idx: &[Option<usize>],
) -> Result<Vec<Diagnostic>, Error> {
    assert_eq!(
        files.len(),
        schema_idx.len(),
        "run_workspace: files and schema_idx must be parallel"
    );

    let roster = build_roster(files, schemas, schema_idx);
    let reqs = roster.iter().fold(Requirements::default(), |acc, e| {
        acc.merge(e.rule.requires())
    });
    let options = make_options(reqs);

    let arena = Arena::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut parsed: Vec<Option<ParsedDocument<'_>>> = Vec::with_capacity(files.len());
    for file in files {
        match parse_file_with_options(file, &arena, &options) {
            Ok(doc) => parsed.push(Some(doc)),
            Err(e) => {
                diagnostics.push(parse_failure_diagnostic(file, &format_parse_error(&e)));
                parsed.push(None);
            }
        }
    }

    let ws = build_workspace(files, &parsed, reqs);

    for (i, doc_opt) in parsed.iter().enumerate() {
        let Some(doc) = doc_opt.as_ref() else {
            continue;
        };
        let Some(s_idx) = schema_idx[i] else {
            continue;
        };
        let schema = &schemas[s_idx];
        let mut body_diags = CoreValidator.validate(doc, schema);
        diagnostics.append(&mut body_diags);
    }

    for entry in &roster {
        entry.rule.check(&ws, &entry.scope, &mut diagnostics);
    }

    diagnostics.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule.cmp(b.rule))
    });

    Ok(diagnostics)
}

fn build_roster<'a>(
    files: &[PathBuf],
    schemas: &'a [Schema],
    schema_idx: &[Option<usize>],
) -> Vec<RosterEntry<'a>> {
    let mut roster: Vec<RosterEntry<'a>> = Vec::new();
    for (s_idx, schema) in schemas.iter().enumerate() {
        if schema.workspace.is_empty() {
            continue;
        }
        let scope: Vec<PathBuf> = files
            .iter()
            .zip(schema_idx.iter())
            .filter_map(|(file, idx)| {
                if *idx == Some(s_idx) {
                    Some(file.clone())
                } else {
                    None
                }
            })
            .collect();
        if scope.is_empty() {
            continue;
        }
        for rule in &schema.workspace {
            roster.push(RosterEntry {
                rule: rule.as_ref(),
                scope: scope.clone(),
            });
        }
    }
    roster
}

fn make_options(reqs: Requirements) -> comrak::Options<'static> {
    let mut options = comrak::Options::default();
    if reqs.links_wiki {
        options.extension.wikilinks_title_after_pipe = true;
        // GFM tables escape literal `|` as `\|`; without the table extension comrak never
        // tokenises tables, so a wikilink inside a cell (`[[T\|A]]`) reaches the wikilink
        // parser with the backslash-pipe intact and the `|alias` split is lost. Enabling
        // tables runs the cell-level un-escape before the wikilink extension sees content.
        options.extension.table = true;
    }
    options
}

fn build_workspace(
    files: &[PathBuf],
    parsed: &[Option<ParsedDocument<'_>>],
    reqs: Requirements,
) -> Workspace {
    let mut ws = Workspace::default();
    for file in files {
        ws.files.push(file.clone());
        if let Some(stem) = file.file_stem().and_then(OsStr::to_str) {
            ws.by_basename
                .entry(stem.to_lowercase())
                .or_default()
                .push(file.clone());
        }
    }
    for doc in parsed.iter().flatten() {
        let path = doc.path.clone();
        ws.frontmatter.insert(path.clone(), doc.frontmatter.clone());
        if reqs.headings {
            let mut h = Vec::new();
            extract::extract_headings(doc, &mut h);
            if !h.is_empty() {
                ws.headings.insert(path.clone(), h);
            }
        }
        if reqs.links_inline || reqs.links_wiki {
            let mut l = Vec::new();
            extract::extract_links(doc, reqs, &mut l);
            if !l.is_empty() {
                ws.links.insert(path, l);
            }
        }
    }
    ws
}

fn parse_failure_diagnostic(file: &Path, message: &str) -> Diagnostic {
    Diagnostic {
        file: file.to_path_buf(),
        line: None,
        rule: RUNNER_PARSE_RULE_ID,
        severity: Severity::Error,
        message: message.to_string(),
        fixit: None,
    }
}

fn format_parse_error(error: &Error) -> String {
    match error {
        Error::Frontmatter { message, .. } => format!("frontmatter parse failed: {message}"),
        Error::Io { source, .. } => format!("read failed: {source}"),
        Error::Schema(msg) => format!("schema error: {msg}"),
        Error::Other(msg) => msg.clone(),
    }
}
