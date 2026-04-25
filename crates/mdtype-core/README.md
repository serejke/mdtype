# mdtype-core

Core data model + traits for [`mdtype`](https://github.com/serejke/mdtype) — a type checker for Markdown.

This crate defines the `Diagnostic`, `Severity`, `Fixit`, `Summary`, `Schema`, `SchemaEntry`, `ParsedDocument` types and the `BodyRule`, `BodyRuleFactory`, `SchemaSource`, `Reporter`, `Validator` traits, plus the parser and the default `CoreValidator`. It depends on no sibling crate — downstream users may build on `mdtype-core` alone and supply their own schema source, rules, and reporter.

See [`docs/extending.md`](https://github.com/serejke/mdtype/blob/main/docs/extending.md) in the project repo for a working example.

## License

MIT.
