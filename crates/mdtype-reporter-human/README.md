# mdtype-reporter-human

Human-readable reporter for [`mdtype`](https://github.com/serejke/mdtype): diagnostics grouped by file with a bold header, indented one line per finding, summary trails the list. Silent on success. Color follows the constructor flag (the CLI clears it for non-tty stdout / `--no-color`).

Depends only on `mdtype-core`. The structured machine-readable output lives in [`mdtype-reporter-json`](https://crates.io/crates/mdtype-reporter-json).

## License

MIT.
