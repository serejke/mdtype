# JSON Output Contract

`mdtype --format json` emits a single JSON document per run. The shape below is the **public contract** — consumers may rely on every documented field. Snapshot tests in `crates/mdtype-reporter-json/src/snapshots/` and `crates/mdtype/tests/snapshots/golden__blog_site_json.snap` pin it.

```json
{
  "version": "1",
  "summary": {
    "files_scanned": 4,
    "files_with_errors": 2,
    "errors": 2,
    "warnings": 0
  },
  "diagnostics": [
    {
      "file": "content/posts/hello.md",
      "line": null,
      "rule": "frontmatter.schema",
      "severity": "error",
      "message": "missing required field 'author'",
      "fixit": {
        "kind": "AddFrontmatterField",
        "field": "author",
        "hint": "string"
      }
    }
  ]
}
```

## Top-level

| Field         | Type     | Notes                                                                                |
| ------------- | -------- | ------------------------------------------------------------------------------------ |
| `version`     | string   | Currently `"1"`. Required, unconditional. Bumped on any breaking change (see below). |
| `summary`     | object   | Aggregate counts for the run. Always present.                                        |
| `diagnostics` | object[] | Findings, sorted by `(file, line, rule)`. Empty array on a clean run.                |

## `summary`

| Field               | Type    | Notes                                                               |
| ------------------- | ------- | ------------------------------------------------------------------- |
| `files_scanned`     | integer | Number of `.md` / `.markdown` files the CLI walked into.            |
| `files_with_errors` | integer | Number of files that produced at least one diagnostic.              |
| `errors`            | integer | Count of diagnostics with `severity == "error"`.                    |
| `warnings`          | integer | Count of diagnostics with `severity == "warning"` (always 0 in v1). |

## `diagnostics[]`

| Field      | Type            | Notes                                                                                             |
| ---------- | --------------- | ------------------------------------------------------------------------------------------------- |
| `file`     | string          | Path of the offending file as the CLI saw it (relative when input was relative).                  |
| `line`     | integer \| null | 1-indexed file line of the violation; `null` for whole-file issues (e.g. missing required field). |
| `rule`     | string          | Stable rule id, e.g. `body.forbid_h1`. Renames are breaking and bump `version`.                   |
| `severity` | string          | `"error"` or `"warning"`. v1 emits only `"error"`.                                                |
| `message`  | string          | Human-readable description. Wording may evolve; do not match against it programmatically.         |
| `fixit`    | object \| null  | Optional repair hint. `mdtype` itself never rewrites files. See [Fixit kinds](#fixit-kinds).      |

### Fixit kinds

A fixit is `null` or an object tagged by its `kind`. v1 ships four kinds:

| `kind`                  | Extra fields                                                   |
| ----------------------- | -------------------------------------------------------------- |
| `"AddFrontmatterField"` | `field: string`, `hint: string \| null`                        |
| `"DeleteLine"`          | `line: integer` (1-indexed)                                    |
| `"AppendSection"`       | `heading: string` (with `##` markers), `after: string \| null` |
| `"Custom"`              | `name: string` (rule-namespaced), `payload: any`               |

Downstream rules may emit `Custom` fixits with rule-specific payloads.

## Versioning

- `version` is required and unconditional.
- Any **breaking change** bumps `version` to `"2"` (etc.). Breaking changes include: removing or renaming a field, narrowing a value type, removing a fixit kind, renaming a rule id.
- Adding a new optional field, a new severity, a new fixit kind, or a new rule id is **non-breaking** and stays on the current `version`.
- Consumers may safely ignore unknown fields for forward compatibility.

## Exit codes

The JSON document is emitted on stdout regardless of exit code. Parsing it tells you what was found; the exit code tells you whether the build should fail:

| Exit code | Meaning                                                               |
| --------- | --------------------------------------------------------------------- |
| `0`       | All files clean. `summary.errors == 0` (and `summary.warnings == 0`). |
| `1`       | At least one diagnostic of severity `error`.                          |
| `2`       | Config or schema problem; no validation performed.                    |
