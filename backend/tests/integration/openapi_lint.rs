//! Quality gate for the generated OpenAPI spec.
//!
//! The spec is the only description of the API that reaches the docs site and the
//! generated clients, so gaps in it are invisible here and obvious there. This runs
//! against the same public spec `generate-openapi` writes to disk, and fails on the
//! classes of defect that had accumulated silently: fields with no description,
//! union variants with no label, response bodies with no schema, and free-form
//! strings that have a real enum sitting next to them.
//!
//! Run with: `cargo test openapi_lint -- --nocapture`

use scanopy::server::openapi::public_spec;
use serde_json::Value;
use std::collections::BTreeSet;

/// Properties utoipa synthesises for `#[serde(tag = "...")]` enums. They are
/// single-value string enums naming their own variant, so there is no Rust field
/// to hang a doc comment on and nothing a description would add.
fn is_serde_tag(schema: &Value) -> bool {
    schema
        .get("enum")
        .and_then(Value::as_array)
        .map(|e| e.len() == 1)
        == Some(true)
        && schema.get("type").and_then(Value::as_str) == Some("string")
}

/// A property counts as documented if the description sits either directly on it or
/// on one of the wrapper members utoipa emits for `Option<T>` (`oneOf`) and flattened
/// structs (`allOf`).
///
/// A property that is only a `$ref` also counts: it has no text of its own to carry a
/// description, and renderers show the referenced schema's instead. Those schemas are
/// checked separately by [`component_descriptions`].
fn has_description(schema: &Value) -> bool {
    if schema
        .get("description")
        .and_then(Value::as_str)
        .is_some_and(|d| !d.trim().is_empty())
    {
        return true;
    }
    if schema.get("$ref").is_some() {
        return true;
    }
    ["oneOf", "anyOf", "allOf"].iter().any(|key| {
        schema
            .get(key)
            .and_then(Value::as_array)
            .is_some_and(|members| members.iter().any(has_description))
    })
}

/// Whether a `oneOf` member is a union variant that renderers need a label for.
///
/// Not every `oneOf` is a union: utoipa also uses it for `Option<T>` (a `null` member
/// beside the real one), and a `$ref` member is already labelled by its schema name.
/// It also cannot put a `title` on a variant it renders as `allOf` — a struct variant
/// with a flattened field — and the docs pipeline recovers those from the tag property.
fn needs_title(schema: &Value) -> bool {
    let is_null = schema.get("type").and_then(Value::as_str) == Some("null");
    let is_ref = schema.get("$ref").is_some();
    let is_all_of = schema.get("allOf").is_some();
    let is_inline_object = schema.get("properties").is_some();
    is_inline_object && !is_null && !is_ref && !is_all_of
}

fn collect_refs(node: &Value, out: &mut BTreeSet<String>) {
    match node {
        Value::Object(map) => {
            if let Some(name) = map
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|r| r.strip_prefix("#/components/schemas/"))
            {
                out.insert(name.to_string());
            }
            for value in map.values() {
                collect_refs(value, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|i| collect_refs(i, out)),
        _ => {}
    }
}

struct Lint {
    failures: Vec<String>,
    enum_names: BTreeSet<String>,
}

impl Lint {
    fn walk_schema(&mut self, path: &str, schema: &Value, depth: usize) {
        if depth > 8 {
            return;
        }

        if let Some(props) = schema.get("properties").and_then(Value::as_object) {
            for (name, prop) in props {
                if !has_description(prop) && !is_serde_tag(prop) {
                    self.failures.push(format!("{path}.{name}: no description"));
                }
                self.check_stringly_typed(path, name, prop);
                self.walk_schema(&format!("{path}.{name}"), prop, depth + 1);
            }
        }

        if let Some(members) = schema.get("oneOf").and_then(Value::as_array) {
            // A `oneOf` with a single non-null member is `Option<T>`, not a union,
            // so there is nothing for a label to disambiguate.
            let is_union = members
                .iter()
                .filter(|m| m.get("type").and_then(Value::as_str) != Some("null"))
                .count()
                > 1;
            for (i, member) in members.iter().enumerate() {
                if is_union && member.get("title").is_none() && needs_title(member) {
                    self.failures
                        .push(format!("{path}: oneOf variant {i} has no title"));
                }
                self.walk_schema(path, member, depth + 1);
            }
        }
        for key in ["anyOf", "allOf"] {
            if let Some(members) = schema.get(key).and_then(Value::as_array) {
                for member in members {
                    self.walk_schema(path, member, depth + 1);
                }
            }
        }
        if let Some(items) = schema.get("items") {
            self.walk_schema(&format!("{path}[]"), items, depth + 1);
        }
    }

    /// A bare string named after an enum that already exists in the spec is almost
    /// always a field that should have been typed with it.
    fn check_stringly_typed(&mut self, path: &str, name: &str, prop: &Value) {
        let is_bare_string = prop.get("type").and_then(Value::as_str) == Some("string")
            && prop.get("enum").is_none()
            && prop.get("format").is_none();
        if !is_bare_string {
            return;
        }
        let flat = name.replace('_', "").to_lowercase();
        if let Some(matched) = self
            .enum_names
            .iter()
            .find(|e| **e == flat || e.ends_with(&flat) && flat.len() > 3)
        {
            self.failures.push(format!(
                "{path}.{name}: free-form string, but a `{matched}` enum exists"
            ));
        }
    }
}

#[test]
fn openapi_lint() {
    let spec = public_spec();
    let spec: Value = serde_json::to_value(&spec).expect("spec should serialize");

    assert!(
        spec.get("servers")
            .and_then(Value::as_array)
            .is_some_and(|s| !s.is_empty()),
        "spec declares no `servers`; documentation renderers fall back to a placeholder host"
    );

    let schemas = spec
        .pointer("/components/schemas")
        .and_then(Value::as_object)
        .expect("spec should have component schemas");

    let enum_names = schemas
        .iter()
        .filter(|(_, s)| s.get("enum").is_some())
        .map(|(n, _)| n.replace('_', "").to_lowercase())
        .collect();

    let mut lint = Lint {
        failures: Vec::new(),
        enum_names,
    };

    for (name, schema) in schemas {
        lint.walk_schema(name, schema, 0);
    }

    let paths = spec
        .get("paths")
        .and_then(Value::as_object)
        .expect("spec should have paths");
    for (route, item) in paths {
        for (method, op) in item.as_object().into_iter().flatten() {
            let Some(op) = op.as_object() else { continue };

            for param in op
                .get("parameters")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let pname = param.get("name").and_then(Value::as_str).unwrap_or("?");
                if param
                    .get("description")
                    .and_then(Value::as_str)
                    .is_none_or(|d| d.trim().is_empty())
                {
                    lint.failures
                        .push(format!("{method} {route} ?{pname}: no description"));
                }
                if let Some(schema) = param.get("schema") {
                    lint.check_stringly_typed(&format!("{method} {route} ?"), pname, schema);
                }
            }

            let bodies = op.get("requestBody").into_iter().chain(
                op.get("responses")
                    .and_then(Value::as_object)
                    .into_iter()
                    .flat_map(|r| r.values()),
            );
            for body in bodies {
                for (media_type, content) in body
                    .get("content")
                    .and_then(Value::as_object)
                    .into_iter()
                    .flatten()
                {
                    if content.get("schema").is_none() {
                        lint.failures.push(format!(
                            "{method} {route}: `{media_type}` body has no schema"
                        ));
                    }
                }
            }
        }
    }

    // An operation tagged with something the spec never declares gets no description, and
    // shows up as its own unexplained group. Case drift is how `daemons` came to sit beside
    // `Daemons`, so this compares exactly.
    let declared: BTreeSet<&str> = spec
        .get("tags")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();
    let mut used = BTreeSet::new();
    for (_route, item) in paths {
        for (_method, op) in item.as_object().into_iter().flatten() {
            for tag in op
                .get("tags")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(tag) = tag.as_str() {
                    used.insert(tag);
                }
            }
        }
    }
    for tag in used.difference(&declared) {
        lint.failures.push(format!(
            "tag `{tag}` is used but never declared in the spec"
        ));
    }

    // A `$ref` with nothing behind it renders as an empty box and breaks client generators.
    let mut refs = BTreeSet::new();
    collect_refs(&spec, &mut refs);
    for name in refs {
        if !schemas.contains_key(&name) {
            lint.failures
                .push(format!("$ref to `{name}`, which no schema defines"));
        }
    }

    if !lint.failures.is_empty() {
        lint.failures.sort();
        panic!(
            "OpenAPI spec has {} documentation gaps:\n{}",
            lint.failures.len(),
            lint.failures.join("\n")
        );
    }
}
