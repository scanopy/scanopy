//! Resolves the field registry (`shared/entity_fields.rs`) against serialized entities, following
//! relation hops through each related entity's own service.

use crate::server::shared::{
    entities::{ChangeTriggersTopologyStaleness, Entity as EntityEnum, EntityDiscriminants},
    services::{factory::ServiceFactory, traits::CrudService},
    storage::{filter::StorableFilter, traits::Entity},
    types::entity_field::{EntityField, FieldFormat, Hop},
};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Display;
use uuid::Uuid;

/// Where related entities come from: every entity of `entity` whose `column` holds one of `ids`.
#[async_trait]
pub trait RelatedSource: Sync {
    async fn by_column(
        &self,
        entity: EntityDiscriminants,
        column: &'static str,
        ids: &[Uuid],
    ) -> Result<Vec<Value>>;
}

/// Related entities read through their services, scoped to what the caller can see and, for
/// SCD2 entities, to live rows.
pub struct ServiceSource<'a> {
    pub services: &'a ServiceFactory,
    pub site_ids: &'a [Uuid],
    pub organization_id: Uuid,
}

impl ServiceSource<'_> {
    async fn fetch<T, S>(
        &self,
        service: &S,
        column: &'static str,
        ids: &[Uuid],
    ) -> Result<Vec<Value>>
    where
        T: Entity
            + Into<EntityEnum>
            + Default
            + Display
            + ChangeTriggersTopologyStaleness<T>
            + serde::Serialize,
        S: CrudService<T> + Sync,
    {
        let mut filter = StorableFilter::<T>::new_for_access(self.site_ids, &self.organization_id)
            .uuids_column(column, ids);
        if T::HAS_SCD2 {
            filter = filter.live();
        }
        let items = service.get_all(filter).await?;
        items
            .iter()
            .map(|item| serde_json::to_value(item).map_err(Into::into))
            .collect()
    }
}

#[async_trait]
impl RelatedSource for ServiceSource<'_> {
    async fn by_column(
        &self,
        entity: EntityDiscriminants,
        column: &'static str,
        ids: &[Uuid],
    ) -> Result<Vec<Value>> {
        use EntityDiscriminants as E;
        let s = self.services;
        match entity {
            E::Site => self.fetch(&*s.site_service, column, ids).await,
            E::Vlan => self.fetch(&*s.vlan_service, column, ids).await,
            E::Subnet => self.fetch(&*s.subnet_service, column, ids).await,
            E::Host => self.fetch(&*s.host_service, column, ids).await,
            E::Service => self.fetch(&*s.service_service, column, ids).await,
            E::IPAddress => self.fetch(&*s.ip_address_service, column, ids).await,
            E::Interface => self.fetch(&*s.interface_service, column, ids).await,
            E::Port => self.fetch(&*s.port_service, column, ids).await,
            E::Binding => self.fetch(&*s.binding_service, column, ids).await,
            E::Discovery => self.fetch(&*s.discovery_service, column, ids).await,
            E::Daemon => self.fetch(&*s.daemon_service, column, ids).await,
            E::DaemonApiKey => self.fetch(&*s.daemon_api_key_service, column, ids).await,
            E::Tag => self.fetch(&*s.tag_service, column, ids).await,
            E::TagGroup => self.fetch(&*s.tag_group_service, column, ids).await,
            E::User => self.fetch(&*s.user_service, column, ids).await,
            E::UserApiKey => self.fetch(&*s.user_api_key_service, column, ids).await,
            E::Credential => self.fetch(&*s.credential_service, column, ids).await,
            E::Organization => self.fetch(&*s.organization_service, column, ids).await,
            E::Invite => self.fetch(&*s.invite_service, column, ids).await,
            E::Topology => self.fetch(&*s.topology_service, column, ids).await,
            E::Snapshot => self.fetch(&*s.snapshot_service, column, ids).await,
            E::Share => self.fetch(&*s.share_service, column, ids).await,
            E::Dependency => self.fetch(&*s.dependency_service, column, ids).await,
        }
    }
}

fn uuid_of(value: &Value) -> Option<Uuid> {
    value.as_str().and_then(|s| Uuid::parse_str(s).ok())
}

/// The ids held under `key`: one id, or an array of them.
fn ids_at(object: &Value, key: &str) -> Vec<Uuid> {
    match object.get(key) {
        Some(Value::Array(items)) => items.iter().filter_map(uuid_of).collect(),
        Some(value) => uuid_of(value).into_iter().collect(),
        None => vec![],
    }
}

/// The value at a dotted path, flattening arrays met along the way.
fn values_at(object: &Value, path: &str) -> Vec<Value> {
    let mut current = vec![object.clone()];
    for segment in path.split('.') {
        current = current
            .into_iter()
            .filter_map(|v| v.get(segment).cloned())
            .flat_map(|v| match v {
                Value::Array(items) => items,
                other => vec![other],
            })
            .filter(|v| !v.is_null())
            .collect();
    }
    current
}

/// Resolves fields for a batch of root objects, reading each related set once.
pub struct FieldResolver<'a, S: RelatedSource> {
    source: &'a S,
    /// Fetched entities by (type, column), grouped by the column's value.
    cache: HashMap<(EntityDiscriminants, &'static str), HashMap<Uuid, Vec<Value>>>,
}

impl<'a, S: RelatedSource> FieldResolver<'a, S> {
    pub fn new(source: &'a S) -> Self {
        FieldResolver {
            source,
            cache: HashMap::new(),
        }
    }

    async fn related(
        &mut self,
        entity: EntityDiscriminants,
        column: &'static str,
        ids: Vec<Uuid>,
    ) -> Result<&HashMap<Uuid, Vec<Value>>> {
        let known = self.cache.entry((entity, column)).or_default();
        let missing: Vec<Uuid> = ids
            .into_iter()
            .filter(|id| !known.contains_key(id))
            .collect();
        if !missing.is_empty() {
            let fetched = self.source.by_column(entity, column, &missing).await?;
            let known = self.cache.entry((entity, column)).or_default();
            for id in &missing {
                known.entry(*id).or_default();
            }
            for item in fetched {
                for key in ids_at(&item, column) {
                    known.entry(key).or_default().push(item.clone());
                }
            }
        }
        Ok(&self.cache[&(entity, column)])
    }

    async fn follow(&mut self, hop: &Hop, rows: Vec<Vec<Value>>) -> Result<Vec<Vec<Value>>> {
        match *hop {
            Hop::Embedded { key } => Ok(rows
                .into_iter()
                .map(|objects| objects.iter().flat_map(|o| values_at(o, key)).collect())
                .collect()),
            Hop::Parent { entity, key } => {
                let ids: Vec<Uuid> = rows.iter().flatten().flat_map(|o| ids_at(o, key)).collect();
                let by_id = self.related(entity, "id", ids).await?;
                Ok(rows
                    .iter()
                    .map(|objects| {
                        objects
                            .iter()
                            .flat_map(|o| ids_at(o, key))
                            .flat_map(|id| by_id.get(&id).cloned().unwrap_or_default())
                            .collect()
                    })
                    .collect())
            }
            Hop::Children { entity, key } => {
                let ids: Vec<Uuid> = rows
                    .iter()
                    .flatten()
                    .flat_map(|o| ids_at(o, "id"))
                    .collect();
                let by_parent = self.related(entity, key, ids).await?;
                Ok(rows
                    .iter()
                    .map(|objects| {
                        objects
                            .iter()
                            .flat_map(|o| ids_at(o, "id"))
                            .flat_map(|id| by_parent.get(&id).cloned().unwrap_or_default())
                            .collect()
                    })
                    .collect())
            }
        }
    }

    /// The values of `field` for each root, in root order.
    pub async fn resolve(
        &mut self,
        field: &EntityField,
        roots: &[Value],
    ) -> Result<Vec<Vec<Value>>> {
        let mut rows: Vec<Vec<Value>> = roots.iter().map(|r| vec![r.clone()]).collect();
        for hop in field.path {
            rows = self.follow(hop, rows).await?;
        }
        Ok(match field.field {
            None => rows,
            Some(path) => rows
                .into_iter()
                .map(|objects| objects.iter().flat_map(|o| values_at(o, path)).collect())
                .collect(),
        })
    }
}

fn scalar_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// CSV text for one value of a field: machine-readable, so ids for entities and raw scalars.
pub fn csv_text(format: FieldFormat, value: &Value) -> String {
    match format {
        FieldFormat::Entity => value.get("id").map(scalar_text).unwrap_or_default(),
        FieldFormat::Port => match (value.get("number"), value.get("protocol")) {
            (Some(number), Some(protocol)) => {
                format!(
                    "{}/{}",
                    scalar_text(number),
                    scalar_text(protocol).to_lowercase()
                )
            }
            _ => scalar_text(value),
        },
        FieldFormat::OperatingSystem => {
            let part = |key: &str| value.get(key).map(scalar_text).filter(|s| !s.is_empty());
            [part("name").or_else(|| part("family")), part("version")]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ")
        }
        FieldFormat::Text
        | FieldFormat::Prose
        | FieldFormat::Number
        | FieldFormat::Boolean
        | FieldFormat::Timestamp
        | FieldFormat::Identifier
        | FieldFormat::BitsPerSecond
        | FieldFormat::Metadata { .. }
        | FieldFormat::AttributeSource => scalar_text(value),
    }
}

/// Columns every CSV leads with, ahead of the entity's registry fields.
pub const CSV_SYSTEM_COLUMNS: &[&str] = &["id", "created_at", "updated_at"];

/// CSV bytes for `roots` of `entity`: the system columns, then one column per exported registry
/// field. A field with several values (children, arrays) joins them with `; `.
pub async fn build_entity_csv<S: RelatedSource>(
    source: &S,
    entity: EntityDiscriminants,
    roots: &[Value],
) -> Result<Vec<u8>> {
    let fields: Vec<&EntityField> = entity.fields().iter().filter(|f| f.csv).collect();
    let mut resolver = FieldResolver::new(source);
    let mut columns = Vec::with_capacity(fields.len());
    for field in &fields {
        let values = resolver.resolve(field, roots).await?;
        columns.push(
            values
                .iter()
                .map(|vs| {
                    vs.iter()
                        .map(|v| csv_text(field.format, v))
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                .collect::<Vec<_>>(),
        );
    }

    let mut writer = csv::Writer::from_writer(vec![]);
    writer.write_record(
        CSV_SYSTEM_COLUMNS
            .iter()
            .copied()
            .chain(fields.iter().map(|f| f.id)),
    )?;
    for (row, root) in roots.iter().enumerate() {
        let system = CSV_SYSTEM_COLUMNS
            .iter()
            .map(|c| root.get(*c).map(scalar_text).unwrap_or_default());
        let values = columns.iter().map(|column| column[row].clone());
        writer.write_record(system.chain(values))?;
    }
    writer
        .into_inner()
        .map_err(|e| anyhow::anyhow!(e.into_error()))
}

/// [`build_entity_csv`] for typed entities.
pub async fn entities_csv<S: RelatedSource, T: serde::Serialize>(
    source: &S,
    entity: EntityDiscriminants,
    items: &[T],
) -> Result<Vec<u8>> {
    let roots = items
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;
    build_entity_csv(source, entity, &roots).await
}

#[cfg(test)]
mod tests {
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
                        let schema = schema.unwrap_or_else(|| {
                            panic!("{root}.{}: no key `{key}` on {on}", field.id)
                        });
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
        let port_id = Uuid::new_v4();
        host["id"] = json!(host_id);
        ip["host_id"] = json!(host_id);
        ip["ip_address"] = json!("10.0.0.5");
        other_ip["id"] = json!(Uuid::new_v4());
        other_ip["host_id"] = json!(host_id);
        other_ip["ip_address"] = json!("10.0.0.6");
        port["id"] = json!(port_id);
        port["number"] = json!(443);
        port["protocol"] = json!("Tcp");
        let mut service = example(EntityDiscriminants::Service).unwrap();
        service["host_id"] = json!(host_id);
        service["bindings"] =
            json!([{ "type": "Port", "port_id": port_id, "ip_address_id": null }]);

        let source = MemorySource(HashMap::from([
            (EntityDiscriminants::Host, vec![host.clone()]),
            (EntityDiscriminants::IPAddress, vec![ip, other_ip]),
            (EntityDiscriminants::Port, vec![port]),
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
        let mut ips: Vec<&str> = ips.split("; ").collect();
        ips.sort();
        assert_eq!(ips, ["10.0.0.5", "10.0.0.6"]);
        assert_eq!(
            headers
                .iter()
                .take(CSV_SYSTEM_COLUMNS.len())
                .collect::<Vec<_>>(),
            CSV_SYSTEM_COLUMNS
        );
    }
}
