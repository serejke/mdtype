# Integration Recipes

`mdtype` is a single binary with stable exit codes and a versioned JSON output, so it slots into any pipeline that can run a command and check `$?`. Three common shapes:

> If you want all three at once in a fresh project, install the [`setup-mdtype`](../skills/setup-mdtype/SKILL.md) agent skill (`npx skills add serejke/mdtype --skill setup-mdtype`) — it writes the config, registers mdtype with `CLAUDE.md`/`AGENTS.md`, and offers to install the hooks below.

## 1. Pre-commit hook

The repo ships a ready-to-install hook at [`hooks/pre-commit`](../hooks/pre-commit) plus a one-shot installer.

```
git clone https://github.com/serejke/mdtype
./mdtype/hooks/install.sh /path/to/your/project
```

That symlinks the hook into `.git/hooks/pre-commit`. Subsequent `git commit` calls run `mdtype` against staged `.md` files only — fast on large repos. Exit `0` lets the commit through; exit `1` blocks with diagnostics on the developer's terminal; exit `2` blocks because the schema or config is broken.

If you use the [pre-commit framework](https://pre-commit.com), the repo also exposes a manifest (`.pre-commit-hooks.yaml`):

```yaml
repos:
  - repo: https://github.com/serejke/mdtype
    rev: v0.1.0
    hooks:
      - id: mdtype
```

See [`hooks/README.md`](../hooks/README.md) for env-var configuration (`MDTYPE_BIN`, `MDTYPE_FORMAT`, `MDTYPE_ARGS`).

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

## 3. Claude Code Stop hook

When an agent edits a Markdown file, you want "done" gated on `mdtype` being clean, and you want failures fed back as actionable structure — not a wall of text.

The repo ships [`hooks/claude-code-stop`](../hooks/claude-code-stop). Drop it into your project and wire it into `.claude/settings.json`:

```json
{
  "hooks": {
    "Stop": [{ "matcher": "*", "command": "./.claude/mdtype-stop.sh" }]
  }
}
```

When Claude tries to stop the session, the hook runs `mdtype --format json .`. On exit `0` it emits nothing and the session ends. On any diagnostic it emits `{"decision":"block","reason":"..."}` containing the full JSON contract — Claude refuses to stop and re-prompts itself with the diagnostics. Because every diagnostic carries a stable `rule` id, 1-indexed `line`, and an optional `fixit` hint, the agent can act without scraping prose. Loop until exit `0`.

For other agent harnesses, the same idea works tool-agnostically — read [`docs/json-schema.md`](./json-schema.md), iterate over `diagnostics[]`, and feed the next prompt.

## Why this works

- Exit codes are stable: `0`, `1`, `2`. Nothing else.
- The JSON contract is versioned and snapshot-tested — see [`docs/json-schema.md`](./json-schema.md).
- `mdtype` never rewrites your files; the hook is purely a gate.
- The CLI is a single self-contained binary, no daemon, no watch mode, no sidecar process to manage.
