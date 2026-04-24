//! End-to-end golden test for the canonical fixture in `examples/blog-site/`.
//!
//! Runs the real `mdtype` binary so the entire pipeline (clap → config discovery →
//! schema load → parse → validate → reporter → exit) is exercised against pinned output.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn examples_blog_site_human_no_color() {
    let bin = env!("CARGO_BIN_EXE_mdtype");
    let workspace_root = workspace_root();

    let output = Command::new(bin)
        .arg("--no-color")
        .arg("examples/blog-site/")
        .current_dir(&workspace_root)
        .output()
        .expect("spawn mdtype");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let exit = output.status.code().unwrap_or(-1);

    assert_eq!(
        exit, 1,
        "expected exit 1 (frontmatter violation), got {exit}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stderr.is_empty(),
        "expected empty stderr, got:\n{stderr}"
    );

    insta::assert_snapshot!("blog_site_human_no_color", stdout);
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root above crates/mdtype")
        .to_path_buf()
}
