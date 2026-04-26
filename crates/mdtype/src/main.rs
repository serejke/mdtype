//! `mdtype` CLI. Thin wrapper over the library crates.
//!
//! Every flag maps 1:1 to a library call. The CLI does not contain validation logic.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{anyhow, Context};
use clap::Parser;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use mdtype_core::{
    read_frontmatter, run_workspace, BodyRuleFactory, Diagnostic, Reporter, Schema, SchemaSource,
    Severity, Summary, WorkspaceRuleFactory,
};
use mdtype_reporter_human::HumanReporter;
use mdtype_reporter_json::JsonReporter;
use mdtype_rules_obsidian::register_obsidian;
use mdtype_rules_stdlib::{install_type_checks, register_stdlib, register_stdlib_workspace};
use mdtype_schema_yaml::{config_walk_up, load_schema_file, YamlSchemaSource};

const PARSE_RULE_ID: &str = "mdtype.parse";

/// A type checker for Markdown.
#[derive(Debug, Parser)]
#[command(name = "mdtype", version, about, long_about = None)]
struct Cli {
    /// Files or directories to validate. Defaults to the current directory.
    paths: Vec<PathBuf>,

    /// Path to `.mdtype.yaml`. Defaults to walking up from the current directory.
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Validate every path against this schema, ignoring the glob map.
    #[arg(long, value_name = "FILE")]
    schema: Option<PathBuf>,

    /// Output format. `auto` picks `human` for a tty, `json` otherwise.
    #[arg(short, long, value_enum, default_value_t = Format::Auto)]
    format: Format,

    /// Disable colored output in the human reporter.
    #[arg(long)]
    no_color: bool,

    /// Suppress the summary line.
    #[arg(long)]
    quiet: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum Format {
    /// `human` if stdout is a tty, `json` otherwise.
    Auto,
    /// Colored, grouped-by-file text output.
    Human,
    /// Structured JSON matching the documented contract.
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(code) => code,
        Err(e) => {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "mdtype: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: &Cli) -> anyhow::Result<ExitCode> {
    let body_factories = Arc::new(register_stdlib());
    let mut workspace_factory_list = register_stdlib_workspace();
    workspace_factory_list.extend(register_obsidian());
    let workspace_factories = Arc::new(workspace_factory_list);
    let cwd = std::env::current_dir().context("reading current directory")?;

    let mut schemas: Vec<Schema> = Vec::new();
    let mut override_cache: HashMap<PathBuf, usize> = HashMap::new();
    let mode = build_mode(
        cli,
        &cwd,
        &body_factories,
        &workspace_factories,
        &mut schemas,
    )?;

    let walk_roots = if cli.paths.is_empty() {
        vec![cwd.clone()]
    } else {
        cli.paths.clone()
    };
    let mut files: Vec<PathBuf> = Vec::new();
    for root in &walk_roots {
        collect_md_files(root, &mut files)
            .with_context(|| format!("walking {}", root.display()))?;
    }
    files.sort();
    files.dedup();

    // Frontmatter pre-pass: read each file's YAML block (no body parse), use it to pick
    // the file's schema. Pre-pass failures are CLI-side parse diagnostics; the file is
    // excluded from the runner's input so the runner never re-attempts a parse the CLI
    // already failed.
    let mut prepass_diagnostics: Vec<Diagnostic> = Vec::new();
    let mut runner_files: Vec<PathBuf> = Vec::with_capacity(files.len());
    let mut runner_schema_idx: Vec<Option<usize>> = Vec::with_capacity(files.len());
    for file in &files {
        let frontmatter = match read_frontmatter(file) {
            Ok(v) => v,
            Err(e) => {
                prepass_diagnostics.push(parse_failure_diagnostic(file, &format_parse_error(&e)));
                continue;
            }
        };
        let schema_idx = match resolve_schema_index(
            &mode,
            file,
            &frontmatter,
            &body_factories,
            &workspace_factories,
            &mut schemas,
            &mut override_cache,
        ) {
            Ok(idx) => idx,
            Err(e) => {
                prepass_diagnostics.push(parse_failure_diagnostic(file, &e.to_string()));
                continue;
            }
        };
        runner_files.push(file.clone());
        runner_schema_idx.push(schema_idx);
    }

    // Synthesize schema-derived type checks (currently `types.entity_ref`) into each
    // schema's workspace pipeline. Must run after all schemas — including per-file
    // `schema:` overrides loaded during the pre-pass — are in the pool, and before any
    // rule executes.
    install_type_checks(&mut schemas);

    let runner_diagnostics = run_workspace(&runner_files, &schemas, &runner_schema_idx)
        .context("running workspace pipeline")?;

    let mut diagnostics: Vec<Diagnostic> = prepass_diagnostics;
    diagnostics.extend(runner_diagnostics);
    diagnostics.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule.cmp(b.rule))
    });

    let mut files_with_errors: HashSet<PathBuf> = HashSet::new();
    for d in &diagnostics {
        files_with_errors.insert(d.file.clone());
    }

    let summary = Summary {
        files_scanned: files.len(),
        files_with_errors: files_with_errors.len(),
        errors: diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count(),
        warnings: diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count(),
    };

    write_report(cli, &diagnostics, &summary)?;

    Ok(if summary.errors > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn parse_failure_diagnostic(file: &Path, message: &str) -> Diagnostic {
    Diagnostic {
        file: file.to_path_buf(),
        line: None,
        rule: PARSE_RULE_ID,
        severity: Severity::Error,
        message: message.to_string(),
        fixit: None,
    }
}

/// Render a [`mdtype_core::Error`] as a diagnostic message — strips the redundant `path:`
/// prefix the Display impls carry (the diagnostic's `file` field already names the file).
fn format_parse_error(error: &mdtype_core::Error) -> String {
    use mdtype_core::Error;
    match error {
        Error::Frontmatter { message, .. } => format!("frontmatter parse failed: {message}"),
        Error::Io { source, .. } => format!("read failed: {source}"),
        Error::Schema(msg) => format!("schema error: {msg}"),
        Error::Other(msg) => msg.clone(),
    }
}

/// How the CLI selects a schema for each file.
enum Mode {
    /// `--schema FILE` was given; every file uses `schemas[0]`.
    Single,
    /// Glob-map mode driven by `.mdtype.yaml`.
    GlobMap {
        /// Directory schema globs and overrides resolve against.
        root: PathBuf,
        /// Compiled glob set; index `i` corresponds to `entries_offset + i` in `schemas`.
        glob_set: GlobSet,
        /// Where the glob-map entries start inside `schemas`.
        entries_offset: usize,
    },
}

fn build_mode(
    cli: &Cli,
    cwd: &Path,
    body_factories: &Arc<Vec<Box<dyn BodyRuleFactory>>>,
    workspace_factories: &Arc<Vec<Box<dyn WorkspaceRuleFactory>>>,
    schemas: &mut Vec<Schema>,
) -> anyhow::Result<Mode> {
    if let Some(p) = &cli.schema {
        let schema = load_schema_file(p, body_factories, workspace_factories)
            .with_context(|| format!("loading --schema {}", p.display()))?;
        schemas.push(schema);
        return Ok(Mode::Single);
    }

    let config_path = if let Some(p) = &cli.config {
        p.clone()
    } else {
        let found = discover_config(cwd, &cli.paths)?;
        warn_on_nested_configs(&found);
        found
    };

    let source = YamlSchemaSource::new(
        config_path.clone(),
        Arc::clone(body_factories),
        Arc::clone(workspace_factories),
    );
    let root = source.root.clone();
    let entries = source
        .load()
        .with_context(|| format!("loading {}", config_path.display()))?;

    let entries_offset = schemas.len();
    let mut builder = GlobSetBuilder::new();
    for entry in entries {
        // `literal_separator(true)` makes globs behave like Unix shell:
        // `*` does not cross `/`; only `**` traverses directories. So
        // `*.md` matches top-level files only, not `sub/dir/file.md`.
        let glob = GlobBuilder::new(&entry.glob)
            .literal_separator(true)
            .build()
            .with_context(|| format!("invalid glob '{}'", entry.glob))?;
        builder.add(glob);
        schemas.push(entry.schema);
    }
    let glob_set = builder.build().context("building glob set")?;

    Ok(Mode::GlobMap {
        root,
        glob_set,
        entries_offset,
    })
}

fn resolve_schema_index(
    mode: &Mode,
    file: &Path,
    frontmatter: &serde_json::Value,
    body_factories: &Arc<Vec<Box<dyn BodyRuleFactory>>>,
    workspace_factories: &Arc<Vec<Box<dyn WorkspaceRuleFactory>>>,
    schemas: &mut Vec<Schema>,
    override_cache: &mut HashMap<PathBuf, usize>,
) -> anyhow::Result<Option<usize>> {
    if matches!(mode, Mode::Single) {
        return Ok(Some(0));
    }
    let Mode::GlobMap {
        root,
        glob_set,
        entries_offset,
    } = mode
    else {
        unreachable!()
    };

    if let Some(rel) = frontmatter.get("schema").and_then(|v| v.as_str()) {
        let resolved = if Path::new(rel).is_absolute() {
            PathBuf::from(rel)
        } else {
            root.join(rel)
        };
        let key = fs::canonicalize(&resolved).unwrap_or_else(|_| resolved.clone());
        if let Some(&idx) = override_cache.get(&key) {
            return Ok(Some(idx));
        }
        let schema = load_schema_file(&resolved, body_factories, workspace_factories)
            .with_context(|| format!("loading override schema {}", resolved.display()))?;
        let idx = schemas.len();
        schemas.push(schema);
        override_cache.insert(key, idx);
        return Ok(Some(idx));
    }

    let match_path = match_path_for(root, file);
    Ok(glob_set
        .matches(&match_path)
        .first()
        .copied()
        .map(|i| entries_offset + i))
}

/// Warn on stderr when the loaded `.mdtype.yaml` has descendant `.mdtype.yaml` files inside
/// its tree. mdtype only honours one config — the closest-to-cwd wins, no merging — so
/// nested configs are silently shadowed and almost always indicate a misunderstanding.
/// Suggest the schema-per-folder pattern via globs.
fn warn_on_nested_configs(loaded: &Path) {
    let Some(root) = loaded.parent() else {
        return;
    };
    let mut nested: Vec<PathBuf> = Vec::new();
    collect_nested(root, loaded, &mut nested);
    if nested.is_empty() {
        return;
    }
    nested.sort();
    let mut stderr = io::stderr().lock();
    let _ = writeln!(
        stderr,
        "mdtype: warning: nested .mdtype.yaml files inside {} are SHADOWED by the loaded config and have no effect:",
        root.display()
    );
    for p in &nested {
        let _ = writeln!(stderr, "  {}", p.display());
    }
    let _ = writeln!(
        stderr,
        "mdtype: only the closest .mdtype.yaml to cwd is loaded — there is no merging across nested configs.\n\
         mdtype: to apply different rules to different folders, use globs in {} (see examples/multi-folder/).",
        loaded.display()
    );
}

fn collect_nested(dir: &Path, loaded: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            // Skip dot-dirs (.git, .obsidian, etc.) — never user content.
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'))
            {
                continue;
            }
            collect_nested(&path, loaded, out);
        } else if ft.is_file()
            && path.file_name().and_then(|n| n.to_str()) == Some(".mdtype.yaml")
            && path != loaded
        {
            out.push(path);
        }
    }
}

/// Locate `.mdtype.yaml` by walking up from the cwd; if absent, fall back to walking up from
/// each user-supplied path's parent. Returns the first match.
fn discover_config(cwd: &Path, paths: &[PathBuf]) -> anyhow::Result<PathBuf> {
    if let Some(found) = config_walk_up(cwd) {
        return Ok(found);
    }
    for p in paths {
        let start = if p.is_file() {
            p.parent().unwrap_or(p)
        } else {
            p.as_path()
        };
        if let Some(found) = config_walk_up(start) {
            return Ok(found);
        }
    }
    Err(anyhow!(
        "no .mdtype.yaml found by walking up from {} or any input path",
        cwd.display()
    ))
}

/// Compute the path to feed into the glob set for a file:
/// `file` made relative to the config `root` when possible, otherwise the file path itself.
fn match_path_for(root: &Path, file: &Path) -> PathBuf {
    let canon_root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let canon_file = fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    canon_file
        .strip_prefix(&canon_root)
        .map(Path::to_path_buf)
        .unwrap_or(canon_file)
}

fn collect_md_files(root: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let meta = fs::metadata(root)?;
    if meta.is_file() {
        if is_markdown(root) {
            out.push(root.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            collect_md_files(&path, out)?;
        } else if ft.is_file() && is_markdown(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_markdown(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };
    let lower = ext.to_ascii_lowercase();
    lower == "md" || lower == "markdown"
}

fn write_report(cli: &Cli, diagnostics: &[Diagnostic], summary: &Summary) -> anyhow::Result<()> {
    let stdout_is_tty = io::stdout().is_terminal();
    let mut stdout = io::stdout().lock();

    let format = match cli.format {
        Format::Auto => {
            if stdout_is_tty {
                Format::Human
            } else {
                Format::Json
            }
        }
        f => f,
    };

    match format {
        Format::Human => {
            let color = !cli.no_color && stdout_is_tty;
            let reporter = HumanReporter::new(color).with_quiet(cli.quiet);
            reporter
                .report(diagnostics, summary, &mut stdout)
                .context("writing human report")?;
        }
        Format::Json => {
            let reporter = JsonReporter::new(stdout_is_tty);
            reporter
                .report(diagnostics, summary, &mut stdout)
                .context("writing json report")?;
        }
        Format::Auto => unreachable!("resolved above"),
    }
    Ok(())
}
