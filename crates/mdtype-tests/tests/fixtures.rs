//! Fixture-driven integration tests.
//!
//! Walks `crates/mdtype-tests/fixtures/<scenario>/`, runs the real `mdtype` binary against
//! each, and asserts:
//!
//! * exit code matches `expected/exit_code`
//! * stdout in `--format human --no-color` matches `expected/stdout.human`
//! * stdout in `--format json` (re-pretty-printed) matches `expected/stdout.json`
//! * stderr matches `expected/stderr` if present, else stderr is empty
//!
//! Per-scenario knobs:
//!
//! * `args.txt` — one CLI arg per line, used instead of the default `.`. Lets a scenario
//!   pass `--config`, `--schema`, multiple paths, etc.
//!
//! All output is path-normalised before comparison: occurrences of the absolute scenario
//! path get rewritten to `<scenario>` so goldens stay portable across checkouts.
//!
//! Adding a new scenario requires only a new folder under `fixtures/` — no Rust changes.
//! Regenerate goldens with `UPDATE_FIXTURES=1 cargo test -p mdtype-tests --test fixtures`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mdtype_tests::mdtype_bin;

const UPDATE_ENV: &str = "UPDATE_FIXTURES";

#[test]
fn all_fixtures_pass() {
    let fixtures_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let scenarios = collect_scenarios(&fixtures_root);
    if scenarios.is_empty() {
        eprintln!(
            "no fixtures under {} — harness ran with zero scenarios.",
            fixtures_root.display()
        );
        return;
    }

    let mut failures: Vec<String> = Vec::new();
    for scenario in &scenarios {
        for format in [Format::Human, Format::Json] {
            if let Err(msg) = run_scenario(scenario, format) {
                let name = scenario
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                failures.push(format!("[{name} / {format:?}] {msg}"));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} scenario assertion(s) failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[derive(Debug, Clone, Copy)]
enum Format {
    Human,
    Json,
}

impl Format {
    const fn cli_args(self) -> &'static [&'static str] {
        match self {
            Self::Human => &["--format", "human", "--no-color"],
            Self::Json => &["--format", "json"],
        }
    }

    const fn expected_filename(self) -> &'static str {
        match self {
            Self::Human => "stdout.human",
            Self::Json => "stdout.json",
        }
    }
}

fn collect_scenarios(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.starts_with('_') || name.starts_with('.') {
            continue;
        }
        out.push(path);
    }
    out.sort();
    out
}

fn run_scenario(scenario: &Path, format: Format) -> Result<(), String> {
    let canonical_scenario = fs::canonicalize(scenario)
        .map_err(|e| format!("canonicalize {}: {e}", scenario.display()))?;

    let mut cmd = Command::new(mdtype_bin());
    cmd.args(format.cli_args()).current_dir(scenario);
    for arg in scenario_args(scenario)? {
        cmd.arg(arg);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("failed to spawn mdtype: {e}"))?;

    let actual_exit = output
        .status
        .code()
        .ok_or_else(|| String::from("process terminated by signal"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let normalised_stderr = normalise_paths(&stderr, &canonical_scenario);

    let expected_dir = scenario.join("expected");
    let expected_exit = read_expected_exit(&expected_dir).map_err(|e| {
        format!("{e}\nactual exit: {actual_exit}\nstdout:\n{stdout}\nstderr:\n{stderr}")
    })?;
    if actual_exit != expected_exit {
        return Err(format!(
            "exit code mismatch: expected {expected_exit}, got {actual_exit}\n\
             stdout:\n{stdout}\nstderr:\n{stderr}"
        ));
    }

    // On exit 2 (config/schema errors) the CLI writes nothing to stdout and the message
    // lands on stderr — keep empty stdout as-is rather than trying to JSON-parse it.
    let normalised_stdout = if output.stdout.is_empty() {
        String::new()
    } else {
        match format {
            Format::Human => normalise_paths(&stdout, &canonical_scenario),
            Format::Json => normalise_paths(&normalise_json(&output.stdout)?, &canonical_scenario),
        }
    };

    let updating = std::env::var(UPDATE_ENV).as_deref() == Ok("1");
    if updating {
        fs::create_dir_all(&expected_dir)
            .map_err(|e| format!("create {}: {e}", expected_dir.display()))?;
    }

    write_or_compare(
        &expected_dir.join(format.expected_filename()),
        &normalised_stdout,
        updating,
    )?;

    let expected_stderr_path = expected_dir.join("stderr");
    if expected_stderr_path.exists() || (updating && !normalised_stderr.is_empty()) {
        write_or_compare(&expected_stderr_path, &normalised_stderr, updating)?;
    } else if !normalised_stderr.is_empty() {
        return Err(format!(
            "expected empty stderr, got:\n{normalised_stderr}\n\
             (write expected/stderr to assert against it instead)"
        ));
    }

    Ok(())
}

fn write_or_compare(path: &Path, actual: &str, updating: bool) -> Result<(), String> {
    if updating {
        return fs::write(path, actual).map_err(|e| format!("write {}: {e}", path.display()));
    }
    let expected = fs::read_to_string(path).map_err(|e| {
        format!(
            "read {}: {e}\n\
             actual was:\n{actual}\n\
             (re-run with {UPDATE_ENV}=1 to write expected files)",
            path.display()
        )
    })?;
    if actual != expected {
        return Err(format!(
            "{} mismatch\n--- expected ---\n{expected}--- actual ---\n{actual}",
            path.display()
        ));
    }
    Ok(())
}

fn scenario_args(scenario: &Path) -> Result<Vec<String>, String> {
    let path = scenario.join("args.txt");
    let raw = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![".".into()]),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
    let args: Vec<String> = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();
    if args.is_empty() {
        return Err(format!(
            "{} is empty; either delete it or provide at least one arg",
            path.display()
        ));
    }
    Ok(args)
}

fn normalise_paths(text: &str, canonical_scenario: &Path) -> String {
    let canonical = canonical_scenario.to_string_lossy().into_owned();
    text.replace(&canonical, "<scenario>")
}

fn read_expected_exit(expected_dir: &Path) -> Result<i32, String> {
    let path = expected_dir.join("exit_code");
    let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    raw.trim()
        .parse::<i32>()
        .map_err(|e| format!("parse exit_code in {}: {e}", path.display()))
}

fn normalise_json(raw: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(raw)
        .map_err(|e| format!("CLI emitted non-utf8 stdout: {e}"))?
        .trim();
    let parsed: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| format!("CLI emitted invalid JSON: {e}\nraw:\n{text}"))?;
    let mut out =
        serde_json::to_string_pretty(&parsed).map_err(|e| format!("re-serialise JSON: {e}"))?;
    out.push('\n');
    Ok(out)
}
