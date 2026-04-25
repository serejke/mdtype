---
name: setup-mdtype
description: Initialize mdtype in the current project. Creates a root `.mdtype.yaml` + a starter schema, appends a "Validate Markdown with mdtype" section to the agent's instructions file (CLAUDE.md or AGENTS.md), and offers to install the pre-commit + Claude Code Stop hooks. Use when the user says "set up mdtype", "init mdtype", or runs `/setup-mdtype`.
---

# Setup mdtype

You are bootstrapping [`mdtype`](https://github.com/serejke/mdtype) — a type checker for Markdown — in the project the user is currently in. The end state: every `.md` file in this project is validated against a YAML schema, the agent (you, in future sessions) knows mdtype exists and treats it as a blocking gate.

## Preconditions

Before doing anything, check:

1. **`mdtype` is installed.** Run `mdtype --version`. If the binary is missing, tell the user how to install — `cargo install mdtype`, prebuilt binary from <https://github.com/serejke/mdtype/releases/latest>, or `cargo binstall mdtype`. Do not proceed with steps below until they confirm install.
2. **Not already initialized.** If `./.mdtype.yaml` already exists at the project root, ask the user whether to overwrite or extend. Don't silently clobber.
3. **You are at the project root.** Check `pwd` and the presence of common project markers (`.git/`, `Cargo.toml`, `package.json`, `pyproject.toml`, an Obsidian-style folder structure, etc.). If unsure, ask.

## What to create

### `.mdtype.yaml` at the project root

A minimal "one root config + one catch-all glob" starting point. The user can split into per-folder schemas later (point them at `examples/multi-folder/` in the mdtype repo).

```yaml
# Root mdtype config. Closest .mdtype.yaml to cwd wins; nested configs do NOT merge —
# use globs here, not subdirectory configs.
rules:
  - glob: "**/*.md"
    schema: .mdtype/schemas/note.yaml
```

### `.mdtype/schemas/note.yaml`

A starter schema. Conservative defaults — frontmatter with `created` / `updated` / `description` and a forbidden-sections rule that catches leftover scratch.

```yaml
name: note
description: |
  Default schema for any markdown note in this project. Adjust to taste.
frontmatter:
  type: object
  required: [created, updated, description]
  additionalProperties: false
  properties:
    created:
      type: string
      format: date
    updated:
      type: string
      format: date
    description:
      type: string
      minLength: 20

body:
  - rule: forbidden-sections
    sections: [TODO, Scratch, WIP, Draft, FIXME]
```

If the project clearly has stricter or looser conventions (peek at 2–3 existing `.md` files before writing the schema), tune `required`, `additionalProperties`, and `forbidden-sections` accordingly. Mention what you tuned and why in chat.

## Register mdtype with the agent's instructions file

Detect which instructions file this project uses, in order:

1. `./CLAUDE.md` exists → append to it.
2. `./AGENTS.md` exists → append to it.
3. Neither exists → create `./AGENTS.md` (universally honoured by Claude Code, Codex, Cursor, and most other coding agents).

**Append** (don't overwrite) the section below. Wrap it in clear sentinel comments so a future re-run of this skill can locate and update it idempotently:

````markdown
<!-- mdtype:begin -->

## Validate Markdown with `mdtype`

This project uses [`mdtype`](https://github.com/serejke/mdtype) as a blocking gate on Markdown changes. Treat it the way you treat the test suite or the linter — a red `mdtype` is a red CI build, and you are not done until it is green.

**Run before declaring "done":**

```sh
mdtype --format json .
```
````

Exit codes:

- `0` — clean.
- `1` — at least one diagnostic of severity `error`. Fix every diagnostic. Each diagnostic carries `rule`, `file`, `line`, a short imperative `message`, and an optional `fixit` hint — use them; do not scrape the human-readable text.
- `2` — config or schema problem. Tell the user; do not try to "fix" the schema yourself unless they ask.

**Schema lives at `.mdtype.yaml`** (root) + `.mdtype/schemas/`. Closest config to cwd wins, no merging across nested configs. To apply different rules to different folders, add globs to the root config — do not create subdirectory `.mdtype.yaml` files.

**When you write or edit a Markdown file**, satisfy its schema:

- Quote any frontmatter scalar that contains a colon (`description: "Long sentence: with colon"`) — the YAML parser will otherwise split it into two keys.
- Body sections are level-2 (`##`) headings; leave H1 to the file title.
- Forbidden sections (`## TODO`, `## Scratch`, etc.) must not ship — move that content into the body or out of the file.

**Diagnostic message style** is documented at <https://github.com/serejke/mdtype/blob/main/docs/error-messages.md>. Honour it when you propose new rules.

<!-- mdtype:end -->

````

If the sentinel block already exists (re-run case), replace it in-place rather than appending a second copy.

## Offer hook installation

After the files are written, offer the user two follow-ups (do not install without asking):

1. **Pre-commit hook** — blocks bad Markdown from landing. Install with:
   ```sh
   curl -sSL https://raw.githubusercontent.com/serejke/mdtype/main/hooks/pre-commit -o .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit
````

Or, if the project already uses the [pre-commit framework](https://pre-commit.com), suggest the `.pre-commit-config.yaml` snippet from `mdtype/hooks/README.md`.

2. **Claude Code Stop hook** — blocks the agent (you) from declaring "done" while diagnostics exist. Install with:
   ```sh
   mkdir -p .claude && curl -sSL https://raw.githubusercontent.com/serejke/mdtype/main/hooks/claude-code-stop -o .claude/mdtype-stop.sh && chmod +x .claude/mdtype-stop.sh
   ```
   Then wire into `.claude/settings.json`:
   ```json
   {
     "hooks": {
       "Stop": [{ "matcher": "*", "command": "./.claude/mdtype-stop.sh" }]
     }
   }
   ```

## Verify before reporting done

1. Run `mdtype --format json .` and check the JSON output. It should be either:
   - `{"summary":{"errors":0,...}}` — clean. Report that to the user.
   - `{"summary":{"errors":N,...},"diagnostics":[...]}` — surface the count and the first 3 diagnostics so the user can decide whether to fix-up content or loosen the schema.
2. If `mdtype` exit was `2`, surface the stderr message — schema or config is broken.

## Report

Tell the user, in this order:

1. What files you created (relative paths).
2. Which instructions file got the `<!-- mdtype:begin -->` block.
3. The sweep result (errors / files scanned).
4. The two follow-up hook installation commands they can run if they want.

Keep the report tight. Five short bullets is enough.
