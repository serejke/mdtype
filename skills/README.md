# Agent skills

Drop-in skills for coding agents (Claude Code, Codex, Cursor, …) that bootstrap or operate `mdtype`.

| Skill                                   | What it does                                                                                                                                                  |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`setup-mdtype`](setup-mdtype/SKILL.md) | One-shot init: writes `.mdtype.yaml` + a starter schema, registers mdtype with the agent's instruction file (`CLAUDE.md` / `AGENTS.md`), offers hook install. |

## Install

The skills follow the [`vercel-labs/skills`](https://github.com/vercel-labs/skills) convention — frontmatter is `{name, description}`, files live under `skills/<name>/SKILL.md`, agents resolve them via the `npx skills` CLI.

**Install one skill into the current project, auto-detecting the agent:**

```sh
npx skills add serejke/mdtype --skill setup-mdtype
```

**Specify the agent explicitly** (Claude Code, Codex, Cursor, …):

```sh
npx skills add serejke/mdtype --skill setup-mdtype -a claude-code
npx skills add serejke/mdtype --skill setup-mdtype -a codex
```

Per-agent install paths (handled by `npx skills` automatically):

| Agent       | Project path      | Global path         |
| ----------- | ----------------- | ------------------- |
| Claude Code | `.claude/skills/` | `~/.claude/skills/` |
| Codex       | `.agents/skills/` | `~/.codex/skills/`  |
| Cursor      | `.agents/skills/` | `~/.cursor/skills/` |

**Install globally** (one user, every project):

```sh
npx skills add serejke/mdtype --skill setup-mdtype --global
```

**Manual install** (no `npx`, just `cp`):

```sh
mkdir -p .claude/skills/setup-mdtype
curl -sSL https://raw.githubusercontent.com/serejke/mdtype/main/skills/setup-mdtype/SKILL.md \
  -o .claude/skills/setup-mdtype/SKILL.md
```

## Use

Once installed, invoke from inside the agent:

- Claude Code: `/setup-mdtype`
- Codex / Cursor: `/setup-mdtype` or describe the intent ("set up mdtype in this project").

The skill writes config + schemas, registers itself with `CLAUDE.md` / `AGENTS.md`, runs a first sweep, and reports the findings. Detailed steps in [`setup-mdtype/SKILL.md`](setup-mdtype/SKILL.md).

## Why a skill, not a `mdtype init` subcommand

`mdtype` itself stays a focused validator (one binary, no scaffolding). Project setup, instruction-file editing, and hook wiring are agent-shaped tasks — they need to read the project, ask the user, and write multiple files. A skill is the natural delivery vehicle: portable across agents via the `npx skills` convention, no new commands on the binary's surface, and the install path is one line.
