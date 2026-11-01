//! The field registry: what each entity type is described by. See `types/entity_field.rs`.
//!
//! Ids keep the column names CSV export used before the registry, and the list tables' column
//! keys, so neither saved column preferences nor scripts reading the CSV lose a column. An
//! `Entity` field whose id ends in `_id` is a foreign key: CSV writes the related entity's id.

use crate::server::shared::entities::EntityDiscriminants as E;
use crate::server::shared::types::entity_field::{
    EntityField as F, FieldFormat, FieldGroup as G, Hop, MetadataFixture as Fx, TableColumn as C,
};

const SITE: F =
    F::new("site_id", "Site", G::Relationships).entity(&[Hop::parent(E::Site, "site_id")]);
const SOURCE: F = F::new("source", "Source", G::Discovery)
    .field("source.type")
    .metadata(Fx::EntitySources);
const DESCRIPTION: F = F::new("description", "Description", G::Identity).format(FieldFormat::Prose);
const TIMESTAMP: FieldFormat = FieldFormat::Timestamp;

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
    SOURCE,
    SITE,
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
        .searchable(),
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
        .searchable(),
    F::new("speed_bps", "Speed", G::Network)
        .format(FieldFormat::BitsPerSecond)
        .summary(),
    F::new("oper_status", "Status", G::Status)
        .metadata(Fx::IfOperStatuses)
        .summary(),
    F::new("admin_status", "Admin Status", G::Status),
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
    F::new("if_name", "ifName", G::Identity).searchable(),
    F::new("if_descr", "ifDescr", G::Identity).searchable(),
    F::new("if_alias", "Alias", G::Identity).searchable(),
    F::new("if_type", "ifType", G::Identity),
    F::new("if_index", "Index", G::Identity).format(FieldFormat::Number),
    F::new("ip_configured", "IP Configured", G::Network).format(FieldFormat::Boolean),
    F::new("fdb_macs", "Forwarding Table MACs", G::Network)
        .source("fdb_macs_source")
        .format(FieldFormat::Identifier)
        .many(),
    F::new("interface_source", "Source", G::Identity).format(FieldFormat::AttributeSource),
    SITE,
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
    F::new("containerized_by", "Containerized By", G::Virtualization)
        .entity(&[Hop::parent(E::Service, "virtualization_service_id")]),
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
    SOURCE,
    SITE,
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
    F::new("name", "Name", G::Identity).summary().searchable(),
    F::new("cidr", "CIDR", G::Network)
        .source("cidr_source")
        .format(FieldFormat::Identifier)
        .summary()
        .searchable(),
    F::new("subnet_type", "Subnet Type", G::Identity)
        .metadata(Fx::SubnetTypes)
        .summary(),
    DESCRIPTION,
    F::new("vlan_ids", "VLANs", G::Network)
        .entity(&[Hop::parent(E::Vlan, "vlan_ids")])
        .many()
        .summary(),
    F::new("managed_by", "Managed By", G::Virtualization)
        .entity(&[Hop::parent(E::Service, "virtualization_service_id")]),
    SOURCE,
    SITE,
];

const VLAN: &[F] = &[
    F::new("vlan_number", "VLAN Number", G::Network)
        .format(FieldFormat::Number)
        .summary()
        .searchable(),
    F::new("name", "Name", G::Identity).summary().searchable(),
    DESCRIPTION.summary(),
    F::new("subnet_ids", "Subnets", G::Network)
        .entity(&[Hop::parent(E::Subnet, "subnet_ids")])
        .many(),
    F::new("interface_ids", "Interfaces", G::Network)
        .entity(&[Hop::parent(E::Interface, "interface_ids")])
        .many(),
    SOURCE,
    SITE,
];

const DEPENDENCY: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    DESCRIPTION.summary(),
    F::new("dependency_type", "Dependency Type", G::Identity)
        .metadata(Fx::DependencyTypes)
        .summary(),
    F::new("color", "Color", G::Identity),
    SOURCE,
    SITE,
];

const TAG: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    DESCRIPTION.summary(),
    F::new("color", "Color", G::Identity),
    F::new("tag_group", "Tag Group", G::Relationships)
        .entity(&[Hop::parent(E::TagGroup, "tag_group_id")]),
    F::new("icon", "Icon", G::Identity),
];

const TAG_GROUP: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    DESCRIPTION.tooltip(),
    F::new("group_type", "Group Type", G::Identity),
    F::new("entity_types", "Applies To", G::Configuration).many(),
];

const DAEMON: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::new("mode", "Daemon Mode", G::Configuration),
    F::new("version", "Version", G::Status).summary(),
    F::new("last_seen", "Last Seen", G::Status).format(TIMESTAMP),
    F::new("url", "URL", G::Configuration),
    F::new("host_id", "Host", G::Relationships).entity(&[Hop::parent(E::Host, "host_id")]),
    F::new("maintainer", "Maintainer", G::Relationships).entity(&[Hop::parent(E::User, "user_id")]),
    SITE,
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
];

const DAEMON_API_KEY: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::new("is_enabled", "Enabled", G::Access)
        .format(FieldFormat::Boolean)
        .tooltip(),
    F::new("last_used", "Last Used", G::Access)
        .format(TIMESTAMP)
        .tooltip(),
    F::new("expires_at", "Expires", G::Access).format(TIMESTAMP),
    F::new("daemon_id", "Daemon", G::Relationships).entity(&[Hop::parent(E::Daemon, "daemon_id")]),
    SITE,
];

const USER_API_KEY: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::new("permissions", "Permissions", G::Access).metadata(Fx::Permissions),
    F::new("is_enabled", "Enabled", G::Access)
        .format(FieldFormat::Boolean)
        .tooltip(),
    F::new("last_used", "Last Used", G::Access)
        .format(TIMESTAMP)
        .tooltip(),
    F::new("expires_at", "Expires", G::Access).format(TIMESTAMP),
    F::new("user_id", "User", G::Relationships).entity(&[Hop::parent(E::User, "user_id")]),
];

const CREDENTIAL: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::new("credential_type", "Credential Type", G::Identity)
        .field("credential_type.type")
        .metadata(Fx::CredentialTypes)
        .summary(),
    DESCRIPTION.tooltip(),
    F::new("daemon_os", "Daemon OS", G::Configuration),
];

const SITE_FIELDS: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::new("stale_after_hours", "Stale After (hours)", G::Configuration)
        .format(FieldFormat::Number),
];

const USER: &[F] = &[
    F::new("email", "Email", G::Identity).summary(),
    F::new("permissions", "Permissions", G::Access)
        .metadata(Fx::Permissions)
        .summary(),
    F::new("email_verified", "Email Verified", G::Access).format(FieldFormat::Boolean),
    F::new("oidc_provider", "OIDC Provider", G::Access),
];

const SHARE: &[F] = &[
    F::new("name", "Name", G::Identity).summary(),
    F::new("is_enabled", "Enabled", G::Access).format(FieldFormat::Boolean),
    F::new("expires_at", "Expires", G::Access).format(TIMESTAMP),
    F::new("topology_id", "Topology", G::Relationships)
        .entity(&[Hop::parent(E::Topology, "topology_id")]),
    F::new("created_by", "Created By", G::Relationships)
        .entity(&[Hop::parent(E::User, "created_by")]),
    SITE,
];

const TOPOLOGY: &[F] = &[SITE];
const ORGANIZATION: &[F] = &[F::new("name", "Name", G::Identity).summary()];
const SNAPSHOT: &[F] = &[
    F::new("taken_at", "Taken", G::Identity)
        .format(TIMESTAMP)
        .summary(),
    SITE,
];
const INVITE: &[F] = &[
    F::new("send_to", "Sent To", G::Identity).summary(),
    F::new("permissions", "Permissions", G::Access).metadata(Fx::Permissions),
    F::new("expires_at", "Expires", G::Access).format(TIMESTAMP),
];

// List-table columns that aren't fields; see `TableColumn`.
const CREATED_AT: C = C::new("created_at", G::Timestamps);
const UPDATED_AT: C = C::new("updated_at", G::Timestamps);
const LAST_SEEN_AT: C = C::new("last_seen_at", G::Discovery);
const FIRST_FOUND_BY: C = C::new("first_found_by", G::Discovery);
const LAST_FOUND_BY: C = C::new("last_found_by", G::Discovery);

const HOST_COLUMNS: &[C] = &[
    // Read from `HostResponse`, not the stored `Host`, which is why it is a column, not a field.
    C::new("device_type", G::Device),
    C::new("mac_address", G::Network),
    C::new("credentials", G::Relationships),
    LAST_SEEN_AT,
    FIRST_FOUND_BY,
    LAST_FOUND_BY,
    CREATED_AT,
    UPDATED_AT,
];

const SERVICE_COLUMNS: &[C] = &[
    C::new("category", G::Identity),
    C::new("match_confidence", G::Discovery),
    LAST_SEEN_AT,
    FIRST_FOUND_BY,
    LAST_FOUND_BY,
    CREATED_AT,
    UPDATED_AT,
];

const SUBNET_COLUMNS: &[C] = &[
    C::new("utilization", G::Network),
    LAST_SEEN_AT,
    FIRST_FOUND_BY,
    LAST_FOUND_BY,
    CREATED_AT,
    UPDATED_AT,
];

const VLAN_COLUMNS: &[C] = &[
    LAST_SEEN_AT,
    FIRST_FOUND_BY,
    LAST_FOUND_BY,
    CREATED_AT,
    UPDATED_AT,
];

const DAEMON_COLUMNS: &[C] = &[
    C::new("status", G::Status),
    C::new("os", G::Device),
    C::new("interfaced_subnet_ids", G::Network),
    CREATED_AT,
    UPDATED_AT,
];

const DISCOVERY_COLUMNS: &[C] = &[
    C::new("last_run", G::Status),
    C::new("scan_count", G::Status),
    C::new("legacy", G::Status),
    C::new("progress", G::Status),
    C::new("phase", G::Status),
    C::new("started_at", G::Status),
    C::new("finished_at", G::Status),
    C::new("duration", G::Status),
    C::new("warnings", G::Status),
    C::new("schedule", G::Configuration),
    C::new("force_full_scan", G::Configuration),
    C::new("scan_mode", G::Configuration),
    CREATED_AT,
    UPDATED_AT,
];

const CREDENTIAL_COLUMNS: &[C] = &[
    C::new("integration", G::Identity),
    C::new("beta", G::Identity),
    C::new("unofficial_api", G::Identity),
    C::new("target", G::Relationships),
    C::new("assigned_sites", G::Relationships),
    C::new("assigned_hosts", G::Relationships),
    CREATED_AT,
    UPDATED_AT,
];

const SITE_COLUMNS: &[C] = &[
    C::new("effective_stale_after_hours", G::Configuration),
    C::new("vlans", G::Network),
    C::new("subnets", G::Network),
    C::new("daemons", G::Relationships),
    C::new("credentials", G::Relationships),
    CREATED_AT,
    UPDATED_AT,
];

const TAG_GROUP_COLUMNS: &[C] = &[C::new("ordered", G::Configuration), CREATED_AT, UPDATED_AT];

const USER_API_KEY_COLUMNS: &[C] = &[C::new("site_ids", G::Relationships), CREATED_AT, UPDATED_AT];

// The User list table is the members list (`users/members.rs`): users and the invites waiting to
// become users. Its columns that aren't user fields, the invite ones included, are declared here.
const USER_COLUMNS: &[C] = &[
    C::new("oidc_linked_at", G::Access),
    C::new("invite_status", G::Access),
    C::new("invite_url", G::Access),
    C::new("expires_at", G::Access),
    C::new("invited_by", G::Relationships),
    C::new("site_ids", G::Relationships),
    CREATED_AT,
    UPDATED_AT,
];

const TIMESTAMP_COLUMNS: &[C] = &[CREATED_AT, UPDATED_AT];

impl E {
    /// The list table's columns that aren't fields, so the column menu can group them. Empty for
    /// types without a list table.
    pub fn table_columns(&self) -> &'static [C] {
        match self {
            E::Host => HOST_COLUMNS,
            E::Service => SERVICE_COLUMNS,
            E::Subnet => SUBNET_COLUMNS,
            E::Vlan => VLAN_COLUMNS,
            E::Daemon => DAEMON_COLUMNS,
            E::Discovery => DISCOVERY_COLUMNS,
            E::Credential => CREDENTIAL_COLUMNS,
            E::Site => SITE_COLUMNS,
            E::TagGroup => TAG_GROUP_COLUMNS,
            E::UserApiKey => USER_API_KEY_COLUMNS,
            E::User => USER_COLUMNS,
            E::Tag | E::DaemonApiKey => TIMESTAMP_COLUMNS,
            E::IPAddress
            | E::Interface
            | E::Port
            | E::Binding
            | E::Organization
            | E::Invite
            | E::Topology
            | E::Snapshot
            | E::Share
            | E::Dependency => &[],
        }
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
