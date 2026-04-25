# mdtype-schema-yaml

YAML-backed `SchemaSource` for [`mdtype`](https://github.com/serejke/mdtype) — reads `.mdtype.yaml`, walks each `rules:` entry, and resolves `body:` rules through a caller-supplied `BodyRuleFactory` registry.

Depends only on `mdtype-core`. See the [project README](https://github.com/serejke/mdtype) and [`docs/schema.md`](https://github.com/serejke/mdtype/blob/main/docs/schema.md) for the schema format.

## License

MIT.
