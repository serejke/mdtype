# Contributing to `mdtype`

Thanks for the interest. `mdtype` is a small, opinionated tool — please read this page before opening a PR.

## Working tree

```
git clone https://github.com/serejke/mdtype
cd mdtype
cargo build --workspace          # MSRV 1.89, pinned via rust-toolchain.toml
cargo test  --workspace
```

## The gates

Every change must pass these locally and in CI (`.github/workflows/ci.yml`):

```
cargo fmt   --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test  --workspace
```

If you touched a rule or the validator, also regenerate the end-to-end fixture goldens:

```
UPDATE_FIXTURES=1 cargo test -p mdtype-tests --test fixtures
```

The harness writes one `expected/stdout.human`, `expected/stdout.json`, and (for error paths) `expected/stderr` per scenario. Diff those alongside your code change so reviewers can see the user-visible effect.

## Where things live

| Layer               | Crate                   | Add a new …                                     |
| ------------------- | ----------------------- | ----------------------------------------------- |
| Trait / data model  | `mdtype-core`           | type or trait — never a concrete rule           |
| YAML schema source  | `mdtype-schema-yaml`    | config-format change (rare)                     |
| Built-in body rule  | `mdtype-rules-stdlib`   | new `BodyRule` + `BodyRuleFactory`              |
| Human reporter      | `mdtype-reporter-human` | format tweak (snapshot-pinned)                  |
| JSON reporter       | `mdtype-reporter-json`  | **bump `CONTRACT_VERSION`** if you change shape |
| CLI                 | `mdtype`                | new flag (justify in commit message)            |
| End-to-end scenario | `mdtype-tests`          | folder under `fixtures/<scenario>/`             |

For an extension that lives outside the repo (custom `BodyRule`, `SchemaSource`, `Reporter`), see [`docs/extending.md`](docs/extending.md).

## Architectural invariants

These are the rules. Violations get bounced — please don't ignore them.

1. **`mdtype-core` depends on no sibling crate.** If your change adds such a dep, redesign.
2. **No rule logic in `mdtype-core`.** New rules go into `mdtype-rules-stdlib` or a downstream crate.
3. **Stable rule ids.** Renaming a `rule` id is a breaking change to the JSON contract — bump `version` and document it in `docs/json-schema.md` + the CHANGELOG.
4. **Reuse, don't reinvent.** Frontmatter validation IS JSON Schema (via `jsonschema`). Markdown parsing IS CommonMark (via `comrak`). Don't write a new YAML parser, JSON Schema engine, or Markdown parser.
5. **No autofix.** Diagnostics may carry a `fixit` _hint_; `mdtype` never rewrites files.
6. **Neutral examples only.** Fixtures and docs use the generic `examples/blog-site/` domain. No personal-workflow names.

## Diagnostic messages

Every `Diagnostic.message` is consumed by humans **and** by LLM agents driving fixes. Read [`docs/error-messages.md`](docs/error-messages.md) before adding or editing one.

## Commits

- **Conventional Commits**: `<type>(<scope>): <subject>`. Types: `feat`, `fix`, `chore`, `refactor`, `docs`, `test`, `perf`, `build`, `ci`, `style`, `revert`. Breaking changes get `!` after type/scope.
- **One logical change per commit.** Refactors and behaviour changes don't ride together.
- **No bot-generated co-author / footer lines.**

## Releasing

Distribution is crates.io only — no prebuilt binaries, no release matrix. Users `cargo install mdtype` (or build from source). Maintainer flow:

1. Land everything for the release on `main`. CI green.
2. Update `CHANGELOG.md`: move the relevant items from `[Unreleased]` into a new `[X.Y.Z] — YYYY-MM-DD` section. Refresh the comparison links at the bottom.
3. Bump the workspace `version` in `Cargo.toml` (`[workspace.package]`).
4. Commit: `chore(release): vX.Y.Z`. Tag: `git tag -a vX.Y.Z -m "vX.Y.Z"` and push.
5. Publish to crates.io in dependency order, waiting a few seconds between each for the index to catch up:
   ```sh
   for c in mdtype-core mdtype-rules-stdlib mdtype-schema-yaml \
            mdtype-reporter-human mdtype-reporter-json mdtype; do
     cargo publish -p "$c" && sleep 10
   done
   ```
   `mdtype-tests` stays unpublished — it has `publish = false`.

## Filing a bug

Include:

- The exact `mdtype` invocation.
- A minimal `.mdtype.yaml` + schema + offending `.md` snippet.
- Actual vs expected JSON output (`mdtype --format json` is the canonical reproducer).
- Your `mdtype --version`.
