//! The Issues list as CSV: one row per Issue, with the labels the Issues table shows and the
//! entities it names read through the entity CSV's field resolver.

use std::collections::BTreeSet;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use super::types::{OpenSetupItem, SetupItem, SetupItemDiscriminants, SetupListEntry};
use crate::{
    daemon::discovery::types::warnings::DiscoveryWarningCode,
    server::{
        credentials::r#impl::types::CredentialIntegration,
        daemons::r#impl::api::DaemonHealth,
        shared::{
            entities::EntityDiscriminants,
            services::entity_fields::{RelatedSource, build_csv},
            types::{
                entity_field::{EntityField, FieldFormat, FieldGroup, Hop},
                metadata::TypeMetadataProvider,
            },
        },
    },
};

/// The Issue CSV's columns, read off each [`IssueCsvRow`]. The entity columns name each entity
/// the Issue is about, one per value, read as the exporting user, so an entity they can't read
/// drops out.
pub const ISSUE_CSV_FIELDS: &[EntityField] = &[
    EntityField::new("issue", "Issue", FieldGroup::Identity),
    EntityField::new("issue_type", "Issue type", FieldGroup::Identity),
    EntityField::new("category", "Category", FieldGroup::Identity),
    EntityField::new("health", "Health", FieldGroup::Status),
    EntityField::new("integration", "Integration", FieldGroup::Identity),
    EntityField::new("warning_code", "Warning code", FieldGroup::Status),
    EntityField::new("daemons", "Daemons", FieldGroup::Relationships)
        .via(
            &[Hop::Parent {
                entity: EntityDiscriminants::Daemon,
                key: "daemon_ids",
            }],
            Some("name"),
        )
        .many(),
    EntityField::new("credentials", "Credentials", FieldGroup::Relationships)
        .via(
            &[Hop::Parent {
                entity: EntityDiscriminants::Credential,
                key: "credential_ids",
            }],
            Some("name"),
        )
        .many(),
    EntityField::new("hosts", "Hosts", FieldGroup::Relationships)
        .via(
            &[Hop::Parent {
                entity: EntityDiscriminants::Host,
                key: "host_ids",
            }],
            Some("name"),
        )
        .many(),
    EntityField::new("sites", "Sites", FieldGroup::Relationships)
        .via(
            &[Hop::Parent {
                entity: EntityDiscriminants::Site,
                key: "site_ids",
            }],
            Some("name"),
        )
        .many(),
    EntityField::new("opened_at", "Opened", FieldGroup::Status).format(FieldFormat::Timestamp),
];

/// One Issue as its CSV row reads it: labels from the same metadata the Issues table shows, the
/// wire values of the Issue's enums, and the ids of the entities it names.
#[derive(Serialize, Debug, PartialEq)]
pub struct IssueCsvRow {
    issue: &'static str,
    issue_type: SetupItemDiscriminants,
    category: &'static str,
    health: Option<DaemonHealth>,
    integration: Option<CredentialIntegration>,
    warning_code: Option<DiscoveryWarningCode>,
    daemon_ids: Vec<Uuid>,
    credential_ids: Vec<Uuid>,
    host_ids: Vec<Uuid>,
    site_ids: Vec<Uuid>,
    /// When the sweep first saw the Issue open. `None` until it has.
    opened_at: Option<DateTime<Utc>>,
}

impl IssueCsvRow {
    pub fn new(entry: &SetupListEntry, open: &[OpenSetupItem]) -> Self {
        let item = &entry.item;
        let issue = match item {
            SetupItem::RunWarnings { code, .. } => code.name(),
            _ => SetupItemDiscriminants::from(item).name(),
        };
        let mut daemon_ids = Vec::new();
        let mut credential_ids = BTreeSet::new();
        let mut host_ids = BTreeSet::new();
        let (mut health, mut integration, mut warning_code) = (None, None, None);
        match item {
            SetupItem::DaemonHealth {
                daemon_id,
                health: status,
                ..
            } => {
                daemon_ids.push(*daemon_id);
                health = Some(*status);
            }
            SetupItem::UncoveredIntegration {
                host_ids: ids,
                integration: of,
                ..
            } => {
                host_ids.extend(ids.iter().copied());
                integration = Some(*of);
            }
            SetupItem::TargetlessCredential {
                credential_id,
                integration: of,
                ..
            } => {
                credential_ids.insert(*credential_id);
                integration = Some(*of);
            }
            SetupItem::RunWarnings {
                daemon_id,
                code,
                warnings,
                ..
            } => {
                daemon_ids.push(*daemon_id);
                warning_code = Some(*code);
                for warning in warnings {
                    let mut warning = warning.clone();
                    let slots = warning.entity_id_slots();
                    credential_ids.extend(slots.credential_id.and_then(|id| *id));
                    host_ids.extend(slots.host_id.and_then(|id| *id));
                    host_ids.extend(slots.remote_host_id.and_then(|id| *id));
                }
            }
            _ => {}
        }
        Self {
            issue,
            issue_type: SetupItemDiscriminants::from(item),
            category: item.issue_category().map(|c| c.name()).unwrap_or_default(),
            health,
            integration,
            warning_code,
            daemon_ids,
            credential_ids: credential_ids.into_iter().collect(),
            host_ids: host_ids.into_iter().collect(),
            site_ids: entry.site_ids.clone(),
            opened_at: open
                .iter()
                .find(|o| o.key == entry.key)
                .map(|o| o.open_since),
        }
    }
}

/// CSV bytes for `entries`, in their order. `open` is the org's sweep record of open items.
pub async fn issues_csv<S: RelatedSource>(
    source: &S,
    entries: &[SetupListEntry],
    open: &[OpenSetupItem],
) -> Result<Vec<u8>> {
    let roots = entries
        .iter()
        .map(|entry| serde_json::to_value(IssueCsvRow::new(entry, open)))
        .collect::<Result<Vec<_>, _>>()?;
    build_csv(source, &[], ISSUE_CSV_FIELDS, &roots).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::setup::types::SetupItemState;
    use async_trait::async_trait;
    use serde_json::{Value, json};
    use std::collections::HashMap;

    /// The entities a reader may read, by type: anything missing is unreadable.
    struct Readable(HashMap<EntityDiscriminants, Vec<Value>>);

    #[async_trait]
    impl RelatedSource for Readable {
        async fn by_column(
            &self,
            entity: EntityDiscriminants,
            _column: &'static str,
            ids: &[Uuid],
        ) -> Result<Vec<Value>> {
            Ok(self
                .0
                .get(&entity)
                .into_iter()
                .flatten()
                .filter(|row| {
                    row["id"]
                        .as_str()
                        .and_then(|id| Uuid::parse_str(id).ok())
                        .is_some_and(|id| ids.contains(&id))
                })
                .cloned()
                .collect())
        }
    }

    fn named(id: Uuid, name: &str) -> Value {
        json!({ "id": id, "name": name })
    }

    /// Each Issue is one row naming its entities, one per value; an entity the reader can't read
    /// drops out, and an Issue the sweep hasn't seen has no opened time.
    #[tokio::test]
    async fn each_issue_names_the_entities_its_reader_can_read() {
        let (site, daemon, seen_host, hidden_host) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let unhealthy = SetupListEntry::new(
            SetupItem::DaemonHealth {
                daemon_id: daemon,
                name: "Edge daemon".into(),
                site_id: site,
                health: DaemonHealth::Unreachable,
            },
            SetupItemState::Open,
        );
        let uncovered = SetupListEntry::new(
            SetupItem::UncoveredIntegration {
                integration: CredentialIntegration::Snmp,
                host_ids: vec![seen_host, hidden_host],
                site_ids: vec![site],
            },
            SetupItemState::Open,
        );
        let opened = crate::server::shared::clock::now();
        let open = [OpenSetupItem {
            key: unhealthy.key.clone(),
            open_since: opened,
        }];
        let source = Readable(HashMap::from([
            (EntityDiscriminants::Site, vec![named(site, "Office")]),
            (
                EntityDiscriminants::Daemon,
                vec![named(daemon, "Edge daemon")],
            ),
            (EntityDiscriminants::Host, vec![named(seen_host, "printer")]),
        ]));

        let bytes = issues_csv(&source, &[unhealthy, uncovered], &open)
            .await
            .unwrap();
        let mut reader = csv::Reader::from_reader(bytes.as_slice());
        let headers = reader.headers().unwrap().clone();
        let rows: Vec<HashMap<String, String>> = reader
            .records()
            .map(|record| {
                headers
                    .iter()
                    .zip(record.unwrap().iter())
                    .map(|(h, v)| (h.to_string(), v.to_string()))
                    .collect()
            })
            .collect();

        // A cell holds the enum's wire value, as the API sends it.
        let wire = |value: Value| value.as_str().unwrap().to_string();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["daemons"], "Edge daemon");
        assert_eq!(rows[0]["sites"], "Office");
        assert!(!rows[0]["opened_at"].is_empty());
        assert_eq!(
            rows[0]["issue_type"],
            wire(json!(SetupItemDiscriminants::DaemonHealth))
        );
        assert_eq!(rows[0]["health"], wire(json!(DaemonHealth::Unreachable)));
        assert_eq!(rows[0]["integration"], "");
        assert_eq!(rows[1]["hosts"], "printer");
        assert_eq!(rows[1]["daemons"], "");
        assert_eq!(rows[1]["opened_at"], "");
        assert_eq!(
            rows[1]["integration"],
            wire(json!(CredentialIntegration::Snmp))
        );
        assert_eq!(rows[1]["health"], "");
        assert_eq!(rows[1]["warning_code"], "");
    }
}
