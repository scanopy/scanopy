use crate::server::credentials::r#impl::types::CredentialAssignment;
use crate::server::hosts::r#impl::attributes::{
    HostChassisIdAttributed, HostFirmwareRevisionAttributed, HostHostnameAttributed,
    HostManagementUrlAttributed, HostManufacturerAttributed, HostModelAttributed,
    HostSerialNumberAttributed, HostSoftwareRevisionAttributed, HostSysContactAttributed,
    HostSysDescrAttributed, HostSysLocationAttributed, HostSysNameAttributed,
    HostSysObjectIdAttributed,
};
use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
use crate::server::hosts::r#impl::virtualization::HostVirtualization;
use crate::server::shared::attribution::{self, AttributeSource, Attributed};
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::types::api::deserialize_empty_string_as_none;
use crate::server::shared::types::entities::EntitySource;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// The 100-character cap the API has always enforced on a host name. A custom validator rather
/// than `#[validate(length)]` because the derive cannot see a length through [`HostName`].
fn validate_host_name(name: &HostName) -> Result<(), validator::ValidationError> {
    if name.value().as_str().chars().count() > 100 {
        return Err(validator::ValidationError::new("length"));
    }
    Ok(())
}

/// Base data for a Host entity (stored in database).
/// Child entities (ip_addresses, ports, services) are stored in their own tables
/// and queried by `host_id`. They are NOT stored on the host.
#[derive(Debug, Clone, Serialize, Validate, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct HostBase {
    /// The host's name, together with the rung of the naming ladder that produced it.
    ///
    /// Serialises as the two flat keys `name` and `name_source`, so the wire format is a bare
    /// string exactly as it has always been. Assign only through [`HostBase::apply_name`].
    #[serde(
        flatten,
        deserialize_with = "crate::server::hosts::r#impl::name::deserialize_host_name"
    )]
    #[schema(value_type = HostName)]
    #[validate(custom(function = "validate_host_name"))]
    pub name: HostName,
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// The host's hostname, with the source that produced it.
    ///
    /// An identifier rather than a name, so it is never copied into `name`: see the placement rule
    /// in [`super::name`]. Two flat wire keys, `hostname` and `hostname_source`, so a daemon that
    /// sends only `hostname` still deserialises, at `Unspecified`.
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostHostnameAttributed)]
    pub hostname: Option<HostHostnameAttributed>,
    /// Free-text notes about the host.
    #[validate(length(min = 0, max = 500))]
    #[serde(deserialize_with = "deserialize_empty_string_as_none")]
    #[schema(required)]
    pub description: Option<String>,
    /// How this host came to be known — discovered, imported, or created by hand.
    #[schema(read_only)]
    pub source: EntitySource,
    /// How the host is virtualized, when it is a VM or container guest.
    #[schema(required)]
    pub virtualization_metadata: Option<HostVirtualization>,
    /// The service doing the virtualizing — the hypervisor this VM runs on.
    ///
    /// Its own column with a foreign key rather than a field inside
    /// [`HostVirtualization`]: a reference that no longer resolves now fails the write instead of
    /// surviving as a value nothing matches, and `ON DELETE SET NULL` clears it when the
    /// hypervisor service goes away (GH #650).
    #[schema(required)]
    pub virtualization_service_id: Option<Uuid>,
    /// Whether the host is hidden from topology views.
    pub hidden: bool,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    // Discovered attributes. Each carries the source that produced it, as two flat wire keys —
    // `model` and `model_source` — so the payload shape a daemon sends is unchanged except for the
    // rung riding alongside. Assign only through `Attributed::apply`.
    /// SNMP sysDescr.0 - full system description
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostSysDescrAttributed)]
    pub sys_descr: Option<HostSysDescrAttributed>,
    /// SNMP sysObjectID.0 - vendor OID for device identification
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostSysObjectIdAttributed)]
    pub sys_object_id: Option<HostSysObjectIdAttributed>,
    /// SNMP sysLocation.0 - physical location
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostSysLocationAttributed)]
    pub sys_location: Option<HostSysLocationAttributed>,
    /// SNMP sysContact.0 - admin contact info
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostSysContactAttributed)]
    pub sys_contact: Option<HostSysContactAttributed>,
    /// URL for device management interface (manual or discovered)
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostManagementUrlAttributed)]
    pub management_url: Option<HostManagementUrlAttributed>,
    /// LLDP lldpLocChassisId - globally unique device identifier for deduplication
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostChassisIdAttributed)]
    pub chassis_id: Option<HostChassisIdAttributed>,
    /// SNMP sysName.0 - administratively-assigned hostname
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostSysNameAttributed)]
    pub sys_name: Option<HostSysNameAttributed>,
    /// ENTITY-MIB entPhysicalMfgName - hardware manufacturer
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostManufacturerAttributed)]
    pub manufacturer: Option<HostManufacturerAttributed>,
    /// ENTITY-MIB entPhysicalModelName - hardware model
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostModelAttributed)]
    pub model: Option<HostModelAttributed>,
    /// ENTITY-MIB entPhysicalSerialNum - hardware serial number
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostSerialNumberAttributed)]
    pub serial_number: Option<HostSerialNumberAttributed>,
    /// Firmware revision of the device as a whole — ENTITY-MIB `entPhysicalFirmwareRev`.
    ///
    /// Written by whichever source read it — a controller's REST inventory, an industrial probe's
    /// identity response, and `entPhysicalFirmwareRev`. Before this column existed each of those
    /// had nowhere to put a version it had already read, and two of them flattened it into
    /// `sys_descr` as prose.
    ///
    /// Device-level rather than per-module: everything downstream is host-shaped, and the NCCoE
    /// asset-inventory minimum says "product version", singular.
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostFirmwareRevisionAttributed)]
    pub firmware_revision: Option<HostFirmwareRevisionAttributed>,
    /// Software revision of the device as a whole — ENTITY-MIB `entPhysicalSoftwareRev`.
    ///
    /// Its own field rather than sharing `firmware_revision`, because RFC 4133 defines `.9` and
    /// `.10` as distinct objects: on a Cisco chassis they are the bootloader and the IOS version,
    /// and one column holding either with nothing recording which cannot tell them apart. Only
    /// ENTITY-MIB writes it — every other source reports a single version, which is the firmware.
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = HostSoftwareRevisionAttributed)]
    pub software_revision: Option<HostSoftwareRevisionAttributed>,
    /// Credential assignments for this host (hydrated from junction table).
    #[serde(default)]
    #[schema(required)]
    pub credential_assignments: Vec<CredentialAssignment>,
}

impl Default for HostBase {
    fn default() -> Self {
        Self {
            name: HostName::unnamed(),
            network_id: Uuid::nil(),
            hostname: None,
            description: None,
            source: EntitySource::Unknown,
            virtualization_metadata: None,
            virtualization_service_id: None,
            hidden: false,
            tags: Vec::new(),
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            firmware_revision: None,
            software_revision: None,
            credential_assignments: Vec::new(),
        }
    }
}

/// The primary-address subquery the last rung of
/// [`display_name_sql`](crate::server::hosts::r#impl::name_ladder::display_name_sql) resolves
/// through, joined against whichever alias holds the `hosts` row.
///
/// `last_seen_at DESC, position ASC` over live rows is what makes the SQL faithful: it is the
/// choice [`Host::name_ladder`] makes for its address rung, so a host whose lease moved is sorted
/// under the address it is titled by, not the stale one it still holds.
macro_rules! host_primary_address_join {
    ($hosts:literal) => {
        concat!(
            "LEFT JOIN (\
                SELECT DISTINCT ON (host_id) host_id, ip_address \
                FROM ip_addresses \
                WHERE valid_to IS NULL \
                ORDER BY host_id, last_seen_at DESC, position ASC\
            ) AS primary_interface ON ",
            $hosts,
            ".id = primary_interface.host_id"
        )
    };
}
pub(crate) use host_primary_address_join;

/// The join for a query whose `hosts` row is the `hosts` table itself.
///
/// A shared `const` so the two order fields that need it cannot drift into two subqueries that
/// claim the same `primary_interface` alias while selecting different rows. `apply_ordering`
/// compares the two `join_sql()` results for equality before adding the second, so a copy-pasted
/// duplicate would still dedupe today — but only for exactly as long as the copies stay identical.
pub const PRIMARY_INTERFACE_JOIN: &str = host_primary_address_join!("hosts");

impl HostBase {
    /// Assign the host's name if `candidate` is at least as authoritative as what is stored.
    /// Returns whether anything changed.
    ///
    /// **This and [`Self::clear_name`] are the only places the name and its source are written.**
    /// The ordering lives entirely in [`AttributeSource::rank`], so there is no per-call-site
    /// precedence to keep in sync — a caller only has to say where its name came from.
    ///
    /// Equal rank from the same source wins, which is what makes a re-sync idempotent in the useful
    /// direction: a controller rename propagates on the next discovery, while a lower rung (reverse
    /// DNS, a detected service, an address) never displaces it, and nothing displaces
    /// [`AttributeSource::Manual`].
    pub fn apply_name(&mut self, candidate: HostName) -> bool {
        self.name.apply_in_place(candidate)
    }

    /// Drop the stored name. Returns whether anything changed.
    ///
    /// A person clearing the name field. The host then displays the next rung of
    /// [`Host::name_ladder`], and the next discovery can name it again, because a blank incumbent
    /// never blocks [`Self::apply_name`].
    ///
    /// Separate from `apply_name` because that applier reads a blank candidate as a scan that
    /// learned no name, and keeps what is stored. Here the blank is the instruction.
    pub fn clear_name(&mut self) -> bool {
        if self.name.is_blank() {
            return false;
        }
        self.name = HostName::unnamed();
        true
    }

    /// Merge every discovered attribute from `incoming`, returning whether anything changed.
    ///
    /// Rank-based, not first-write-wins. Before provenance this was an `is_none()` gate, so
    /// whichever source reached a field first owned it permanently and a better reading could never
    /// land — on a switch answering both SNMP and EtherNet/IP, precedence was decided by which
    /// probe happened to finish first. What protects a value a person entered is now
    /// [`AttributeSource::Manual`] outranking everything discovery can write, rather than the
    /// accident of having got there first.
    ///
    /// **The destructure below is the point of this method.** These arms used to be written out
    /// one per field at the single call site, and a field added to `HostBase` without one compiled
    /// perfectly and then silently dropped that field on every re-scan — collected, stored once,
    /// and never refreshed. Here a new field fails to compile until it is classified: either it is
    /// a discovered attribute and gets merged, or it is named in the ignore list because something
    /// else owns it (`name` has its own entry point above, `tags` and `credential_assignments` are
    /// user state, `hidden` is a user preference).
    pub fn apply_attributes_from(&mut self, incoming: &HostBase) -> bool {
        let HostBase {
            // Not attributes: owned by the naming ladder, by the user, or by the row itself.
            name: _,
            network_id: _,
            description: _,
            source: _,
            virtualization_metadata: _,
            virtualization_service_id: _,
            hidden: _,
            tags: _,
            credential_assignments: _,
            // Discovered attributes.
            hostname,
            sys_descr,
            sys_object_id,
            sys_location,
            sys_contact,
            management_url,
            chassis_id,
            sys_name,
            manufacturer,
            model,
            serial_number,
            firmware_revision,
            software_revision,
        } = incoming;

        // A macro rather than a closure: each field is a different carrier type, so every call is
        // monomorphised separately and picks up that field's own blank rule and refreshable policy.
        let mut changed = false;
        macro_rules! merge {
            ($($field:ident),* $(,)?) => {
                $(
                    if let Some(candidate) = $field
                        && Attributed::apply(&mut self.$field, candidate.clone())
                    {
                        changed = true;
                    }
                )*
            };
        }
        merge!(
            hostname,
            sys_descr,
            sys_object_id,
            sys_location,
            sys_contact,
            management_url,
            chassis_id,
            sys_name,
            manufacturer,
            model,
            serial_number,
            firmware_revision,
            software_revision,
        );
        changed
    }

    /// Refuse a daemon payload's claim that a person typed this name into Scanopy, keeping the
    /// name itself. Returns whether anything changed.
    ///
    /// A single source rather than a rank ceiling, because `Manual` is the only claim a daemon
    /// cannot make: every other source is something a daemon legitimately observed, and demoting
    /// by rank would strip the provenance off every inbound name rather than the one that is not
    /// a daemon's to assert.
    ///
    /// The value survives at `Unspecified` — the claim told us nothing believable about where the
    /// name came from, and nothing believable is exactly what `Unspecified` means. It keeps the
    /// name while letting the next real reading correct the rung.
    pub fn reject_manual_name_claim(&mut self) -> bool {
        if self.name.source() != AttributeSource::Manual {
            return false;
        }
        self.name = self.name.clone().clamped_to(AttributeSource::Unspecified);
        true
    }

    /// Drop a name that is really an identifier copied into `name`. Returns whether anything
    /// changed.
    ///
    /// Daemons up to v0.17.14 sent the hostname again as the name, and the address as an
    /// `OwnAddress` name. The same payload carries each in its own column, which is where the
    /// placement rule in [`super::name`] keeps identifiers, so the copy is dropped rather than
    /// stored twice.
    pub fn drop_identifier_copy(&mut self) -> bool {
        if !crate::server::hosts::r#impl::name::is_identifier_copy(self.name.source()) {
            return false;
        }
        self.name = HostName::unnamed();
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Default, ToSchema, Validate)]
#[schema(example = crate::server::shared::types::examples::host)]
pub struct Host {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    /// SCD2: when this row version became live. Equal to `created_at` for
    /// rows that have never ridden a snapshot; advanced to the snapshot's
    /// `taken_at` for live rows after a network snapshot fires.
    #[serde(default)]
    #[schema(read_only)]
    pub valid_from: DateTime<Utc>,
    /// SCD2: when this row was closed by a snapshot. NULL = currently live.
    #[serde(default)]
    #[schema(read_only)]
    pub valid_to: Option<DateTime<Utc>>,
    /// Lineage pointer on closed historical rows back to the live row whose
    /// state they capture. NULL on live rows.
    #[serde(default)]
    #[schema(read_only)]
    pub lineage_id: Option<Uuid>,
    /// Last successful natural-key match by daemon discovery against this
    /// live row. Refreshed every scan, regardless of field changes.
    #[serde(default)]
    #[schema(read_only)]
    pub last_seen_at: DateTime<Utc>,
    /// Discovery (historical row) that last touched this entity. Populated
    /// post-terminal by the per-entity-service subscriber on
    /// `DiscoveryProcessed`. NULL until the first successful discovery
    /// session terminates after this row was created.
    #[serde(default)]
    #[schema(read_only)]
    pub last_discovery_id: Option<Uuid>,
    /// Discovery (historical row) that first observed this entity. Set once
    /// (post-terminal); immutable thereafter via the `IS NULL` guard in
    /// `update_discovery_fks`.
    #[serde(default)]
    #[schema(read_only)]
    pub first_discovery_id: Option<Uuid>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: HostBase,
}

impl Hash for Host {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Host {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {:?}", self.base.name, self.id)
    }
}

impl Host {
    pub fn new(base: HostBase) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base,
        }
    }
}

impl ChangeTriggersTopologyStaleness<Host> for Host {
    fn triggers_staleness(&self, other: Option<Host>) -> bool {
        if let Some(other_host) = other {
            // The value only: a better source for the same hostname redraws nothing.
            attribution::text_of(&self.base.hostname)
                != attribution::text_of(&other_host.base.hostname)
                || self.base.virtualization_metadata != other_host.base.virtualization_metadata
                || self.base.virtualization_service_id != other_host.base.virtualization_service_id
                || self.base.hidden != other_host.base.hidden
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::attributes::{
        HostChassisIdValue, HostHostnameValue, HostModelValue, HostSysNameValue,
    };
    use crate::server::hosts::r#impl::name_ladder::HostNameRung;
    use crate::server::services::r#impl::patterns::ClientProbe;

    fn controller_name(name: &str) -> HostName {
        HostName::from_controller(name.to_string(), ClientProbe::UnifiController)
    }

    /// A host carrying nothing but the rungs under test, so a fall-through cannot be masked by a
    /// leftover value from a fuller fixture.
    fn nameless_host() -> Host {
        let mut host = crate::server::shared::types::examples::host();
        host.base.name = HostName::unnamed();
        host.base.hostname = None;
        host
    }

    fn probed<V>(value: V) -> Attributed<V>
    where
        V: crate::server::shared::attribution::AttributeValue,
    {
        Attributed::new(value, AttributeSource::Probe(ClientProbe::Snmp))
    }

    /// The ladder descends only as far as it has to.
    ///
    /// Written as one walk down rather than a case per rung: what matters is the *ordering* between
    /// them — that a sysName never displaces a hostname, and an address never displaces either —
    /// and an assertion per rung in isolation would pass even if the `or_else` chain were shuffled.
    ///
    /// Each step also asserts the rung. The editor tells a person which piece of evidence named the
    /// host, so a rung that disagreed with the value would explain the title wrongly.
    #[test]
    fn display_name_stops_at_the_highest_rung_the_host_carries() {
        let addresses = [crate::server::shared::types::examples::ip_address()];
        let mut host = nameless_host();
        let titled = |value: &str, rung| Some((value.to_string(), rung));

        // Nothing at all: absence, not `Some("")`. This is what every caller's fallback hangs on —
        // a blank title would be read as a name the host actually has.
        assert_eq!(host.display_name(&addresses[..0]), None);
        assert_eq!(host.resolved_name(&addresses[..0]), None);

        // The bottom rung, reached only because the four above are empty.
        assert_eq!(
            host.resolved_name(&addresses),
            titled("192.168.1.100", HostNameRung::Address)
        );

        host.base.chassis_id = Some(probed(HostChassisIdValue("00:1a:2b:3c:4d:5e".to_string())));
        assert_eq!(
            host.resolved_name(&addresses),
            titled("00:1a:2b:3c:4d:5e", HostNameRung::ChassisId)
        );

        host.base.sys_name = Some(probed(HostSysNameValue("core-sw-01".to_string())));
        assert_eq!(
            host.resolved_name(&addresses),
            titled("core-sw-01", HostNameRung::SysName)
        );

        host.base.hostname = Some(Attributed::new(
            HostHostnameValue("switch.lan".to_string()),
            AttributeSource::ReverseDns,
        ));
        assert_eq!(
            host.resolved_name(&addresses),
            titled("switch.lan", HostNameRung::Hostname)
        );

        host.base.name = HostName::manual("Core Switch".to_string());
        assert_eq!(
            host.resolved_name(&addresses),
            titled("Core Switch", HostNameRung::Name)
        );
        assert_eq!(
            host.display_name(&addresses),
            Some("Core Switch".to_string())
        );

        // A person clearing the name hands the title back to the evidence below it.
        assert!(host.base.clear_name());
        assert_eq!(host.base.name.source(), AttributeSource::Unspecified);
        assert_eq!(
            host.resolved_name(&addresses),
            titled("switch.lan", HostNameRung::Hostname)
        );
    }

    /// A rung holding whitespace is not a rung.
    ///
    /// SNMP agents and controllers return `" "` and `""` for fields they don't populate, and a
    /// host titled with a space is indistinguishable on screen from one titled with nothing —
    /// except that it silently outranks the real evidence below it.
    #[test]
    fn display_name_treats_a_blank_rung_as_absent() {
        let addresses = [crate::server::shared::types::examples::ip_address()];
        let mut host = nameless_host();
        host.base.hostname = Some(Attributed::new(
            HostHostnameValue("   ".to_string()),
            AttributeSource::ReverseDns,
        ));
        host.base.sys_name = Some(probed(HostSysNameValue(String::new())));
        host.base.chassis_id = Some(probed(HostChassisIdValue("  ".to_string())));

        assert_eq!(
            host.resolved_name(&addresses),
            Some(("192.168.1.100".to_string(), HostNameRung::Address))
        );
    }

    /// `apply_name`'s return value is what `upsert_host` uses to decide whether the host actually
    /// changed, and an Updated event (and a topology rebuild) rides on that. A re-sync that
    /// reports the same name must be silent, not a no-op write that still looks like a change.
    #[test]
    fn reapplying_an_unchanged_name_reports_no_change() {
        let mut base = HostBase::default();
        assert!(base.apply_name(controller_name("Core Switch")));
        assert!(!base.apply_name(controller_name("Core Switch")));
        assert!(base.apply_name(controller_name("Core Switch 2")));
    }

    /// The same value arriving from a *better* source is still a change worth recording: the name
    /// reads the same, but the host is now protected from the rungs in between.
    #[test]
    fn the_same_name_from_a_higher_rung_is_recorded() {
        let mut base = HostBase::default();
        base.apply_name(HostName::from_service("switch.lan".to_string()));
        assert!(base.apply_name(controller_name("switch.lan")));
        assert_eq!(
            base.name.source(),
            AttributeSource::Authored(ClientProbe::UnifiController)
        );
    }

    /// What a person typed into Scanopy survives every subsequent scan. This used to be the
    /// `is_none()` gate — first writer wins, whoever they were — and is now `Manual` outranking
    /// everything discovery can produce, which is what makes a refreshable `model` safe.
    #[test]
    fn a_manually_entered_value_is_never_displaced() {
        let mut existing = HostBase {
            model: Some(Attributed::new(
                HostModelValue("typed-by-a-person".to_string()),
                AttributeSource::Manual,
            )),
            ..Default::default()
        };
        let incoming = HostBase {
            model: Some(Attributed::new(
                HostModelValue("read-over-snmp".to_string()),
                AttributeSource::Probe(ClientProbe::Snmp),
            )),
            serial_number: Some(Attributed::new(
                crate::server::hosts::r#impl::attributes::HostSerialNumberValue(
                    "FOC1234X5YZ".to_string(),
                ),
                AttributeSource::Probe(ClientProbe::Snmp),
            )),
            ..Default::default()
        };

        assert!(existing.apply_attributes_from(&incoming));

        assert_eq!(
            attribution::text_of(&existing.model).as_deref(),
            Some("typed-by-a-person")
        );
        assert_eq!(
            attribution::text_of(&existing.serial_number).as_deref(),
            Some("FOC1234X5YZ")
        );
    }

    /// The behaviour the `is_none()` gate could not express: a value already present is displaced
    /// when a better source reads it. Under first-write-wins the model below stayed "Cisco Switch"
    /// for the life of the host, whatever SNMP later said.
    #[test]
    fn a_weak_value_is_displaced_by_a_stronger_source() {
        let mut existing = HostBase {
            model: Some(Attributed::new(
                HostModelValue("Cisco Switch".to_string()),
                AttributeSource::Probe(ClientProbe::UnifiController),
            )),
            ..Default::default()
        };
        let incoming = HostBase {
            model: Some(Attributed::new(
                HostModelValue("WS-C2960X-48FPD-L".to_string()),
                AttributeSource::Probe(ClientProbe::Snmp),
            )),
            ..Default::default()
        };

        assert!(existing.apply_attributes_from(&incoming));
        assert_eq!(
            attribution::text_of(&existing.model).as_deref(),
            Some("WS-C2960X-48FPD-L")
        );
    }

    /// The ordering this item exists to establish, on the field that prompted it. ENTITY-MIB is
    /// Track 2's reader, but its rung is decided here: a device answering SNMP outranks a
    /// controller describing a device it manages, so a firmware revision from the MIB displaces
    /// one a controller reported rather than losing to whichever probe finished first.
    #[test]
    fn firmware_from_the_device_displaces_firmware_from_a_controller() {
        use crate::server::hosts::r#impl::attributes::HostFirmwareRevisionValue;

        let mut existing = HostBase {
            firmware_revision: Some(Attributed::new(
                HostFirmwareRevisionValue("6.5.59".to_string()),
                AttributeSource::Probe(ClientProbe::UnifiController),
            )),
            ..Default::default()
        };
        let incoming = HostBase {
            firmware_revision: Some(Attributed::new(
                HostFirmwareRevisionValue("17.03.01".to_string()),
                AttributeSource::Probe(ClientProbe::Snmp),
            )),
            ..Default::default()
        };

        assert!(existing.apply_attributes_from(&incoming));
        assert_eq!(
            attribution::text_of(&existing.firmware_revision).as_deref(),
            Some("17.03.01")
        );
    }

    /// `upsert_host` publishes an Updated event and triggers a topology rebuild off this return
    /// value, so a scan that learns nothing new must report no change.
    #[test]
    fn learning_nothing_new_reports_no_change() {
        let snmp = AttributeSource::Probe(ClientProbe::Snmp);
        let model = |v: &str| Some(Attributed::new(HostModelValue(v.to_string()), snmp));
        let mut existing = HostBase {
            model: model("WS-C2960X"),
            ..Default::default()
        };
        let incoming = HostBase {
            model: model("WS-C2960X"),
            ..Default::default()
        };

        assert!(!existing.apply_attributes_from(&incoming));
        assert!(!existing.apply_attributes_from(&HostBase::default()));
    }
}
