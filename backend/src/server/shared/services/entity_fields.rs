//! Resolves the field registry (`shared/entity_fields.rs`) against serialized entities, following
//! relation hops through each related entity's own service.

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::shared::{
    entities::{ChangeTriggersTopologyStaleness, Entity as EntityEnum, EntityDiscriminants},
    entity_fields::EntityReference,
    services::{factory::ServiceFactory, traits::CrudService},
    storage::{filter::StorableFilter, traits::Entity},
    types::{
        api::ApiError,
        entity_field::{EntityField, FieldFormat, Hop},
    },
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

/// Related entities read through their services, scoped to what `reader` may read of each type
/// and, for SCD2 entities, to live rows.
pub struct ServiceSource<'a> {
    pub services: &'a ServiceFactory,
    pub reader: &'a AuthenticatedEntity,
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
        let Some(site_ids) = self.reader.readable_site_ids(T::ENTITY_TYPE.resource()) else {
            return Ok(Vec::new());
        };
        let mut filter = StorableFilter::<T>::new_for_access(&site_ids, &self.organization_id)
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

/// The ids `reference` holds on `object`.
fn reference_ids(reference: &EntityReference, object: &Value) -> Vec<Uuid> {
    let mut holders = vec![object.clone()];
    for key in &reference.embedded {
        holders = holders.iter().flat_map(|o| values_at(o, key)).collect();
    }
    holders
        .iter()
        .flat_map(|o| ids_at(o, reference.key))
        .collect()
}

/// Refuses a request that names something its caller can't use: every id in `incoming` that the
/// registry declares a reference must be a row the caller can reach, on the entity's own site when
/// both have one. A missing id and another tenant's are refused alike.
///
/// `stored` is the row being updated. Ids already on it pass, so an edit to another field can't
/// fail on what the row already holds. `own_ids` are rows the same request creates, which no read
/// can find yet. Each target is read as `caller`, so it needs `Read` on the target's resource.
/// Sites themselves are checked across the organization: a key limited to one site can still save
/// a user who holds others.
pub async fn validate_references(
    services: &ServiceFactory,
    entity: EntityDiscriminants,
    incoming: &Value,
    stored: Option<&Value>,
    own_ids: &[Uuid],
    caller: &AuthenticatedEntity,
    organization_id: Uuid,
) -> Result<(), ApiError> {
    let entity_site = incoming.get("site_id").and_then(uuid_of);
    let mut organization_sites: Option<Vec<Uuid>> = None;
    let source = ServiceSource {
        services,
        reader: caller,
        organization_id,
    };

    for reference in entity.references() {
        let held = stored
            .map(|row| reference_ids(&reference, row))
            .unwrap_or_default();
        let mut ids = reference_ids(&reference, incoming);
        ids.retain(|id| !held.contains(id) && !own_ids.contains(id));
        ids.sort();
        ids.dedup();
        if ids.is_empty() {
            continue;
        }

        if reference.target == EntityDiscriminants::Site {
            if organization_sites.is_none() {
                let sites = services
                    .site_service
                    .get_all(StorableFilter::new_from_org_id(&organization_id))
                    .await
                    .map_err(ApiError::from)?;
                organization_sites = Some(sites.iter().map(|s| s.id).collect());
            }
            let sites = organization_sites.as_deref().unwrap_or_default();
            if !ids.iter().all(|id| sites.contains(id)) {
                return Err(ApiError::entity_reference_refused(reference.key));
            }
            continue;
        }

        let rows = source
            .by_column(reference.target, "id", &ids)
            .await
            .map_err(ApiError::from)?;
        let usable = |id: &Uuid| {
            rows.iter().any(|row| {
                row.get("id").and_then(uuid_of) == Some(*id)
                    && match (entity_site, row.get("site_id").and_then(uuid_of)) {
                        (Some(own), Some(theirs)) => own == theirs,
                        _ => true,
                    }
            })
        };
        if !ids.iter().all(usable) {
            return Err(ApiError::entity_reference_refused(reference.key));
        }
    }
    Ok(())
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
    let fields: Vec<EntityField> = entity
        .fields()
        .iter()
        .filter(|f| f.csv)
        .map(|f| f.for_csv())
        .collect();
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
mod tests;
