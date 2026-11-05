use super::*;
use crate::server::shared::types::entity_field::FieldWrite;
use crate::server::shared::types::examples;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use std::collections::HashSet;
use strum::IntoEnumIterator;

/// Related entities held in memory, keyed by type.
struct MemorySource(HashMap<EntityDiscriminants, Vec<Value>>);

#[async_trait]
impl RelatedSource for MemorySource {
    async fn by_column(
        &self,
        entity: EntityDiscriminants,
        column: &'static str,
        ids: &[Uuid],
    ) -> Result<Vec<Value>> {
        Ok(self
            .0
            .get(&entity)
            .into_iter()
            .flatten()
            .filter(|item| ids_at(item, column).iter().any(|id| ids.contains(id)))
            .cloned()
            .collect())
    }
}

/// The serialized example of each entity type that has one.
fn example(entity: EntityDiscriminants) -> Option<Value> {
    use EntityDiscriminants as E;
    let value = match entity {
        E::Site => serde_json::to_value(examples::site()),
        E::Host => serde_json::to_value(examples::host()),
        E::Subnet => serde_json::to_value(examples::subnet()),
        E::IPAddress => serde_json::to_value(examples::ip_address()),
        E::Port => serde_json::to_value(examples::port()),
        E::Dependency => serde_json::to_value(examples::dependency()),
        E::Service => serde_json::to_value(examples::service()),
        E::Binding => serde_json::to_value(examples::binding()),
        E::Tag => serde_json::to_value(examples::tag()),
        E::TagGroup => serde_json::to_value(examples::tag_group()),
        E::DaemonApiKey => serde_json::to_value(examples::daemon_api_key()),
        E::Daemon => serde_json::to_value(examples::daemon()),
        E::User => serde_json::to_value(examples::user()),
        E::Organization => serde_json::to_value(examples::organization()),
        E::Discovery => serde_json::to_value(examples::discovery()),
        E::Interface => serde_json::to_value(examples::interface()),
        E::Vlan
        | E::UserApiKey
        | E::Credential
        | E::Invite
        | E::Topology
        | E::Snapshot
        | E::Share => return None,
    };
    Some(value.expect("example serializes"))
}

/// The property schemas a type's serialized form can carry, from its OpenAPI schema, which
/// lists optional keys an example may omit.
fn schema_props(name: &str) -> HashMap<String, Value> {
    static SPEC: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
        serde_json::to_value(crate::server::openapi::full_spec()).unwrap()
    });
    let schemas = &SPEC["components"]["schemas"];
    fn collect(schemas: &Value, schema: &Value, out: &mut HashMap<String, Value>) {
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            let name = reference.rsplit('/').next().unwrap();
            collect(schemas, &schemas[name], out);
        }
        if let Some(props) = schema.get("properties").and_then(Value::as_object) {
            out.extend(props.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        for key in ["allOf", "oneOf", "anyOf"] {
            for part in schema
                .get(key)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                collect(schemas, part, out);
            }
        }
    }
    let mut out = HashMap::new();
    collect(schemas, &schemas[name], &mut out);
    out
}

/// Whether a property schema is an array, including an optional (`null | array`) one.
fn is_array(schema: &Value) -> bool {
    let names_array = |t: &Value| {
        t == "array"
            || t.as_array()
                .is_some_and(|ts| ts.iter().any(|t| t == "array"))
    };
    schema.get("type").is_some_and(names_array)
        || ["oneOf", "anyOf"].iter().any(|k| {
            schema
                .get(*k)
                .and_then(Value::as_array)
                .is_some_and(|parts| parts.iter().any(is_array))
        })
}

/// Every key a field's path and value read exists on the type it is read from, and `many`
/// is set exactly when one entity can yield several values.
fn check_field(root: &str, field: &EntityField) {
    if field.computed || field.unserialized {
        return;
    }
    let props = |name: &str| schema_props(name);
    let mut on = root.to_string();
    let mut embedded = false;
    let mut several = false;
    for hop in field.path {
        match *hop {
            Hop::Parent {
                entity: target,
                key,
            } => {
                if !embedded {
                    let schema = props(&on).remove(key);
                    let schema = schema
                        .unwrap_or_else(|| panic!("{root}.{}: no key `{key}` on {on}", field.id));
                    several |= is_array(&schema);
                }
                on = target.to_string();
                embedded = false;
            }
            Hop::Children {
                entity: target,
                key,
            } => {
                assert!(
                    props(target.as_ref()).contains_key(key),
                    "{root}.{}: no key `{key}` on {target}",
                    field.id
                );
                on = target.to_string();
                several = true;
            }
            Hop::Embedded { key } => {
                assert!(
                    props(&on).contains_key(key),
                    "{root}.{}: no array `{key}` on {on}",
                    field.id
                );
                embedded = true;
                several = true;
            }
        }
    }
    if let (Some(path), false) = (field.field, embedded) {
        let head = path.split('.').next().unwrap();
        let schema = props(&on).remove(head);
        let schema =
            schema.unwrap_or_else(|| panic!("{root}.{}: no key `{head}` on {on}", field.id));
        several |= field.path.is_empty() && is_array(&schema);
    }
    assert_eq!(
        field.many, several,
        "{root}.{}: `many` should be {several}",
        field.id
    );
}

#[test]
fn every_field_path_names_keys_the_serialized_entities_carry() {
    for entity in EntityDiscriminants::iter() {
        for field in entity.fields() {
            check_field(entity.as_ref(), field);
        }
    }
}

#[test]
fn every_edge_field_names_keys_its_edge_type_carries() {
    use crate::server::topology::types::edges::EdgeType;
    use strum::IntoEnumIterator;
    for edge_type in EdgeType::iter() {
        for field in edge_type.fields() {
            check_field("EdgeType", field);
        }
    }
}

/// Keys every entity may carry that the storage layer owns, so they need no registry entry.
const SYSTEM_KEYS: &[&str] = &[
    "id",
    "created_at",
    "updated_at",
    "valid_from",
    "valid_to",
    "lineage_id",
    "last_seen_at",
    "first_discovery_id",
    "last_discovery_id",
];

/// Keys read through `deserialize_lenient_option`, which turns a value of the wrong shape into
/// `None`, so no probe can see them being read.
const LENIENT_KEYS: &[&str] = &["virtualization_metadata"];

/// The serialized key a field reads from the entity itself, when it reads one.
fn own_key(field: &EntityField) -> Option<&'static str> {
    match field.path.first() {
        Some(Hop::Parent { key, .. } | Hop::Embedded { key }) => Some(key),
        Some(Hop::Children { .. }) => None,
        None => field.field.and_then(|path| path.split('.').next()),
    }
}

/// Whether `from_value` reads `key`: a value of the wrong shape refuses to parse if it does.
/// Two shapes, because a struct of defaults accepts any object and nothing but a bool or an
/// untyped value accepts `true`.
fn deserializes<T: DeserializeOwned>(base: &Value, key: &str) -> bool {
    [json!({ "__probe": true }), json!(true)]
        .iter()
        .any(|junk| {
            let mut probe = base.clone();
            probe[key] = junk.clone();
            serde_json::from_value::<T>(probe).is_err()
        })
}

/// What the registry promises about `T`: every serialized key has an entry, every rule runs
/// on `T`, and each field's serde and schema match its write policy.
fn registry_contract_breaks<T>(entity: EntityDiscriminants) -> Vec<String>
where
    T: Entity + Serialize + DeserializeOwned + Default + 'static,
{
    let fields = entity.fields();
    let props = schema_props(entity.as_ref());
    let mut breaks = Vec::new();

    let mut covered: HashSet<&str> = fields.iter().filter_map(|f| own_key(f)).collect();
    covered.extend(fields.iter().filter_map(|f| f.source_field));
    for key in props.keys().map(String::as_str) {
        if !SYSTEM_KEYS.contains(&key) && !covered.contains(key) {
            breaks.push(format!("{entity}.{key} has no registry field"));
        }
    }

    T::default().preserve_immutable_fields(&T::default());

    let base = example(entity)
        .unwrap_or_else(|| serde_json::to_value(T::default()).expect("default serializes"));
    if let Err(e) = serde_json::from_value::<T>(base.clone()) {
        breaks.push(format!("{entity}: the base payload must parse: {e}"));
        return breaks;
    }
    let mut probed = HashSet::new();
    for field in fields.iter().filter(|f| !f.computed) {
        let Some(key) = own_key(field) else { continue };
        if LENIENT_KEYS.contains(&key) || !probed.insert(key) {
            continue;
        }
        let server_owned = matches!(field.write, FieldWrite::ServerOwned(_));
        // Through `api::ignored`, or `skip` for a field that never serializes. utoipa treats
        // `skip_deserializing` as `skip`, which drops the field from the schema and so from
        // the generated types, though responses still carry it.
        if server_owned && deserializes::<T>(&base, key) {
            breaks.push(format!(
                "{entity}.{key} is server-owned, so a request must not be able to set it"
            ));
        }
        if !server_owned && !field.unserialized && !deserializes::<T>(&base, key) {
            breaks.push(format!(
                "{entity}.{key} isn't server-owned, so a request must be able to set it"
            ));
        }
        // utoipa writes `read_only` on an array's items.
        if server_owned
            && let Some(schema) = props.get(key)
            && schema.get("readOnly") != Some(&json!(true))
            && schema["items"].get("readOnly") != Some(&json!(true))
        {
            breaks.push(format!(
                "{entity}.{key} is server-owned, so its schema is read_only"
            ));
        }
    }
    breaks
}

#[test]
fn every_entity_keeps_its_registry_contract() {
    use crate::server::{
        bindings::r#impl::base::Binding, credentials::r#impl::base::Credential,
        daemon_api_keys::r#impl::base::DaemonApiKey, daemons::r#impl::base::Daemon,
        dependencies::r#impl::base::Dependency, discovery::r#impl::base::Discovery,
        hosts::r#impl::base::Host, interfaces::r#impl::base::Interface,
        invites::r#impl::base::Invite, ip_addresses::r#impl::base::IPAddress,
        organizations::r#impl::base::Organization, ports::r#impl::base::Port,
        services::r#impl::base::Service, shares::r#impl::base::Share, sites::r#impl::Site,
        snapshots::types::base::Snapshot, subnets::r#impl::base::Subnet,
        tags::groups::base::TagGroup, tags::r#impl::base::Tag, topology::types::base::Topology,
        user_api_keys::r#impl::base::UserApiKey, users::r#impl::base::User,
        vlans::r#impl::base::Vlan,
    };
    use EntityDiscriminants as E;
    let breaks: Vec<String> = EntityDiscriminants::iter()
        .flat_map(|entity| match entity {
            E::Site => registry_contract_breaks::<Site>(entity),
            E::Vlan => registry_contract_breaks::<Vlan>(entity),
            E::Subnet => registry_contract_breaks::<Subnet>(entity),
            E::Host => registry_contract_breaks::<Host>(entity),
            E::Service => registry_contract_breaks::<Service>(entity),
            E::IPAddress => registry_contract_breaks::<IPAddress>(entity),
            E::Interface => registry_contract_breaks::<Interface>(entity),
            E::Port => registry_contract_breaks::<Port>(entity),
            E::Binding => registry_contract_breaks::<Binding>(entity),
            E::Discovery => registry_contract_breaks::<Discovery>(entity),
            E::Daemon => registry_contract_breaks::<Daemon>(entity),
            E::DaemonApiKey => registry_contract_breaks::<DaemonApiKey>(entity),
            E::Tag => registry_contract_breaks::<Tag>(entity),
            E::TagGroup => registry_contract_breaks::<TagGroup>(entity),
            E::User => registry_contract_breaks::<User>(entity),
            E::UserApiKey => registry_contract_breaks::<UserApiKey>(entity),
            E::Credential => registry_contract_breaks::<Credential>(entity),
            E::Organization => registry_contract_breaks::<Organization>(entity),
            E::Invite => registry_contract_breaks::<Invite>(entity),
            E::Topology => registry_contract_breaks::<Topology>(entity),
            E::Snapshot => registry_contract_breaks::<Snapshot>(entity),
            E::Share => registry_contract_breaks::<Share>(entity),
            E::Dependency => registry_contract_breaks::<Dependency>(entity),
        })
        .collect();
    assert!(breaks.is_empty(), "{}", breaks.join("\n"));
}

/// Request schemas outside the registry whose ids a handler checks itself, and where.
const HANDLER_CHECKED: &[(&str, &str, &str)] = &[
    ("BindingInput", "ip_address_id", "binding_validation.rs"),
    ("BindingInput", "port_id", "binding_validation.rs"),
    ("BulkSetEnabledRequest", "ids", "discovery/bulk.rs load_all"),
    (
        "BulkTagRequest",
        "entity_ids",
        "validate_entities_in_tenant",
    ),
    ("BulkTagRequest", "tag_id", "validate_tag_full"),
    ("CreateBindingInput", "ip_address_id", "create_service"),
    ("CreateBindingInput", "port_id", "create_service"),
    ("CreateHostRequest", "tags", "validate_host_request"),
    (
        "CreateHostRequest",
        "virtualization_interface_id",
        "validate_host_request",
    ),
    (
        "CreateHostRequest",
        "virtualization_service_id",
        "validate_host_request",
    ),
    (
        "CreateInviteRequest",
        "site_ids",
        "validate_site_ids_access",
    ),
    ("CreateServiceRequest", "host_id", "create_service"),
    ("CreateServiceRequest", "tags", "create_service"),
    (
        "CreateServiceRequest",
        "virtualization_service_id",
        "create_service",
    ),
    (
        "CredentialAssignment",
        "credential_id",
        "validate_host_request",
    ),
    (
        "CredentialAssignment",
        "ip_address_ids",
        "validate_host_request",
    ),
    (
        "DaemonRegistrationRequest",
        "daemon_id",
        "process_registration",
    ),
    (
        "DependencyMembers",
        "binding_ids",
        "validate_dependency_members",
    ),
    (
        "DependencyMembers",
        "service_ids",
        "validate_dependency_members",
    ),
    (
        "DaemonRegistrationRequest",
        "user_id",
        "register_daemon's maintainer lookup",
    ),
    (
        "DiscoveryUpdatePayload",
        "daemon_id",
        "scope_discovery_report",
    ),
    (
        "DiscoveryUpdatePayload",
        "session_id",
        "scope_discovery_report",
    ),
    (
        "DiscoveryUpdatePayload",
        "discovery_id",
        "scope_discovery_report",
    ),
    ("DiscoveryType", "subnet_ids", "scope_discovery_report"),
    (
        "CredentialRunResult",
        "credential_id",
        "scope_discovery_report",
    ),
    ("ScannedEntityIds", "binding_ids", "retain_scanned_on_site"),
    (
        "ScannedEntityIds",
        "found_subnet_ids",
        "retain_scanned_on_site",
    ),
    ("ScannedEntityIds", "host_ids", "retain_scanned_on_site"),
    (
        "ScannedEntityIds",
        "interface_ids",
        "retain_scanned_on_site",
    ),
    (
        "ScannedEntityIds",
        "ip_address_ids",
        "retain_scanned_on_site",
    ),
    ("ScannedEntityIds", "port_ids", "retain_scanned_on_site"),
    ("ScannedEntityIds", "service_ids", "retain_scanned_on_site"),
    ("ScannedEntityIds", "subnet_ids", "retain_scanned_on_site"),
    ("ScannedEntityIds", "vlan_ids", "retain_scanned_on_site"),
    (
        "EmailInstallCommandRequest",
        "daemon_id",
        "validate_site_access",
    ),
    (
        "EntitlementRequest",
        "instance_id",
        "a signed license key, not a tenant row",
    ),
    ("IPAddressInput", "subnet_id", "validate_host_request"),
    (
        "IdentifiedRule_ElementRule",
        "tag_ids",
        "a display filter: foreign ids match nothing",
    ),
    (
        "IntegrationTarget",
        "credential_id",
        "provision_daemon, and both dispatch paths",
    ),
    ("InterfaceInput", "ip_address_id", "validate_host_request"),
    (
        "LegacyCapabilities",
        "interfaced_subnet_ids",
        "filter_existing_subnet_ids",
    ),
    (
        "MergeSubnetRequest",
        "into",
        "merge_subnet, then merge_into",
    ),
    (
        "ProvisionDaemonRequest",
        "daemon_id",
        "load_reprovision_target",
    ),
    ("ServiceInput", "tags", "validate_host_request"),
    (
        "ServiceInput",
        "virtualization_service_id",
        "validate_host_request",
    ),
    ("SetTagsRequest", "entity_id", "validate_entities_in_tenant"),
    ("SetTagsRequest", "tag_ids", "validate_tag_full"),
    (
        "TagOrderRequest",
        "tag_ids",
        "reorder_tags: a permutation of the group's own",
    ),
    ("UpdateHostRequest", "tags", "validate_host_request"),
    (
        "UpdateHostRequest",
        "virtualization_interface_id",
        "validate_host_request",
    ),
    (
        "UpdateHostRequest",
        "virtualization_service_id",
        "validate_host_request",
    ),
];

/// Ids a daemon reports that are stored on its own rows without being checked. None is followed
/// to a row outside the daemon's site: each is compared against site-filtered rows, or
/// resolved later in the viewer's own scope. `scope_discovery_report` fixes the daemon and
/// site they sit under. A new id that is followed to another row belongs in `HANDLER_CHECKED`.
const DAEMON_REPORTED_UNCHECKED: &[(&str, &str)] = &[
    ("CredentialAttempt", "credential_id"),
    ("CredentialRunOutcome", "hosts_with_guests_listed"),
    ("DiscoveryType", "host_id"),
    ("DiscoveryType", "target_host_ids"),
    ("DiscoveryWarning", "credential_id"),
    ("DiscoveryWarning", "host_id"),
    ("MalformedNeighbours", "credential_id"),
    ("ProvisionalSubnet", "seen_by_host_ids"),
    ("ProvisionalSubnet", "subnet_id"),
    ("UnconfirmedHost", "host_id"),
    ("UnmatchedNeighbour", "host_id"),
    ("UnresolvedPort", "host_id"),
    ("UnresolvedPort", "remote_host_id"),
];

/// Whether a property holds a UUID or an array of them.
fn holds_uuid(schema: &Value) -> bool {
    let names_uuid = |s: &Value| s.get("format").is_some_and(|f| f == "uuid");
    names_uuid(schema)
        || schema.get("items").is_some_and(names_uuid)
        || ["oneOf", "anyOf"].iter().any(|k| {
            schema
                .get(*k)
                .and_then(Value::as_array)
                .is_some_and(|parts| parts.iter().any(holds_uuid))
        })
}

/// Walks a request schema, collecting every id property nothing accounts for. `root` is the
/// nearest enclosing entity, whose references also cover the types nested in it.
fn unaccounted_ids(
    schemas: &Value,
    schema: &Value,
    name: &str,
    root: Option<EntityDiscriminants>,
    seen: &mut HashSet<(String, Option<EntityDiscriminants>)>,
    found: &mut std::collections::BTreeSet<String>,
) {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let target = reference.rsplit('/').next().unwrap();
        let root = EntityDiscriminants::iter()
            .find(|e| e.as_ref() == target)
            .or(root);
        if seen.insert((target.to_string(), root)) {
            unaccounted_ids(schemas, &schemas[target], target, root, seen, found);
        }
        return;
    }
    if let Some(props) = schema.get("properties").and_then(Value::as_object) {
        for (key, prop) in props {
            // The generic handlers check an entity's own site and organization.
            let accounted = SYSTEM_KEYS.contains(&key.as_str())
                || key == "site_id"
                || key == "organization_id"
                || root.is_some_and(|entity| {
                    entity.references().iter().any(|r| r.key == key)
                        || entity
                            .fields()
                            .iter()
                            .any(|f| f.id == key && matches!(f.write, FieldWrite::ServerOwned(_)))
                })
                || HANDLER_CHECKED
                    .iter()
                    .any(|(schema, field, _)| *schema == name && field == key)
                || DAEMON_REPORTED_UNCHECKED
                    .iter()
                    .any(|(schema, field)| *schema == name && field == key);
            if holds_uuid(prop) && !accounted {
                found.insert(format!("{name}.{key}"));
            }
            unaccounted_ids(schemas, prop, name, root, seen, found);
        }
    }
    if let Some(items) = schema.get("items") {
        unaccounted_ids(schemas, items, name, root, seen, found);
    }
    for key in ["allOf", "oneOf", "anyOf"] {
        for part in schema
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            unaccounted_ids(schemas, part, name, root, seen, found);
        }
    }
}

/// Every id a request body can carry is a registry reference, a server-owned field, or checked
/// by a named handler. A new id field fails here until someone decides which.
#[test]
fn every_id_in_a_request_body_is_accounted_for() {
    let spec = serde_json::to_value(crate::server::openapi::full_spec()).unwrap();
    let schemas = &spec["components"]["schemas"];
    let mut found = std::collections::BTreeSet::new();
    let mut seen = HashSet::new();
    for path in spec["paths"].as_object().unwrap().values() {
        for operation in path.as_object().unwrap().values() {
            let body = &operation["requestBody"]["content"]["application/json"]["schema"];
            if !body.is_null() {
                unaccounted_ids(schemas, body, "body", None, &mut seen, &mut found);
            }
        }
    }
    assert!(
        found.is_empty(),
        "ids with no reference, policy or handler check:\n{}",
        found.into_iter().collect::<Vec<_>>().join("\n")
    );
}

/// Field ids are unique, so a list-table column has one group, and no exported field repeats a
/// column CSV already leads with.
#[test]
fn field_ids_are_unique_and_never_shadow_a_system_column() {
    for entity in EntityDiscriminants::iter() {
        let mut seen = std::collections::HashSet::new();
        for field in entity.fields() {
            assert!(
                seen.insert(field.id),
                "{entity}: duplicate field `{}`",
                field.id
            );
            assert!(
                !field.csv || !CSV_SYSTEM_COLUMNS.contains(&field.id),
                "{entity}: `{}` is a system column",
                field.id
            );
        }
    }
}

#[tokio::test]
async fn csv_rows_carry_own_parent_child_and_embedded_values() {
    let mut host = example(EntityDiscriminants::Host).unwrap();
    let mut ip = example(EntityDiscriminants::IPAddress).unwrap();
    let mut other_ip = ip.clone();
    let mut port = example(EntityDiscriminants::Port).unwrap();
    let host_id = Uuid::new_v4();
    let ip_id = Uuid::new_v4();
    let other_ip_id = Uuid::new_v4();
    let port_id = Uuid::new_v4();
    host["id"] = json!(host_id);
    ip["id"] = json!(ip_id);
    ip["host_id"] = json!(host_id);
    ip["ip_address"] = json!("10.0.0.5");
    other_ip["id"] = json!(other_ip_id);
    other_ip["host_id"] = json!(host_id);
    other_ip["ip_address"] = json!("10.0.0.6");
    port["id"] = json!(port_id);
    port["host_id"] = json!(host_id);
    port["number"] = json!(443);
    port["protocol"] = json!("Tcp");
    let mut service = example(EntityDiscriminants::Service).unwrap();
    service["host_id"] = json!(host_id);
    service["bindings"] = json!([{ "type": "Port", "port_id": port_id, "ip_address_id": null }]);
    let mut binding = example(EntityDiscriminants::Binding).unwrap();
    binding["ip_address_id"] = json!(ip_id);
    binding["port_id"] = json!(port_id);

    let source = MemorySource(HashMap::from([
        (EntityDiscriminants::Host, vec![host.clone()]),
        (EntityDiscriminants::IPAddress, vec![ip.clone(), other_ip]),
        (EntityDiscriminants::Port, vec![port]),
        (EntityDiscriminants::Binding, vec![binding]),
    ]));

    let csv = build_entity_csv(&source, EntityDiscriminants::Service, &[service])
        .await
        .unwrap();
    let mut reader = csv::Reader::from_reader(csv.as_slice());
    let headers = reader.headers().unwrap().clone();
    let row = reader.records().next().unwrap().unwrap();
    let cell = |name: &str| row[headers.iter().position(|h| h == name).unwrap()].to_string();
    assert_eq!(cell("host"), host_id.to_string());
    assert_eq!(cell("port_bindings"), "443/tcp");

    let csv = build_entity_csv(&source, EntityDiscriminants::Host, &[host])
        .await
        .unwrap();
    let mut reader = csv::Reader::from_reader(csv.as_slice());
    let headers = reader.headers().unwrap().clone();
    let row = reader.records().next().unwrap().unwrap();
    let ips = &row[headers.iter().position(|h| h == "interface_ip").unwrap()];
    let mut ips: Vec<String> = ips.split("; ").map(str::to_string).collect();
    ips.sort();
    let mut expected = vec![ip_id.to_string(), other_ip_id.to_string()];
    expected.sort();
    assert_eq!(ips, expected);
    assert_eq!(
        &row[headers.iter().position(|h| h == "ports").unwrap()],
        port_id.to_string()
    );
    assert_eq!(
        headers
            .iter()
            .take(CSV_SYSTEM_COLUMNS.len())
            .collect::<Vec<_>>(),
        CSV_SYSTEM_COLUMNS
    );

    // Through a child row to its parent: the id of the row the path ends on.
    let csv = build_entity_csv(&source, EntityDiscriminants::IPAddress, &[ip])
        .await
        .unwrap();
    let mut reader = csv::Reader::from_reader(csv.as_slice());
    let headers = reader.headers().unwrap().clone();
    let row = reader.records().next().unwrap().unwrap();
    assert_eq!(
        &row[headers.iter().position(|h| h == "ports").unwrap()],
        port_id.to_string()
    );
}
