//! End-to-end golden tests for the canonical fixture in `examples/blog-site/`.
//!
//! Runs the real `mdtype` binary so the entire pipeline (clap → config discovery →
//! schema load → parse → validate → reporter → exit) is exercised against pinned output.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

fn workspace_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root above crates/mdtype-tests")
            .to_path_buf()
    })
}

/// Build (if needed) and return the path to the `mdtype` binary.
fn mdtype_bin() -> &'static Path {
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

fn run_blog_site(extra: &[&str]) -> Output {
    Command::new(mdtype_bin())
        .current_dir(workspace_root())
        .args(extra)
        .arg("examples/blog-site/")
        .output()
        .expect("spawn mdtype")
}

fn assert_clean_stderr(output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "expected empty stderr, got:\n{stderr}");
}

#[test]
fn examples_blog_site_human_no_color() {
    let output = run_blog_site(&["--format", "human", "--no-color"]);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let exit = output.status.code().unwrap_or(-1);

    assert_eq!(
        exit, 1,
        "expected exit 1 (frontmatter violation), got {exit}\nstdout:\n{stdout}"
    );
    assert_clean_stderr(&output);
    insta::assert_snapshot!("blog_site_human_no_color", stdout);
}

#[test]
fn examples_blog_site_json() {
    let output = run_blog_site(&["--format", "json"]);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let exit = output.status.code().unwrap_or(-1);

    assert_eq!(exit, 1, "expected exit 1, got {exit}\nstdout:\n{stdout}");
    assert_clean_stderr(&output);

    // Round-trip through serde_json so the snapshot is deterministic regardless of
    // pretty-vs-compact output choice and immune to incidental whitespace differences.
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("CLI emitted invalid JSON");
    let pretty = serde_json::to_string_pretty(&parsed).expect("re-serialise");
    insta::assert_snapshot!("blog_site_json", pretty);
}

#[test]
fn examples_blog_site_auto_in_pipe_is_json() {
    // Cargo captures stdout, so format=auto must resolve to json (per SPEC).
    let output = run_blog_site(&[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim_start().starts_with('{'),
        "auto format in non-tty should produce JSON, got:\n{stdout}"
    );
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("CLI emitted invalid JSON");
    assert_eq!(parsed["version"], serde_json::json!("1"));
}
