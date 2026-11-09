//! The field registry: what each entity type is described by. See `types/entity_field.rs`.
//!
//! Every key an entity serializes has an entry here, whichever surfaces show it. Ids keep the
//! column names CSV export used before the registry, and the list tables' column keys, so neither
//! saved column preferences nor scripts reading the CSV lose a column. An `Entity` field whose id
//! ends in `_id` is a foreign key: CSV writes the related entity's id. A field read through
//! `Hop::Children` writes ids in CSV, whatever its format (`EntityField::for_csv`).

use std::any::Any;

use crate::server::{
    credentials::r#impl::base::Credential,
    daemon_api_keys::r#impl::base::DaemonApiKey,
    daemons::r#impl::base::Daemon,
    dependencies::r#impl::base::Dependency,
    discovery::r#impl::base::Discovery,
    hosts::r#impl::base::Host,
    interfaces::r#impl::base::Interface,
    ip_addresses::r#impl::base::IPAddress,
    organizations::r#impl::base::Organization,
    ports::r#impl::base::Port,
    services::r#impl::base::Service,
    shared::entities::EntityDiscriminants as E,
    shared::types::entity_field::{
        EntityField as F, FieldFormat, FieldGroup as G, FieldRule, FieldWrite, Hop,
        MetadataFixture as Fx,
    },
    shares::r#impl::base::Share,
    subnets::r#impl::base::Subnet,
    tags::r#impl::base::Tag,
    topology::types::base::Topology,
    user_api_keys::r#impl::base::UserApiKey,
    users::r#impl::base::User,
    vlans::r#impl::base::Vlan,
};

/// A rule's two entities, as the type its registry entry was declared for.
fn typed<'a, T: 'static>(incoming: &'a mut dyn Any, stored: &'a dyn Any) -> (&'a mut T, &'a T) {
    let wrong_entity = "a field rule runs on the entity it is registered for";
    (
        incoming.downcast_mut().expect(wrong_entity),
        stored.downcast_ref().expect(wrong_entity),
    )
}

/// Every update keeps the stored value.
macro_rules! pin {
    ($entity:ty, $($field:tt).+) => {
        FieldRule(|incoming, stored| {
            let (incoming, stored) = typed::<$entity>(incoming, stored);
            #[allow(clippy::clone_on_copy)]
            let value = stored.$($field).+.clone();
            incoming.$($field).+ = value;
        })
    };
}

/// An update that carries `None` keeps the stored value.
macro_rules! keep {
    ($entity:ty, $($field:tt).+) => {
        FieldRule(|incoming, stored| {
            let (incoming, stored) = typed::<$entity>(incoming, stored);
            if incoming.$($field).+.is_none() {
                #[allow(clippy::clone_on_copy)]
                let value = stored.$($field).+.clone();
                incoming.$($field).+ = value;
            }
        })
    };
}

/// An update reconciles the two values through a method on the entity.
macro_rules! merge {
    ($entity:ty, $method:ident) => {
        FieldRule(|incoming, stored| {
            let (incoming, stored) = typed::<$entity>(incoming, stored);
            incoming.$method(stored);
        })
    };
}

const SITE: F =
    F::new("site_id", "Site", G::Relationships).entity(&[Hop::parent(E::Site, "site_id")]);
const SOURCE: F = F::new("source", "Source", G::Discovery)
    .field("source.type")
    .metadata(Fx::EntitySources);
const DESCRIPTION: F = F::new("description", "Description", G::Identity).format(FieldFormat::Prose);
const TIMESTAMP: FieldFormat = FieldFormat::Timestamp;
const TAGS: F = F::internal("tags")
    .entity(&[Hop::parent(E::Tag, "tags")])
    .many();
const ORGANIZATION_ID: F = F::internal("organization_id");

// List-table columns. A column read from a response type or worked out by the table is `computed`.
const CREATED_AT: F = F::column("created_at", "Created", G::Timestamps);
const UPDATED_AT: F = F::column("updated_at", "Updated", G::Timestamps);
const LAST_SEEN_AT: F = F::column("last_seen_at", "Last seen", G::Discovery);
const FIRST_FOUND_BY: F = F::column("first_found_by", "First Found By", G::Discovery).computed();
const LAST_FOUND_BY: F = F::column("last_found_by", "Last Found By", G::Discovery).computed();
// Timestamps on entities without a list table, kept only for their write policy.
const INTERNAL_CREATED_AT: F = F::internal("created_at");
const INTERNAL_UPDATED_AT: F = F::internal("updated_at");

const HOST: &[F] = &[
    F::new("name", "Name", G::Identity)
        .source("name_source")
        .summary()
        .searchable(),
    F::new("hostname", "Hostname", G::Identity)
        .source("hostname_source")
        .summary()
        .searchable(),
    F::new("interface_ip", "IP Addresses", G::Network)
        .via(
            &[Hop::children(E::IPAddress, "host_id")],
            Some("ip_address"),
        )
        .format(FieldFormat::Identifier)
        .summary(),
    F::new("interfaces", "Interfaces", G::Network)
        .entity(&[Hop::children(E::Interface, "host_id")]),
    F::new("services", "Services", G::Relationships)
        .entity(&[Hop::children(E::Service, "host_id")])
        .summary(),
    F::new("ports", "Ports", G::Network)
        .via(&[Hop::children(E::Port, "host_id")], None)
        .format(FieldFormat::Port),
    F::new("virtualized_by", "Virtualized By", G::Virtualization)
        .entity(&[Hop::parent(E::Service, "virtualization_service_id")])
        .summary(),
    F::new("presented_by", "Presented By", G::Virtualization)
        .entity(&[Hop::parent(E::Interface, "virtualization_interface_id")]),
    F::new("os_family", "Operating System", G::Device)
        .field("os")
        .source("os_source")
        .format(FieldFormat::OperatingSystem)
        .summary(),
    F::new("manufacturer", "Manufacturer", G::Device)
        .source("manufacturer_source")
        .searchable(),
    F::new("model", "Model", G::Device)
        .source("model_source")
        .searchable(),
    F::new("serial_number", "Serial Number", G::Identity)
        .source("serial_number_source")
        .format(FieldFormat::Identifier),
    F::new("asset_tag", "Asset Tag", G::Identity)
        .source("asset_tag_source")
        .format(FieldFormat::Identifier),
    F::new("firmware_revision", "Firmware Revision", G::Device)
        .source("firmware_revision_source")
        .format(FieldFormat::Identifier),
    F::new("software_revision", "Software Revision", G::Device)
        .source("software_revision_source")
        .format(FieldFormat::Identifier),
    F::new("sys_name", "System Name", G::Identity)
        .source("sys_name_source")
        .searchable(),
    F::new("sys_descr", "System Description", G::Device).source("sys_descr_source"),
    F::new("sys_object_id", "System OID", G::Device)
        .source("sys_object_id_source")
        .format(FieldFormat::Identifier),
    F::new("sys_location", "Location", G::Device).source("sys_location_source"),
    F::new("sys_contact", "Contact", G::Device).source("sys_contact_source"),
    F::new("chassis_id", "Chassis ID", G::Identity)
        .source("chassis_id_source")
        .format(FieldFormat::Identifier)
        .searchable(),
    F::new("management_url", "Management URL", G::Device).source("management_url_source"),
    DESCRIPTION,
    SOURCE.merged(merge!(Host, merge_source)),
    // After `source`: the dismissal reads the merged source.
    F::internal("dismissed_at").merged(merge!(Host, merge_dismissed_at)),
    SITE,
    F::internal("virtualization_metadata"),
    F::internal("matched_device_type").source("matched_device_type_source"),
    F::internal("credential_assignments").entity(&[
        Hop::embedded("credential_assignments"),
        Hop::parent(E::Credential, "credential_id"),
    ]),
    F::internal("credential_assignment_addresses").entity(&[
        Hop::embedded("credential_assignments"),
        Hop::parent(E::IPAddress, "ip_address_ids"),
    ]),
    TAGS,
    // Read from `HostResponse`, not the stored `Host`, so these are worked out for the table.
    F::column("device_type", "Device type", G::Device).computed(),
    F::column("mac_address", "MAC Address", G::Network).computed(),
    F::column("credentials", "Credentials", G::Relationships).computed(),
    LAST_SEEN_AT,
    FIRST_FOUND_BY,
    LAST_FOUND_BY,
    CREATED_AT.immutable(pin!(Host, created_at)),
    UPDATED_AT.immutable(pin!(Host, updated_at)),
];

const IP_ADDRESS: &[F] = &[
    F::new("ip_address", "IP Address", G::Network)
        .source("ip_address_source")
        .format(FieldFormat::Identifier)
        .summary()
        .searchable(),
    F::new("mac_address", "MAC Address", G::Network)
        .source("mac_address_source")
        .format(FieldFormat::Identifier)
        .summary()
        .searchable()
        .merged(merge!(IPAddress, merge_mac_address)),
    F::new("name", "Name", G::Identity).searchable(),
    F::new("host_id", "Host", G::Relationships)
        .entity(&[Hop::parent(E::Host, "host_id")])
        .summary(),
    F::new("subnet_id", "Subnet", G::Network)
        .entity(&[Hop::parent(E::Subnet, "subnet_id")])
        .summary(),
    F::new("services", "Services", G::Relationships).entity(&[
        Hop::children(E::Binding, "ip_address_id"),
        Hop::parent(E::Service, "service_id"),
    ]),
    F::new("ports", "Ports", G::Network)
        .via(
            &[
                Hop::children(E::Binding, "ip_address_id"),
                Hop::parent(E::Port, "port_id"),
            ],
            None,
        )
        .format(FieldFormat::Port),
    F::new("position", "Position", G::Configuration).format(FieldFormat::Number),
    SITE,
    F::internal("ip_address_source").merged(merge!(IPAddress, merge_ip_address_source)),
    INTERNAL_CREATED_AT.immutable(pin!(IPAddress, created_at)),
];

const INTERFACE: &[F] = &[
    F::new("display_name", "Interface", G::Identity)
        .summary()
        .searchable(),
    F::new("host_id", "Host", G::Relationships)
        .entity(&[Hop::parent(E::Host, "host_id")])
        .summary(),
    F::new("mac_address", "MAC Address", G::Network)
        .source("mac_address_source")
        .format(FieldFormat::Identifier)
        .summary()
        .searchable()
        .merged(merge!(Interface, merge_mac_address)),
    F::new("speed_bps", "Speed", G::Network)
        .format(FieldFormat::BitsPerSecond)
        .summary(),
    // `None` on an incoming row means "this source has nothing to say about it", never "the
    // device stopped reporting it": the two are indistinguishable at this layer, and only the
    // first is safe to assume. Without this, a match resolved on Tier 3 (MAC) by a source that
    // carries only a MAC, the PROFINET DCP case, replaces the whole row with its own near-empty
    // one, erasing an SNMP walk's if_index/if_type/statuses/if_descr. A real SNMP re-walk still
    // overwrites its own prior reading. Losing if_name also breaks tier-1 matching on the next
    // scan.
    F::new("oper_status", "Status", G::Status)
        .metadata(Fx::IfOperStatuses)
        .summary()
        .keep_when_none(keep!(Interface, base.oper_status)),
    F::new("admin_status", "Admin Status", G::Status)
        .keep_when_none(keep!(Interface, base.admin_status)),
    F::new("native_vlan_id", "Native VLAN", G::Network)
        .entity(&[Hop::parent(E::Vlan, "native_vlan_id")])
        .source("vlan_membership_source")
        .summary()
        .searchable(),
    F::new("vlan_ids", "Tagged VLANs", G::Network)
        .entity(&[Hop::parent(E::Vlan, "vlan_ids")])
        .source("vlan_membership_source")
        .many()
        .searchable(),
    F::new("ip_address_id", "IP Address", G::Network)
        .entity(&[Hop::parent(E::IPAddress, "ip_address_id")]),
    F::new("if_name", "ifName", G::Identity)
        .searchable()
        .keep_when_none(keep!(Interface, base.if_name)),
    F::new("if_descr", "ifDescr", G::Identity)
        .searchable()
        .keep_when_none(keep!(Interface, base.if_descr)),
    F::new("if_alias", "Alias", G::Identity).searchable(),
    F::new("if_type", "ifType", G::Identity).keep_when_none(keep!(Interface, base.if_type)),
    F::new("if_index", "Index", G::Identity)
        .format(FieldFormat::Number)
        .keep_when_none(keep!(Interface, base.if_index)),
    F::new("ip_configured", "IP Configured", G::Network).format(FieldFormat::Boolean),
    F::new("fdb_macs", "Forwarding Table MACs", G::Network)
        .source("fdb_macs_source")
        .format(FieldFormat::Identifier)
        .many(),
    F::new("interface_source", "Source", G::Identity)
        .format(FieldFormat::AttributeSource)
        .merged(merge!(Interface, merge_interface_source)),
    SITE,
    F::internal("neighbor_candidates").many(),
    INTERNAL_CREATED_AT.immutable(pin!(Interface, created_at)),
];

const SERVICE: &[F] = &[
    F::new("name", "Name", G::Identity).summary().searchable(),
    F::new("service_definition", "Service Type", G::Identity)
        .metadata(Fx::ServiceDefinitions)
        .summary()
        .searchable(),
    F::new("host", "Host", G::Relationships)
        .entity(&[Hop::parent(E::Host, "host_id")])
        .summary(),
    F::new("port_bindings", "Port Bindings", G::Network)
        .via(
            &[Hop::embedded("bindings"), Hop::parent(E::Port, "port_id")],
            None,
        )
        .format(FieldFormat::Port)
        .summary(),
    F::new("ip_bindings", "IP Address Bindings", G::Network).entity(&[
        Hop::embedded("bindings"),
        Hop::parent(E::IPAddress, "ip_address_id"),
    ]),
    // Both virtualization halves are preserved independently: a caller that supplies neither must
    // not lose the container's identity or its owning runtime.
    F::new("containerized_by", "Containerized By", G::Virtualization)
        .entity(&[Hop::parent(E::Service, "virtualization_service_id")])
        .keep_when_none(keep!(Service, base.virtualization_service_id)),
    F::internal("virtualization_metadata")
        .keep_when_none(keep!(Service, base.virtualization_metadata)),
    F::new("container_name", "Container Name", G::Virtualization)
        .field("virtualization_metadata.details.container_name")
        .summary()
        .searchable(),
    F::new("container_id", "Container ID", G::Virtualization)
        .field("virtualization_metadata.details.container_id")
        .format(FieldFormat::Identifier),
    F::new("compose_project", "Compose Project", G::Virtualization)
        .field("virtualization_metadata.details.compose_project")
        .searchable(),
    F::new("guest_state", "Container State", G::Virtualization)
        .field("virtualization_metadata.details.state")
        .metadata(Fx::GuestStates)
        .summary(),
    F::new("position", "Position", G::Configuration).format(FieldFormat::Number),
    SOURCE.immutable(pin!(Service, base.source)),
    SITE,
    TAGS,
    F::column("category", "Category", G::Identity).computed(),
    F::column("match_confidence", "Match Confidence", G::Discovery).computed(),
    LAST_SEEN_AT,
    FIRST_FOUND_BY,
    LAST_FOUND_BY,
    CREATED_AT,
    UPDATED_AT,
];

const PORT: &[F] = &[
    F::new("port_number", "Port", G::Network)
        .field("number")
        .source("port_source")
        .format(FieldFormat::Number)
        .summary(),
    F::new("protocol", "Protocol", G::Network).summary(),
    F::new("port_type", "Port Type", G::Identity).field("type"),
    F::new("host_id", "Host", G::Relationships).entity(&[Hop::parent(E::Host, "host_id")]),
    SITE,
    F::internal("port_source").merged(merge!(Port, merge_port_source)),
];

const BINDING: &[F] = &[
    F::new("binding_type", "Binding Type", G::Identity).field("type"),
    F::new("service_id", "Service", G::Relationships)
        .entity(&[Hop::parent(E::Service, "service_id")]),
    F::new("ip_address_id", "IP Address", G::Network)
        .entity(&[Hop::parent(E::IPAddress, "ip_address_id")]),
    F::new("port_id", "Port", G::Network).entity(&[Hop::parent(E::Port, "port_id")]),
    SITE,
];

const SUBNET: &[F] = &[
    F::new("name", "Name", G::Identity)
        .summary()
        .searchable()
        .merged(merge!(Subnet, merge_name)),
    F::new("cidr", "CIDR", G::Network)
        .source("cidr_source")
        .format(FieldFormat::Identifier)
        .summary()
        .searchable()
        .merged(merge!(Subnet, merge_cidr)),
    F::new("subnet_type", "Subnet Type", G::Identity)
        .metadata(Fx::SubnetTypes)
        .summary()
        .merged(merge!(Subnet, merge_subnet_type)),
    DESCRIPTION,
    F::new("vlan_ids", "VLANs", G::Network)
        .entity(&[Hop::parent(E::Vlan, "vlan_ids")])
        .many()
        .summary(),
    F::new("managed_by", "Managed By", G::Virtualization)
        .entity(&[Hop::parent(E::Service, "virtualization_service_id")]),
    // After `cidr`: settling the range is what confirms an inferred subnet.
    SOURCE.merged(merge!(Subnet, merge_source)),
    SITE,
    F::internal("name_source"),
    F::internal("subnet_type_source"),
    TAGS,
    F::column("utilization", "Utilization", G::Network).computed(),
    LAST_SEEN_AT,
    FIRST_FOUND_BY,
    LAST_FOUND_BY,
    CREATED_AT.immutable(pin!(Subnet, created_at)),
    UPDATED_AT.immutable(pin!(Subnet, updated_at)),
];

const VLAN: &[F] = &[
    // A VLAN is identified by (site_id, vlan_number). Create enforces that pair is unique per
    // site; an update has no such check, so neither half may move on update.
    F::new("vlan_number", "VLAN Number", G::Network)
        .format(FieldFormat::Number)
        .summary()
        .searchable()
        .immutable(pin!(Vlan, base.vlan_number)),
    F::new("name", "Name", G::Identity)
        .summary()
        .searchable()
        .merged(merge!(Vlan, merge_name)),
    DESCRIPTION.summary(),
    F::new("subnet_ids", "Subnets", G::Network)
        .entity(&[Hop::parent(E::Subnet, "subnet_ids")])
        .many()
        .server_owned(pin!(Vlan, base.subnet_ids)),
    F::new("interface_ids", "Interfaces", G::Network)
        .entity(&[Hop::parent(E::Interface, "interface_ids")])
        .many()
        .server_owned(pin!(Vlan, base.interface_ids)),
    SOURCE.immutable(pin!(Vlan, base.source)),
    SITE.immutable(pin!(Vlan, base.site_id)),
    ORGANIZATION_ID.immutable(pin!(Vlan, base.organization_id)),
    F::internal("name_source"),
    TAGS,
    LAST_SEEN_AT,
    FIRST_FOUND_BY,
    LAST_FOUND_BY,
    CREATED_AT,
    UPDATED_AT,
];

const DEPENDENCY: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    DESCRIPTION.summary(),
    F::new("dependency_type", "Dependency Type", G::Identity)
        .metadata(Fx::DependencyTypes)
        .summary(),
    F::new("color", "Color", G::Identity),
    SOURCE.immutable(pin!(Dependency, base.source)),
    SITE,
    F::internal("members"),
    F::internal("edge_style"),
    TAGS,
    INTERNAL_CREATED_AT.immutable(pin!(Dependency, created_at)),
    INTERNAL_UPDATED_AT.immutable(pin!(Dependency, updated_at)),
];

const TAG: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    DESCRIPTION.summary(),
    F::new("color", "Color", G::Identity),
    F::new("tag_group", "Tag Group", G::Relationships)
        .entity(&[Hop::parent(E::TagGroup, "tag_group_id")]),
    F::new("icon", "Icon", G::Identity),
    ORGANIZATION_ID,
    F::internal("position"),
    // Set by the server: `System` when seeded, `Manual` when a person creates it.
    F::internal("source").immutable(pin!(Tag, base.source)),
    CREATED_AT,
    UPDATED_AT,
];

const TAG_GROUP: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    DESCRIPTION.tooltip(),
    F::new("group_type", "Group Type", G::Identity),
    F::new("entity_types", "Applies To", G::Configuration).many(),
    ORGANIZATION_ID,
    F::internal("scope_computed"),
    F::column("ordered", "Tag order", G::Configuration),
    CREATED_AT,
    UPDATED_AT,
];

const DAEMON: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    // Decides polling enrolment and the shape of the daemon's own on-disk config. The daemon's
    // handshake overwrites it, so a user edit would silently revert.
    F::new("mode", "Daemon Mode", G::Configuration).immutable(pin!(Daemon, base.mode)),
    // Self-reported by the daemon on every handshake.
    F::new("version", "Version", G::Status)
        .summary()
        .immutable(pin!(Daemon, base.version)),
    F::new("last_seen", "Last Seen", G::Status)
        .format(TIMESTAMP)
        .immutable(pin!(Daemon, base.last_seen)),
    // Editable for ServerPoll: it is the address the server dials, and a daemon can move. The
    // update handler rejects changing it in DaemonPoll mode, where it is unused.
    F::new("url", "URL", G::Configuration),
    // The daemon's own Host record, created at provision time.
    F::new("host_id", "Host", G::Relationships)
        .entity(&[Hop::parent(E::Host, "host_id")])
        .immutable(pin!(Daemon, base.host_id)),
    F::new("maintainer", "Maintainer", G::Relationships).entity(&[Hop::parent(E::User, "user_id")]),
    // The tenancy boundary: the daemon's key, host, seeded loopback and discovery jobs were all
    // created against it. The daemon's handshake also overwrites it.
    SITE.immutable(pin!(Daemon, base.site_id)),
    // The 1:1 key binding, maintained by provisioning.
    F::internal("api_key_id").server_owned(pin!(Daemon, base.api_key_id)),
    // Managed by the inactivity background task and the reactivation paths (startup and discovery
    // queue).
    F::internal("standby").server_owned(pin!(Daemon, base.standby)),
    F::internal("standby_cleared_at").server_owned(pin!(Daemon, base.standby_cleared_at)),
    F::internal("is_unreachable"),
    TAGS,
    F::column("status", "Status", G::Status).computed(),
    // Chosen when the daemon is created and fixed after: credentials are checked against it.
    F::column("os", "Operating system", G::Device).immutable(pin!(Daemon, base.os)),
    F::column("interfaced_subnet_ids", "Interfaces With", G::Network).computed(),
    CREATED_AT,
    UPDATED_AT,
];

const DISCOVERY: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::new("discovery_type", "Discovery Type", G::Identity)
        .field("discovery_type.type")
        .metadata(Fx::DiscoveryTypes)
        .summary(),
    F::new("run_type", "Run Type", G::Configuration).field("run_type.type"),
    F::new("daemon_id", "Daemon", G::Relationships).entity(&[Hop::parent(E::Daemon, "daemon_id")]),
    SITE,
    F::internal("next_run_is_full_scan"),
    F::internal("integration_targets").entity(&[
        Hop::embedded("integration_targets"),
        Hop::parent(E::Credential, "credential_id"),
    ]),
    F::internal("scanned_subnet_ids").entity(&[
        Hop::embedded("discovery_type"),
        Hop::parent(E::Subnet, "subnet_ids"),
    ]),
    F::internal("scanned_host_id").entity(&[
        Hop::embedded("discovery_type"),
        Hop::parent(E::Host, "host_id"),
    ]),
    TAGS,
    F::column("last_run", "Last run", G::Status).computed(),
    F::column("scan_count", "Completed scans", G::Status).server_owned(pin!(Discovery, scan_count)),
    F::column("legacy", "Status", G::Status).computed(),
    F::column("progress", "Progress", G::Status).computed(),
    F::column("phase", "Status", G::Status).computed(),
    F::column("started_at", "Started At", G::Status).computed(),
    F::column("finished_at", "Finished At", G::Status).computed(),
    F::column("duration", "Duration", G::Status).computed(),
    F::column("warnings", "Warnings", G::Status).computed(),
    F::column("schedule", "Schedule", G::Configuration).computed(),
    F::column(
        "force_full_scan",
        "Force full scan on next run",
        G::Configuration,
    ),
    F::column("scan_mode", "Scan Mode", G::Configuration).computed(),
    CREATED_AT,
    UPDATED_AT,
];

const DAEMON_API_KEY: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::new("is_enabled", "Enabled", G::Access)
        .format(FieldFormat::Boolean)
        .tooltip(),
    F::new("last_used", "Last Used", G::Access)
        .format(TIMESTAMP)
        .tooltip()
        .server_owned(pin!(DaemonApiKey, base.last_used)),
    F::new("expires_at", "Expires", G::Access).format(TIMESTAMP),
    // The 1:1 binding, set at provision only.
    F::new("daemon_id", "Daemon", G::Relationships)
        .entity(&[Hop::parent(E::Daemon, "daemon_id")])
        .server_owned(pin!(DaemonApiKey, base.daemon_id)),
    SITE,
    // The hash cannot be changed via update: the rotate endpoint does that.
    F::internal("key").server_owned(pin!(DaemonApiKey, base.key)),
    // An inbound update request always deserializes this to `None`. Without keeping it, any PUT
    // would persist NULL and break ServerPoll polling, which needs the stored plaintext to dial
    // the daemon. Only provisioning and rotate set it, outside this update path.
    F::internal("plaintext")
        .unserialized()
        .keep_when_none(keep!(DaemonApiKey, base.plaintext)),
    TAGS,
    CREATED_AT,
    UPDATED_AT,
];

const USER_API_KEY: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::new("permissions", "Permissions", G::Access).metadata(Fx::Permissions),
    F::new("is_enabled", "Enabled", G::Access)
        .format(FieldFormat::Boolean)
        .tooltip(),
    F::new("last_used", "Last Used", G::Access)
        .format(TIMESTAMP)
        .tooltip()
        .server_owned(pin!(UserApiKey, base.last_used)),
    F::new("expires_at", "Expires", G::Access).format(TIMESTAMP),
    F::new("user_id", "User", G::Relationships)
        .entity(&[Hop::parent(E::User, "user_id")])
        .server_owned(pin!(UserApiKey, base.user_id)),
    ORGANIZATION_ID.server_owned(pin!(UserApiKey, base.organization_id)),
    // The hash cannot be changed via update: the rotate endpoint does that.
    F::internal("key").server_owned(pin!(UserApiKey, base.key)),
    TAGS,
    F::internal("id").immutable(pin!(UserApiKey, id)),
    F::column("site_ids", "Sites", G::Relationships)
        .entity(&[Hop::parent(E::Site, "site_ids")])
        .many(),
    CREATED_AT.immutable(pin!(UserApiKey, created_at)),
    UPDATED_AT,
];

const CREDENTIAL: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::new("credential_type", "Credential Type", G::Identity)
        .field("credential_type.type")
        .metadata(Fx::CredentialTypes)
        .summary()
        .merged(merge!(Credential, merge_credential_type)),
    DESCRIPTION.tooltip(),
    F::new("daemon_os", "Daemon OS", G::Configuration),
    F::internal("login_credential_id").entity(&[Hop::parent(E::Credential, "login_credential_id")]),
    // Hydrated from the credentials that name this login; a request can't set it.
    F::internal("used_by")
        .entity(&[Hop::parent(E::Credential, "used_by")])
        .many()
        .server_owned(pin!(Credential, base.used_by)),
    ORGANIZATION_ID,
    F::internal("assigned_site_ids")
        .entity(&[Hop::parent(E::Site, "assigned_site_ids")])
        .many(),
    F::internal("host_assignments").entity(&[
        Hop::embedded("host_assignments"),
        Hop::parent(E::Host, "host_id"),
    ]),
    F::internal("host_assignment_addresses").entity(&[
        Hop::embedded("host_assignments"),
        Hop::parent(E::IPAddress, "ip_address_ids"),
    ]),
    F::internal("is_default"),
    TAGS,
    // Set by the server: `System` when seeded, `Manual` when a person creates it.
    F::internal("source").immutable(pin!(Credential, base.source)),
    F::column("integration", "Integration", G::Identity).computed(),
    F::column("category", "Category", G::Identity).computed(),
    F::column("beta", "Beta", G::Identity).computed(),
    F::column("unofficial_api", "Unofficial API", G::Identity).computed(),
    F::column("target", "Targets", G::Relationships).computed(),
    F::column("assignments", "Assignments", G::Relationships).computed(),
    CREATED_AT,
    UPDATED_AT,
];

const SITE_FIELDS: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::new("stale_after_hours", "Stale After (hours)", G::Configuration)
        .format(FieldFormat::Number),
    ORGANIZATION_ID,
    F::internal("credential_ids")
        .entity(&[Hop::parent(E::Credential, "credential_ids")])
        .many(),
    TAGS,
    F::column(
        "effective_stale_after_hours",
        "Stale after",
        G::Configuration,
    ),
    F::column("vlans", "VLANs", G::Network).computed(),
    F::column("subnets", "Subnets", G::Network).computed(),
    F::column("daemons", "Daemons", G::Relationships).computed(),
    F::column("credentials", "Credentials", G::Relationships).computed(),
    CREATED_AT,
    UPDATED_AT,
];

// A request body may change only `permissions`, `site_ids`, `email_settings` and
// `display_settings`, and the handlers narrow that further. Every other field belongs to a server
// flow (signup, email change, verification, password reset, OIDC linking, "log out everywhere")
// and is taken from the stored user. `#[serde(skip)]` fields arrive at their defaults, so one with
// no policy here is reset on save.
const USER: &[F] = &[
    F::new("email", "Email", G::Identity)
        .summary()
        .immutable(pin!(User, base.email)),
    F::new("permissions", "Permissions", G::Access)
        .metadata(Fx::Permissions)
        .summary(),
    F::new("email_verified", "Email Verified", G::Access)
        .format(FieldFormat::Boolean)
        .server_owned(pin!(User, base.email_verified)),
    F::new("oidc_provider", "OIDC Provider", G::Access)
        .server_owned(pin!(User, base.oidc_provider)),
    ORGANIZATION_ID,
    F::internal("email_settings"),
    F::internal("display_settings"),
    F::internal("has_password").server_owned(pin!(User, base.has_password)),
    F::internal("terms_acceptance_required")
        .server_owned(pin!(User, base.terms_acceptance_required)),
    F::internal("email_verification_required")
        .server_owned(pin!(User, base.email_verification_required)),
    F::internal("password_hash")
        .unserialized()
        .server_owned(pin!(User, base.password_hash)),
    F::internal("oidc_subject")
        .unserialized()
        .server_owned(pin!(User, base.oidc_subject)),
    F::internal("terms_accepted_at")
        .unserialized()
        .server_owned(pin!(User, base.terms_accepted_at)),
    F::internal("email_verification_token")
        .unserialized()
        .server_owned(pin!(User, base.email_verification_token)),
    F::internal("email_verification_expires")
        .unserialized()
        .server_owned(pin!(User, base.email_verification_expires)),
    F::internal("password_reset_token")
        .unserialized()
        .server_owned(pin!(User, base.password_reset_token)),
    F::internal("password_reset_expires")
        .unserialized()
        .server_owned(pin!(User, base.password_reset_expires)),
    F::internal("pending_email")
        .unserialized()
        .server_owned(pin!(User, base.pending_email)),
    F::internal("email_unsubscribe_token")
        .unserialized()
        .server_owned(pin!(User, base.email_unsubscribe_token)),
    F::internal("session_epoch")
        .unserialized()
        .server_owned(pin!(User, base.session_epoch)),
    F::internal("release_notes_seen_version")
        .unserialized()
        .server_owned(pin!(User, base.release_notes_seen_version)),
    F::internal("terms_notice_version")
        .unserialized()
        .server_owned(pin!(User, base.terms_notice_version)),
    // The User list table is the members list (`users/members.rs`): users and the invites waiting
    // to become users, so its invite columns are declared here.
    F::column("oidc_linked_at", "Provider linked", G::Access)
        .server_owned(pin!(User, base.oidc_linked_at)),
    F::column("invite_status", "Status", G::Access).computed(),
    F::column("invite_url", "URL", G::Access).computed(),
    F::column("expires_at", "Expires", G::Access).computed(),
    F::column("invited_by", "Created By", G::Relationships).computed(),
    F::column("site_ids", "Sites", G::Relationships)
        .entity(&[Hop::parent(E::Site, "site_ids")])
        .many(),
    F::column("created_at", "Joined", G::Timestamps),
    UPDATED_AT,
];

const SHARE: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::new("is_enabled", "Enabled", G::Access).format(FieldFormat::Boolean),
    F::new("expires_at", "Expires", G::Access).format(TIMESTAMP),
    F::new("topology_id", "Topology", G::Relationships)
        .entity(&[Hop::parent(E::Topology, "topology_id")]),
    F::new("created_by", "Created By", G::Relationships)
        .entity(&[Hop::parent(E::User, "created_by")])
        .server_owned(pin!(Share, base.created_by)),
    SITE,
    F::internal("password"),
    F::internal("password_hash").unserialized(),
    F::internal("allowed_domains").many(),
    F::internal("options"),
    F::internal("enabled_views").many(),
];

const TOPOLOGY: &[F] = &[
    SITE,
    F::internal("options"),
    F::internal("id").immutable(pin!(Topology, id)),
    INTERNAL_CREATED_AT.immutable(pin!(Topology, created_at)),
    INTERNAL_UPDATED_AT.immutable(pin!(Topology, updated_at)),
];

const ORGANIZATION: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::internal("use_case"),
    // Billing fields are managed by the Stripe integration.
    F::internal("stripe_customer_id")
        .unserialized()
        .server_owned(pin!(Organization, base.stripe_customer_id)),
    F::internal("plan").server_owned(pin!(Organization, base.plan)),
    F::internal("plan_status").server_owned(pin!(Organization, base.plan_status)),
    F::internal("next_renewal_at").server_owned(pin!(Organization, base.next_renewal_at)),
    F::internal("setup")
        .many()
        .server_owned(pin!(Organization, base.setup)),
    // License state is server-managed. The key version is never serialized, so a PUT body always
    // carries the default and would otherwise reset it, reviving retired online keys.
    F::internal("license_paid_through").server_owned(pin!(Organization, base.license_paid_through)),
    F::internal("license_checkin_at").server_owned(pin!(Organization, base.license_checkin_at)),
    F::internal("license_key_version")
        .unserialized()
        .server_owned(pin!(Organization, base.license_key_version)),
    F::internal("license_key_issued_at")
        .unserialized()
        .server_owned(pin!(Organization, base.license_key_issued_at)),
    F::internal("license_key_type")
        .unserialized()
        .server_owned(pin!(Organization, base.license_key_type)),
    F::internal("license_server_version")
        .unserialized()
        .server_owned(pin!(Organization, base.license_server_version)),
    // The cached entitlement is written only by the license service.
    F::internal("license_entitlement")
        .unserialized()
        .server_owned(pin!(Organization, base.license_entitlement)),
    F::internal("license_entitlement_at")
        .unserialized()
        .server_owned(pin!(Organization, base.license_entitlement_at)),
    F::internal("notifications").unserialized(),
    F::internal("has_payment_method"),
    F::internal("bills_by_invoice"),
    F::internal("trial_end_date"),
    F::internal("last_paused_at"),
    F::internal("trial_extended_used"),
    F::internal("last_downgrade_at"),
    F::internal("last_downgrade_from_plan"),
    F::internal("last_discount_at"),
    F::internal("discount_save_offer_percent_off"),
    F::internal("discount_save_offer_active_until"),
    F::internal("air_gapped_key_current_until"),
    // On `OrganizationResponse`, derived from the org's daemons.
    F::internal("daemon_connected").computed(),
];

const SNAPSHOT: &[F] = &[
    F::new("taken_at", "Taken", G::Identity)
        .format(TIMESTAMP)
        .summary(),
    SITE,
    F::internal("created_by_user_id"),
];

const INVITE: &[F] = &[
    F::new("send_to", "Sent To", G::Identity).summary(),
    F::new("permissions", "Permissions", G::Access).metadata(Fx::Permissions),
    F::new("expires_at", "Expires", G::Access).format(TIMESTAMP),
    ORGANIZATION_ID,
    F::internal("site_ids").many(),
    F::internal("url"),
    F::internal("created_by"),
];

/// An id one entity holds for another, and where it sits on the entity's serialized form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityReference {
    pub target: E,
    /// Keys to walk from the entity to the objects holding the id. Empty when the entity holds it.
    pub embedded: Vec<&'static str>,
    /// Key on those objects holding the id or ids.
    pub key: &'static str,
}

impl E {
    /// Every id a request can set on this entity that names another entity. Leaves out the
    /// entity's own `site_id`, which the generic handlers check, and server-owned fields, which no
    /// request can set.
    pub fn references(&self) -> Vec<EntityReference> {
        let mut references: Vec<EntityReference> = Vec::new();
        for field in self.fields() {
            if field.computed || matches!(field.write, FieldWrite::ServerOwned(_)) {
                continue;
            }
            let mut embedded = Vec::new();
            for hop in field.path {
                match *hop {
                    Hop::Embedded { key } => embedded.push(key),
                    Hop::Children { .. } => break,
                    Hop::Parent { entity, key } => {
                        let reference = EntityReference {
                            target: entity,
                            embedded,
                            key,
                        };
                        let own_site = reference.embedded.is_empty() && key == "site_id";
                        if !own_site && !references.contains(&reference) {
                            references.push(reference);
                        }
                        break;
                    }
                }
            }
        }
        references
    }

    /// The fields this entity type is described by, in display order.
    pub fn fields(&self) -> &'static [F] {
        match self {
            E::Site => SITE_FIELDS,
            E::Vlan => VLAN,
            E::Subnet => SUBNET,
            E::Host => HOST,
            E::Service => SERVICE,
            E::IPAddress => IP_ADDRESS,
            E::Interface => INTERFACE,
            E::Port => PORT,
            E::Binding => BINDING,
            E::Discovery => DISCOVERY,
            E::Daemon => DAEMON,
            E::DaemonApiKey => DAEMON_API_KEY,
            E::Tag => TAG,
            E::TagGroup => TAG_GROUP,
            E::User => USER,
            E::UserApiKey => USER_API_KEY,
            E::Credential => CREDENTIAL,
            E::Organization => ORGANIZATION,
            E::Invite => INVITE,
            E::Topology => TOPOLOGY,
            E::Snapshot => SNAPSHOT,
            E::Share => SHARE,
            E::Dependency => DEPENDENCY,
        }
    }

    /// Fields that name one entity in text, tried in order: what a text-only output writes for
    /// an `Entity` field. `display_name` is only present where the server computes one (hosts in
    /// a topology or a host response, interfaces).
    pub fn display_fields(&self) -> &'static [&'static str] {
        match self {
            E::Host => &["display_name", "name"],
            E::Interface => &["display_name", "if_name", "if_descr"],
            E::IPAddress => &["ip_address"],
            E::Port => &["number"],
            E::Vlan => &["vlan_number"],
            E::User | E::Invite => &["email", "send_to"],
            E::Binding => &["id"],
            E::Snapshot => &["taken_at"],
            E::Topology => &["id"],
            E::Site
            | E::Subnet
            | E::Service
            | E::Discovery
            | E::Daemon
            | E::DaemonApiKey
            | E::Tag
            | E::TagGroup
            | E::UserApiKey
            | E::Credential
            | E::Organization
            | E::Share
            | E::Dependency => &["name"],
        }
    }
}
