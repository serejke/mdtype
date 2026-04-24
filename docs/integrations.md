# Integration Recipes

`mdtype` is a single binary with stable exit codes and a versioned JSON output, so it slots into any pipeline that can run a command and check `$?`. Three common shapes:

## 1. Pre-commit hook

Run `mdtype` on the staged Markdown files only — fast on large repos, catches breakage before it lands.

`.git/hooks/pre-commit` (or any equivalent hook framework):

```sh
#!/usr/bin/env sh
set -e

staged=$(git diff --cached --name-only --diff-filter=ACMR -- '*.md' '*.markdown')
[ -z "$staged" ] && exit 0

# shellcheck disable=SC2086
mdtype --format human $staged
```

Exit `0` lets the commit through; exit `1` blocks it with diagnostics already on the developer's terminal; exit `2` blocks it because the schema or config is broken (fix that first).

## 2. CI job

Validate every Markdown file on every push and PR.

```yaml
# .github/workflows/mdtype.yml — adapt to whatever CI you use
name: mdtype
on: [push, pull_request]
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install --git https://github.com/serejke/mdtype mdtype
      - run: mdtype --format json . | tee mdtype.json
```

Because stdout is captured, `--format json` is the default — the explicit flag above is for clarity. The job fails on exit `1` (diagnostics) or `2` (config error). The captured `mdtype.json` can be uploaded as an artifact and consumed by downstream tooling — annotations, dashboards, anything that reads the documented contract.

## 3. LLM agent stop hook

The pattern: an agent edits a Markdown file, you want to gate "done" on `mdtype` being clean, and you want failures fed back as actionable structure rather than a wall of text.

Generic shape (tool-agnostic):

```sh
#!/usr/bin/env sh
# Run mdtype on the file(s) the agent just touched; emit JSON to stdout so the
# orchestrator can parse it and either declare success or feed diagnostics back
# to the model for another iteration.
mdtype --format json "$@"
status=$?

case $status in
  0) echo '{"ok": true}' >&2 ;;
  1) echo '{"ok": false, "reason": "diagnostics"}' >&2 ;;
  2) echo '{"ok": false, "reason": "config-error"}' >&2 ;;
esac

exit $status
```

The orchestrator reads stdout (the `mdtype` JSON), iterates over `diagnostics[]`, and constructs a follow-up prompt: file path, line number, rule id, message, and the optional `fixit` hint. Because every diagnostic carries a stable `rule` id and 1-indexed `line`, the prompt can be deterministic and tight — no scraping of human messages, no model-specific glue. Loop until exit `0` or a retry budget runs out.

## Why this works

- Exit codes are stable: `0`, `1`, `2`. Nothing else.
- The JSON contract is versioned and snapshot-tested — see [`docs/json-schema.md`](./json-schema.md).
- `mdtype` never rewrites your files; the hook is purely a gate.
- The CLI is a single self-contained binary, no daemon, no watch mode, no sidecar process to manage.
