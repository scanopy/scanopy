//! Naming and visibility rules for every operation in the full spec.
//!
//! The public spec is the API's documentation, and the full spec's operation ids key the
//! generated client. Both drifted one handler at a time: `Delete daemon_api_key` beside
//! `Delete Credential`, `get_all_vlans` beside `list_ports`, and UI-only endpoints in the public
//! docs. This fails on each of those, so a new endpoint has to follow the rules to merge.
//!
//! Run with: `cargo test --lib openapi::conventions`

use super::full_spec;
use super::tags::INTERNAL;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::handlers::openapi_macros::api_text;
use serde_json::Value;
use std::collections::BTreeSet;
use strum::IntoEnumIterator;

/// Longer summaries belong in the description, which renders under the title.
const SUMMARY_MAX_LEN: usize = 60;

/// Words that can't follow a summary's verb: "List all hosts" and "Create a new host" are second
/// spellings of "List hosts" and "Create host".
const BANNED_AFTER_VERB: &[&str] = &["a", "an", "the", "all", "new"];

/// Public operations whose security doesn't list `user_api_key`, and why they're public anyway.
const PUBLIC_WITHOUT_API_KEY: &[(&str, &str)] = &[(
    "get_version",
    "Unauthenticated, so a client can check compatibility before it holds a key",
)];

/// Public entity operations whose id doesn't name the entity, and why.
const ID_WITHOUT_ENTITY_NAME: &[(&str, &str)] = &[(
    "list_members",
    "Members are the organization's users together with their pending invites",
)];

const METHODS: [&str; 5] = ["get", "post", "put", "patch", "delete"];

struct EntityNames {
    /// The entity's tag, `ENTITY_NAME_PLURAL`.
    tag: &'static str,
    singular_id: String,
    plural_id: String,
    singular_text: String,
    plural_text: String,
}

fn entity_names() -> Vec<EntityNames> {
    let id = |name: &str| name.to_lowercase().replace(' ', "_");
    EntityDiscriminants::iter()
        .map(|entity| EntityNames {
            tag: entity.entity_name_plural(),
            singular_id: id(entity.entity_name_singular()),
            plural_id: id(entity.entity_name_plural()),
            singular_text: api_text(entity.entity_name_singular()),
            plural_text: api_text(entity.entity_name_plural()),
        })
        .collect()
}

struct Operation<'a> {
    method: &'a str,
    path: &'a str,
    id: &'a str,
    summary: &'a str,
    tags: Vec<&'a str>,
    security: Vec<&'a str>,
}

impl Operation<'_> {
    fn is_public(&self) -> bool {
        !self.tags.contains(&INTERNAL)
    }

    fn label(&self) -> String {
        format!("{} {} ({})", self.method, self.path, self.id)
    }
}

fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn operations(spec: &Value) -> Vec<Operation<'_>> {
    let mut out = Vec::new();
    for (path, item) in spec["paths"].as_object().expect("spec has paths") {
        for method in METHODS {
            let op = &item[method];
            if op.is_null() {
                continue;
            }
            out.push(Operation {
                method,
                path,
                id: op["operationId"].as_str().unwrap_or_default(),
                summary: op["summary"].as_str().unwrap_or_default(),
                tags: strings(&op["tags"]),
                security: op["security"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_object)
                    .flat_map(|requirement| requirement.keys().map(String::as_str))
                    .collect(),
            });
        }
    }
    out
}

/// Every place an entity name appears in `summary` with other than its canonical spelling. Longer
/// names are matched first, so "Tag group" is not also read as "Tag".
fn misspelled_entity_names(summary: &str, entities: &[EntityNames]) -> Vec<String> {
    let mut names: Vec<&str> = entities
        .iter()
        .flat_map(|e| [e.singular_text.as_str(), e.plural_text.as_str()])
        .collect();
    names.sort_by_key(|n| std::cmp::Reverse(n.len()));

    let lower = summary.to_lowercase();
    let mut consumed = vec![false; summary.len()];
    let mut wrong = Vec::new();
    for name in names {
        let needle = name.to_lowercase();
        for (start, _) in lower.match_indices(&needle) {
            let end = start + needle.len();
            let bounded = !lower[..start].ends_with(|c: char| c.is_alphanumeric())
                && !lower[end..].starts_with(|c: char| c.is_alphanumeric());
            if !bounded || consumed[start..end].iter().any(|c| *c) {
                continue;
            }
            consumed[start..end].iter_mut().for_each(|c| *c = true);
            let found = &summary[start..end];
            let at_start = start == 0 && found[1..] == name[1..];
            if found != name && !at_start {
                wrong.push(format!("`{found}` should read `{name}`"));
            }
        }
    }
    wrong
}

fn check_summary(op: &Operation, entities: &[EntityNames], failures: &mut Vec<String>) {
    let summary = op.summary;
    let mut fail = |problem: String| failures.push(format!("{}: summary {problem}", op.label()));
    if summary.trim().is_empty() {
        fail("is missing".into());
        return;
    }
    if summary.contains('\n') {
        fail("spans lines; put the detail after a blank doc-comment line".into());
    }
    if summary.ends_with('.') {
        fail(format!("`{summary}` ends with a period"));
    }
    if summary.len() > SUMMARY_MAX_LEN {
        fail(format!("`{summary}` is over {SUMMARY_MAX_LEN} characters"));
    }
    if !summary.starts_with(|c: char| c.is_uppercase()) {
        fail(format!("`{summary}` doesn't start with a capital"));
    }
    let mut words = summary.split_whitespace();
    if let (Some(verb), Some(next)) = (words.next(), words.next())
        && BANNED_AFTER_VERB.contains(&next.to_lowercase().as_str())
    {
        fail(format!("`{summary}`: drop `{next}` after `{verb}`"));
    }
    for word in summary.split_whitespace() {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if word.contains('_') && word.chars().all(|c| c.is_alphanumeric() || c == '_') {
            fail(format!("`{summary}` contains the identifier `{word}`"));
        }
    }
    for problem in misspelled_entity_names(summary, entities) {
        fail(format!("`{summary}`: {problem}"));
    }
}

fn is_snake_case(id: &str) -> bool {
    !id.is_empty()
        && id.starts_with(|c: char| c.is_ascii_lowercase())
        && !id.ends_with('_')
        && !id.contains("__")
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The operation id the CRUD shape of `op` calls for, if it has one.
fn crud_operation_id(
    op: &Operation,
    entity: &EntityNames,
    paths: &BTreeSet<&str>,
) -> Option<String> {
    let (s, p) = (&entity.singular_id, &entity.plural_id);
    let path = op.path;
    if let Some(collection) = path.strip_suffix("/{id}")
        && paths.contains(collection)
    {
        return match op.method {
            "get" => Some(format!("get_{s}_by_id")),
            "put" => Some(format!("update_{s}")),
            "delete" => Some(format!("delete_{s}")),
            _ => None,
        };
    }
    if paths.contains(format!("{path}/{{id}}").as_str()) {
        return match op.method {
            "get" => Some(format!("list_{p}")),
            "post" => Some(format!("create_{s}")),
            _ => None,
        };
    }
    match op.method {
        "post" if path.ends_with("/bulk-delete") => Some(format!("bulk_delete_{p}")),
        "get" if path.ends_with("/export/csv") => Some(format!("export_{p}_csv")),
        "get" if path.ends_with("/field-values") => Some(format!("get_{s}_field_values")),
        _ => None,
    }
}

fn check_operation_id(
    op: &Operation,
    entity: Option<&EntityNames>,
    paths: &BTreeSet<&str>,
    failures: &mut Vec<String>,
) {
    let mut fail =
        |problem: String| failures.push(format!("{}: operation id {problem}", op.label()));
    if !is_snake_case(op.id) {
        fail("isn't snake_case".into());
    }
    for prefix in ["get_all_", "get_by_id_"] {
        if op.id.starts_with(prefix) {
            fail(format!(
                "starts with `{prefix}`; use `list_` or `get_<entity>_by_id`"
            ));
        }
    }
    let Some(entity) = entity else { return };
    if let Some(expected) = crud_operation_id(op, entity, paths)
        && op.id != expected
    {
        fail(format!("should be `{expected}`"));
    }
    let names_entity = op.id.contains(&entity.singular_id) || op.id.contains(&entity.plural_id);
    let allowed = ID_WITHOUT_ENTITY_NAME.iter().any(|(id, _)| *id == op.id);
    if op.is_public() && !names_entity && !allowed {
        fail(format!("doesn't name `{}`", entity.singular_id));
    }
}

fn check_public(op: &Operation, failures: &mut Vec<String>) {
    let label = op.label();
    if op.tags.len() != 1 {
        failures.push(format!(
            "{label}: public operations take exactly one tag, has {:?}",
            op.tags
        ));
    }
    let allowed = PUBLIC_WITHOUT_API_KEY.iter().any(|(id, _)| *id == op.id);
    if !op.security.contains(&"user_api_key") && !allowed {
        failures.push(format!(
            "{label}: public, but an API key can't call it (security {:?}). Tag it internal",
            op.security
        ));
    }
}

#[test]
fn operations_follow_naming_and_visibility_rules() {
    let spec = serde_json::to_value(full_spec()).expect("spec should serialize");
    let entities = entity_names();
    let ops = operations(&spec);
    let paths: BTreeSet<&str> = ops.iter().map(|op| op.path).collect();
    let mut failures = Vec::new();

    let mut ids = BTreeSet::new();
    for op in &ops {
        if !ids.insert(op.id) {
            failures.push(format!("{}: operation id is not unique", op.label()));
        }
        let entity = op
            .tags
            .iter()
            .find_map(|tag| entities.iter().find(|e| e.tag == *tag));
        check_summary(op, &entities, &mut failures);
        check_operation_id(op, entity, &paths, &mut failures);
        if op.is_public() {
            check_public(op, &mut failures);
        }
    }

    let public_tags: BTreeSet<&str> = ops
        .iter()
        .filter(|op| op.is_public())
        .flat_map(|op| op.tags.iter().copied())
        .collect();
    for tag in spec["tags"].as_array().into_iter().flatten() {
        let name = tag["name"].as_str().unwrap_or_default();
        if name == INTERNAL {
            continue;
        }
        if !public_tags.contains(name) {
            failures.push(format!(
                "tag `{name}` has no public operations; remove it from `ApiDoc`"
            ));
        }
        if !name
            .split(' ')
            .all(|w| w.starts_with(|c: char| c.is_uppercase()))
        {
            failures.push(format!("tag `{name}` isn't title case"));
        }
        if tag["description"]
            .as_str()
            .is_none_or(|d| !d.ends_with('.'))
        {
            failures.push(format!(
                "tag `{name}` needs a description written as sentences"
            ));
        }
    }

    if !failures.is_empty() {
        failures.sort();
        panic!(
            "{} operations break the API spec rules:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

#[test]
fn entity_names_keep_acronyms_mid_sentence() {
    assert_eq!(api_text("Daemon API Keys"), "daemon API keys");
    assert_eq!(api_text("IP Addresses"), "IP addresses");
    assert_eq!(api_text("VLANs"), "VLANs");
    assert_eq!(
        crate::api_text_name!("Tag Groups"),
        api_text("Tag Groups"),
        "the compile-time and runtime forms apply one rule"
    );
}

#[test]
fn misspelled_entity_names_are_found() {
    let entities = entity_names();
    assert!(misspelled_entity_names("Get VLAN by ID", &entities).is_empty());
    assert!(misspelled_entity_names("Tag groups for a site", &entities).is_empty());
    assert_eq!(
        misspelled_entity_names("Get Vlan by ID", &entities).len(),
        1
    );
    assert_eq!(
        misspelled_entity_names("Delete Daemon API Keys", &entities).len(),
        1
    );
}
