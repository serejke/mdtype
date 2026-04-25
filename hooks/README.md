# Hooks

Drop-in scripts so `mdtype` blocks bad Markdown before it ships.

| Script             | Purpose                                                                                                                                       |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `pre-commit`       | Git pre-commit hook — validates every staged `.md` before the commit lands.                                                                   |
| `claude-code-stop` | Claude Code Stop hook — blocks the agent from declaring "done" while diagnostics exist, feeding the JSON report back so it can fix and retry. |
| `install.sh`       | One-shot installer that symlinks `pre-commit` into a target git repo.                                                                         |

## Install — git pre-commit

From a clone of this repo:

```
./hooks/install.sh /path/to/your/project
```

Manually:

```
ln -s "$(pwd)/hooks/pre-commit" /path/to/your/project/.git/hooks/pre-commit
chmod +x /path/to/your/project/.git/hooks/pre-commit
```

Or via the [pre-commit framework](https://pre-commit.com) — add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/serejke/mdtype
    rev: v0.1.0
    hooks:
      - id: mdtype
```

(`.pre-commit-hooks.yaml` at the repo root tells the framework what to run.)

### Configuration

Set in your shell, `.envrc`, or wrapper script:

| Variable        | Default  | Purpose                                                                  |
| --------------- | -------- | ------------------------------------------------------------------------ |
| `MDTYPE_BIN`    | `mdtype` | Path to the binary if it isn't on `PATH`.                                |
| `MDTYPE_FORMAT` | `human`  | `human` or `json`.                                                       |
| `MDTYPE_ARGS`   | (empty)  | Extra args appended to the invocation, e.g. `--config etc/.mdtype.yaml`. |

## Install — Claude Code Stop hook

In the project you want guarded:

```
mkdir -p .claude
cp /path/to/mdtype/hooks/claude-code-stop .claude/mdtype-stop.sh
chmod +x .claude/mdtype-stop.sh
```

Then wire it into `.claude/settings.json`:

```json
{
  "hooks": {
    "Stop": [{ "matcher": "*", "command": "./.claude/mdtype-stop.sh" }]
  }
}
```

What it does:

1. Claude Code tries to stop the session.
2. The hook runs `mdtype --format json .` over the workspace.
3. Exit 0 (clean) → silent, Claude stops.
4. Errors found → emits `{"decision":"block","reason":"..."}` with the full JSON report. Claude refuses to stop and re-prompts itself with the diagnostics so it can fix them and try again.

This pairs perfectly with `mdtype`'s LLM-friendly diagnostic messages (see [`docs/error-messages.md`](../docs/error-messages.md)) — every diagnostic carries a stable `rule` id, `file`, `line`, and an optional `fixit` hint, so the agent doesn't need to scrape prose.

### Configuration

| Variable      | Default  | Purpose                                       |
| ------------- | -------- | --------------------------------------------- |
| `MDTYPE_BIN`  | `mdtype` | Path to the binary if it isn't on `PATH`.     |
| `MDTYPE_PATH` | `.`      | Path to validate.                             |
| `MDTYPE_ARGS` | (empty)  | Extra args, e.g. `--config etc/.mdtype.yaml`. |

`jq` is auto-detected and used when present; falls back to a hand-rolled JSON encoder otherwise.
