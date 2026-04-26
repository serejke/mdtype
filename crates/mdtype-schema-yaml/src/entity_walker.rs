//! Walk a frontmatter JSON Schema and synthesise [`ReferenceSpec`]s from `x-entity`
//! annotations.
//!
//! See `docs/types.md`.
//!
//! # Visited positions
//!
//! For every entry in `frontmatter.properties.<name>`:
//!
//! - `{ type: "string", x-entity: ... }` → scalar reference for `<name>`.
//! - `{ type: "array", items: { type: "string", x-entity: ... } }` → array reference.
//!
//! Anything else: no reference. If the property contains an unsupported construct
//! (`$ref`, `oneOf`, `anyOf`, `allOf`, `if/then/else`, `not`) and `x-entity` is
//! reachable through it, the schema fails to load with [`Error::Schema`].
//!
//! # `x-entity` value
//!
//! Must be either a non-empty string or a non-empty array of non-empty, non-duplicate
//! strings. Anything else fails the schema load.

use std::path::Path;

use mdtype_core::{Error, ReferenceSpec};
use serde_json::Value;

const FORBIDDEN_KEYS: &[&str] = &[
    "$ref", "oneOf", "anyOf", "allOf", "if", "then", "else", "not",
];

/// Walk `frontmatter` (a JSON Schema document) and return the list of typed-reference
/// specs synthesised from `x-entity` annotations.
///
/// `schema_path` is used only for error messages.
///
/// # Errors
///
/// Returns [`Error::Schema`] when an `x-entity` annotation is malformed or sits inside a
/// construct the v1 walker rejects (`$ref`, `oneOf`, `anyOf`, `allOf`, conditional
/// schemas, `not`).
pub fn walk(frontmatter: &Value, schema_path: &Path) -> Result<Vec<ReferenceSpec>, Error> {
    // The walker does not resolve `$ref` in v1. If the schema uses `$ref` anywhere AND
    // `x-entity` anywhere, the combination is rejected — the walker cannot see whether
    // `x-entity` is reachable through the ref and silent degradation is unacceptable.
    if contains_x_entity(frontmatter) && contains_ref(frontmatter) {
        return Err(Error::Schema(format!(
            "schema {}: `$ref` is not supported alongside `x-entity` in v1; declare typed-reference fields directly under 'properties' (without $ref) or remove the x-entity annotations",
            schema_path.display(),
        )));
    }

    let Some(properties) = frontmatter.get("properties").and_then(Value::as_object) else {
        return Ok(Vec::new());
    };

    let mut specs: Vec<ReferenceSpec> = Vec::new();
    for (field_name, prop_schema) in properties {
        let pointer = format!("/properties/{}", json_pointer_escape(field_name));
        if let Some(spec) = walk_property(field_name, prop_schema, &pointer, schema_path)? {
            specs.push(spec);
        }
    }
    Ok(specs)
}

fn contains_ref(value: &Value) -> bool {
    match value {
        Value::Object(obj) => {
            if obj.contains_key("$ref") {
                return true;
            }
            obj.values().any(contains_ref)
        }
        Value::Array(items) => items.iter().any(contains_ref),
        _ => false,
    }
}

fn walk_property(
    field: &str,
    prop_schema: &Value,
    pointer: &str,
    schema_path: &Path,
) -> Result<Option<ReferenceSpec>, Error> {
    let Some(obj) = prop_schema.as_object() else {
        // Non-object property schema (e.g. a `true`) — no reference here.
        // Still verify no x-entity is hiding under composition: a non-object schema
        // cannot, so we're done.
        return Ok(None);
    };

    let type_value = obj.get("type");
    let has_x_entity = obj.contains_key("x-entity");

    // Scalar reference: { type: "string", x-entity: ... } at this position.
    if has_x_entity {
        require_no_forbidden_at(obj, pointer, schema_path)?;
        if !is_string_type(type_value) {
            return Err(Error::Schema(format!(
                "schema {}: x-entity at {pointer} requires `type: string` (got {})",
                schema_path.display(),
                describe_type(type_value),
            )));
        }
        let targets = parse_x_entity(obj.get("x-entity").unwrap(), pointer, schema_path)?;
        return Ok(Some(ReferenceSpec {
            field: field.to_string(),
            targets,
        }));
    }

    // Array reference: { type: "array", items: { type: "string", x-entity: ... } }.
    if is_array_type(type_value) {
        if let Some(items) = obj.get("items") {
            let items_pointer = format!("{pointer}/items");
            if let Some(items_obj) = items.as_object() {
                if items_obj.contains_key("x-entity") {
                    require_no_forbidden_at(items_obj, &items_pointer, schema_path)?;
                    if !is_string_type(items_obj.get("type")) {
                        return Err(Error::Schema(format!(
                            "schema {}: x-entity at {items_pointer} requires `type: string` (got {})",
                            schema_path.display(),
                            describe_type(items_obj.get("type")),
                        )));
                    }
                    let targets = parse_x_entity(
                        items_obj.get("x-entity").unwrap(),
                        &items_pointer,
                        schema_path,
                    )?;
                    return Ok(Some(ReferenceSpec {
                        field: field.to_string(),
                        targets,
                    }));
                }
                // No x-entity on items — but we still have to check no x-entity is
                // hidden under forbidden constructs in items.
                forbid_hidden_x_entity(items, &items_pointer, schema_path)?;
            } else {
                forbid_hidden_x_entity(items, &items_pointer, schema_path)?;
            }
        }
    }

    // Property does not carry a typed reference at the top level. Make sure no
    // x-entity is hiding inside an unsupported construct.
    forbid_hidden_x_entity(prop_schema, pointer, schema_path)?;

    Ok(None)
}

/// Reject `x-entity` reachable through any forbidden construct from `value` downward.
fn forbid_hidden_x_entity(value: &Value, pointer: &str, schema_path: &Path) -> Result<(), Error> {
    match value {
        Value::Object(obj) => {
            for (key, child) in obj {
                if FORBIDDEN_KEYS.contains(&key.as_str()) {
                    if contains_x_entity(child) {
                        return Err(Error::Schema(format!(
                            "schema {}: x-entity reachable through '{key}' at {pointer}/{key} is not supported; declare the field directly under 'properties' with type 'string' or 'array of string'",
                            schema_path.display(),
                        )));
                    }
                    // No x-entity reachable through this forbidden key — safe to skip
                    // (the schema can use composition for non-reference fields).
                    continue;
                }
                let child_pointer = format!("{pointer}/{}", json_pointer_escape(key));
                forbid_hidden_x_entity(child, &child_pointer, schema_path)?;
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                let child_pointer = format!("{pointer}/{i}");
                forbid_hidden_x_entity(item, &child_pointer, schema_path)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Recursively check whether any `x-entity` keyword is present anywhere in `value`.
fn contains_x_entity(value: &Value) -> bool {
    match value {
        Value::Object(obj) => {
            if obj.contains_key("x-entity") {
                return true;
            }
            obj.values().any(contains_x_entity)
        }
        Value::Array(items) => items.iter().any(contains_x_entity),
        _ => false,
    }
}

fn require_no_forbidden_at(
    obj: &serde_json::Map<String, Value>,
    pointer: &str,
    schema_path: &Path,
) -> Result<(), Error> {
    for key in FORBIDDEN_KEYS {
        if obj.contains_key(*key) {
            return Err(Error::Schema(format!(
                "schema {}: x-entity at {pointer} cannot coexist with '{key}'",
                schema_path.display(),
            )));
        }
    }
    Ok(())
}

fn parse_x_entity(value: &Value, pointer: &str, schema_path: &Path) -> Result<Vec<String>, Error> {
    match value {
        Value::String(s) => {
            if s.is_empty() {
                return Err(Error::Schema(format!(
                    "schema {}: x-entity at {pointer} must be a non-empty string",
                    schema_path.display(),
                )));
            }
            Ok(vec![s.clone()])
        }
        Value::Array(items) => {
            if items.is_empty() {
                return Err(Error::Schema(format!(
                    "schema {}: x-entity at {pointer} must be a non-empty array of strings",
                    schema_path.display(),
                )));
            }
            let mut out: Vec<String> = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                let Some(s) = item.as_str() else {
                    return Err(Error::Schema(format!(
                        "schema {}: x-entity at {pointer}[{i}] must be a string",
                        schema_path.display(),
                    )));
                };
                if s.is_empty() {
                    return Err(Error::Schema(format!(
                        "schema {}: x-entity at {pointer}[{i}] must be a non-empty string",
                        schema_path.display(),
                    )));
                }
                if out.iter().any(|existing| existing == s) {
                    return Err(Error::Schema(format!(
                        "schema {}: x-entity at {pointer} contains duplicate target '{s}'",
                        schema_path.display(),
                    )));
                }
                out.push(s.to_string());
            }
            Ok(out)
        }
        _ => Err(Error::Schema(format!(
            "schema {}: x-entity at {pointer} must be a string or array of strings",
            schema_path.display(),
        ))),
    }
}

fn is_string_type(t: Option<&Value>) -> bool {
    matches!(t, Some(Value::String(s)) if s == "string")
}

fn is_array_type(t: Option<&Value>) -> bool {
    matches!(t, Some(Value::String(s)) if s == "array")
}

fn describe_type(t: Option<&Value>) -> String {
    match t {
        None => "no `type:` declared".to_string(),
        Some(Value::String(s)) => format!("`type: {s}`"),
        Some(other) => format!("`type: {other}`"),
    }
}

fn json_pointer_escape(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use super::walk;
    use serde_json::json;
    use std::path::Path;

    fn p() -> &'static Path {
        Path::new("schemas/test.yaml")
    }

    #[test]
    fn empty_schema_yields_no_specs() {
        let specs = walk(&json!({}), p()).expect("ok");
        assert!(specs.is_empty());
    }

    #[test]
    fn no_x_entity_yields_no_specs() {
        let s = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "tags": { "type": "array", "items": { "type": "string" } }
            }
        });
        let specs = walk(&s, p()).expect("ok");
        assert!(specs.is_empty());
    }

    #[test]
    fn scalar_reference_recognised() {
        let s = json!({
            "type": "object",
            "properties": {
                "author_profile": { "type": "string", "x-entity": "author" }
            }
        });
        let specs = walk(&s, p()).expect("ok");
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].field, "author_profile");
        assert_eq!(specs[0].targets, vec!["author".to_string()]);
    }

    #[test]
    fn array_reference_recognised() {
        let s = json!({
            "type": "object",
            "properties": {
                "reviewers": {
                    "type": "array",
                    "items": { "type": "string", "x-entity": "author" }
                }
            }
        });
        let specs = walk(&s, p()).expect("ok");
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].field, "reviewers");
        assert_eq!(specs[0].targets, vec!["author".to_string()]);
    }

    #[test]
    fn union_reference_recognised() {
        let s = json!({
            "type": "object",
            "properties": {
                "discussed_in": {
                    "type": "array",
                    "items": { "type": "string", "x-entity": ["meeting-transcript", "adr"] }
                }
            }
        });
        let specs = walk(&s, p()).expect("ok");
        assert_eq!(specs.len(), 1);
        assert_eq!(
            specs[0].targets,
            vec!["meeting-transcript".to_string(), "adr".to_string()]
        );
    }

    #[test]
    fn rejects_x_entity_on_non_string_type() {
        let s = json!({
            "type": "object",
            "properties": {
                "bad": { "type": "number", "x-entity": "author" }
            }
        });
        let err = walk(&s, p()).unwrap_err().to_string();
        assert!(err.contains("requires `type: string`"), "got: {err}");
    }

    #[test]
    fn rejects_empty_string_target() {
        let s = json!({
            "type": "object",
            "properties": { "f": { "type": "string", "x-entity": "" } }
        });
        let err = walk(&s, p()).unwrap_err().to_string();
        assert!(err.contains("non-empty string"), "got: {err}");
    }

    #[test]
    fn rejects_empty_array_target() {
        let s = json!({
            "type": "object",
            "properties": { "f": { "type": "string", "x-entity": [] } }
        });
        let err = walk(&s, p()).unwrap_err().to_string();
        assert!(err.contains("non-empty array"), "got: {err}");
    }

    #[test]
    fn rejects_non_string_in_array_target() {
        let s = json!({
            "type": "object",
            "properties": { "f": { "type": "string", "x-entity": ["a", 42] } }
        });
        let err = walk(&s, p()).unwrap_err().to_string();
        assert!(err.contains("must be a string"), "got: {err}");
    }

    #[test]
    fn rejects_duplicate_in_union() {
        let s = json!({
            "type": "object",
            "properties": { "f": { "type": "string", "x-entity": ["a", "a"] } }
        });
        let err = walk(&s, p()).unwrap_err().to_string();
        assert!(err.contains("duplicate target"), "got: {err}");
    }

    #[test]
    fn rejects_x_entity_under_oneof() {
        let s = json!({
            "type": "object",
            "properties": {
                "f": {
                    "oneOf": [
                        { "type": "string", "x-entity": "a" },
                        { "type": "string", "x-entity": "b" }
                    ]
                }
            }
        });
        let err = walk(&s, p()).unwrap_err().to_string();
        assert!(err.contains("oneOf"), "got: {err}");
        assert!(err.contains("not supported"), "got: {err}");
    }

    #[test]
    fn rejects_ref_alongside_x_entity() {
        // The walker does not resolve $ref in v1; the combination of $ref and x-entity
        // anywhere in the schema is rejected outright to avoid silent degradation.
        let s = json!({
            "type": "object",
            "$defs": {
                "authorRef": { "type": "string", "x-entity": "author" }
            },
            "properties": {
                "f": { "$ref": "#/$defs/authorRef" }
            }
        });
        let err = walk(&s, p()).unwrap_err().to_string();
        assert!(err.contains("$ref"), "got: {err}");
        assert!(err.contains("not supported"), "got: {err}");
    }

    #[test]
    fn ref_alone_without_x_entity_is_fine() {
        // Schemas that use $ref for non-reference fields are unaffected.
        let s = json!({
            "type": "object",
            "$defs": {
                "color": { "type": "string", "enum": ["red", "blue"] }
            },
            "properties": {
                "color": { "$ref": "#/$defs/color" }
            }
        });
        let specs = walk(&s, p()).expect("ok");
        assert!(specs.is_empty());
    }

    #[test]
    fn rejects_x_entity_under_inlined_ref_value() {
        // Demonstrates the fail path: x-entity sits inside a forbidden key's subtree.
        let s = json!({
            "type": "object",
            "properties": {
                "f": {
                    "allOf": [
                        { "type": "string", "x-entity": "a" }
                    ]
                }
            }
        });
        let err = walk(&s, p()).unwrap_err().to_string();
        assert!(err.contains("allOf"), "got: {err}");
    }

    #[test]
    fn rejects_x_entity_under_if() {
        let s = json!({
            "type": "object",
            "properties": {
                "f": {
                    "if": { "type": "string" },
                    "then": { "x-entity": "a" }
                }
            }
        });
        let err = walk(&s, p()).unwrap_err().to_string();
        assert!(err.contains("not supported"), "got: {err}");
    }

    #[test]
    fn rejects_x_entity_under_not() {
        let s = json!({
            "type": "object",
            "properties": {
                "f": {
                    "not": { "x-entity": "a" }
                }
            }
        });
        let err = walk(&s, p()).unwrap_err().to_string();
        assert!(err.contains("not supported"), "got: {err}");
    }

    #[test]
    fn forbidden_construct_without_x_entity_is_fine() {
        // Schemas can use composition for non-reference fields freely.
        let s = json!({
            "type": "object",
            "properties": {
                "status": {
                    "oneOf": [
                        { "type": "string", "enum": ["draft"] },
                        { "type": "string", "enum": ["published"] }
                    ]
                }
            }
        });
        let specs = walk(&s, p()).expect("ok");
        assert!(specs.is_empty());
    }

    #[test]
    fn multiple_properties_collected() {
        let s = json!({
            "type": "object",
            "properties": {
                "author_profile": { "type": "string", "x-entity": "author" },
                "reviewers": {
                    "type": "array",
                    "items": { "type": "string", "x-entity": "author" }
                }
            }
        });
        let specs = walk(&s, p()).expect("ok");
        assert_eq!(specs.len(), 2);
        let fields: Vec<&str> = specs.iter().map(|s| s.field.as_str()).collect();
        assert!(fields.contains(&"author_profile"));
        assert!(fields.contains(&"reviewers"));
    }
}
