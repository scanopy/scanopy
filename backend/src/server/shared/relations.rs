//! Every reference between stored rows, declared once.
//!
//! A [`Reference`] is a key that holds another entity's id. The field registry
//! (`shared/entity_fields.rs`) builds its `Hop::Parent` steps from these. A [`Relation`] is a
//! reference on one typed row type, with whether that row can exist without the other end. Storage
//! keys its set operations by a relation, the topology bundle's cascade follows the owners, and
//! the registry builds `Hop::Children` from one. Each row type lists its relations in
//! [`Storable::relations`](crate::server::shared::storage::traits::Storable::relations).
//!
//! `Ownership` matches the column's foreign key in the migrations: `ON DELETE CASCADE` is an
//! owner. Site, organization and snapshot scoping columns cascade too, but storage handles those
//! scopes itself, so they aren't listed as relations.

use crate::server::{
    bindings::r#impl::base::Binding,
    credentials::r#impl::junction::{HostCredential, SiteCredential},
    daemons::r#impl::interfaced_subnets::DaemonInterfacedSubnet,
    dependencies::dependency_members::DependencyMemberRecord,
    discovery_warnings::r#impl::base::DiscoveryWarningRecord,
    interface_neighbors::r#impl::base::{
        InterfaceNeighborCandidate, InterfaceNeighborHost, InterfaceNeighborInterface,
    },
    interfaces::r#impl::base::Interface,
    ip_addresses::r#impl::base::IPAddress,
    ports::r#impl::base::Port,
    services::r#impl::base::Service,
    shared::{
        entities::EntityDiscriminants as E,
        storage::relation::{Ids, Ownership, Reference, Relation},
    },
    site_assignments::r#impl::base::{Principal, SiteAssignment},
    tags::entity_tags::EntityTag,
    vlans::r#impl::subnet_vlans::SubnetVlanRecord,
};

// ---------------------------------------------------------------------------------------------
// References: a key and the entity whose id it holds.
// ---------------------------------------------------------------------------------------------

pub const SITE_ID: Reference = Reference::new(E::Site, "site_id");
pub const SITE_IDS: Reference = Reference::new(E::Site, "site_ids");
pub const ASSIGNED_SITE_IDS: Reference = Reference::new(E::Site, "assigned_site_ids");
pub const HOST_ID: Reference = Reference::new(E::Host, "host_id");
pub const SUBNET_ID: Reference = Reference::new(E::Subnet, "subnet_id");
pub const SUBNET_IDS: Reference = Reference::new(E::Subnet, "subnet_ids");
pub const SERVICE_ID: Reference = Reference::new(E::Service, "service_id");
pub const VIRTUALIZATION_SERVICE_ID: Reference =
    Reference::new(E::Service, "virtualization_service_id");
pub const INTERFACE_ID: Reference = Reference::new(E::Interface, "interface_id");
pub const INTERFACE_IDS: Reference = Reference::new(E::Interface, "interface_ids");
pub const NEIGHBOR_INTERFACE_ID: Reference = Reference::new(E::Interface, "neighbor_interface_id");
pub const NEIGHBOR_HOST_ID: Reference = Reference::new(E::Host, "neighbor_host_id");
pub const VIRTUALIZATION_INTERFACE_ID: Reference =
    Reference::new(E::Interface, "virtualization_interface_id");
pub const IP_ADDRESS_ID: Reference = Reference::new(E::IPAddress, "ip_address_id");
pub const IP_ADDRESS_IDS: Reference = Reference::new(E::IPAddress, "ip_address_ids");
pub const PORT_ID: Reference = Reference::new(E::Port, "port_id");
pub const VLAN_ID: Reference = Reference::new(E::Vlan, "vlan_id");
pub const NATIVE_VLAN_ID: Reference = Reference::new(E::Vlan, "native_vlan_id");
pub const VLAN_IDS: Reference = Reference::new(E::Vlan, "vlan_ids");
pub const CREDENTIAL_ID: Reference = Reference::new(E::Credential, "credential_id");
pub const CREDENTIAL_IDS: Reference = Reference::new(E::Credential, "credential_ids");
pub const LOGIN_CREDENTIAL_ID: Reference = Reference::new(E::Credential, "login_credential_id");
pub const USED_BY: Reference = Reference::new(E::Credential, "used_by");
pub const DAEMON_ID: Reference = Reference::new(E::Daemon, "daemon_id");
pub const DEPENDENCY_ID: Reference = Reference::new(E::Dependency, "dependency_id");
pub const DISCOVERY_ID: Reference = Reference::new(E::Discovery, "discovery_id");
pub const TAGS: Reference = Reference::new(E::Tag, "tags");
pub const TAG_ID: Reference = Reference::new(E::Tag, "tag_id");
pub const TAG_GROUP_ID: Reference = Reference::new(E::TagGroup, "tag_group_id");
pub const TOPOLOGY_ID: Reference = Reference::new(E::Topology, "topology_id");
pub const USER_ID: Reference = Reference::new(E::User, "user_id");
pub const API_KEY_ID: Reference = Reference::new(E::UserApiKey, "api_key_id");
pub const INVITE_ID: Reference = Reference::new(E::Invite, "invite_id");
pub const ORGANIZATION_ID: Reference = Reference::new(E::Organization, "organization_id");
pub const CREATED_BY: Reference = Reference::new(E::User, "created_by");

// ---------------------------------------------------------------------------------------------
// Relations: a reference on one row type, and whether the row needs the other end.
// ---------------------------------------------------------------------------------------------

pub const IP_ADDRESS_HOST: Relation<IPAddress> = Relation::to(
    HOST_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.host_id)),
);
pub const IP_ADDRESS_SUBNET: Relation<IPAddress> = Relation::to(
    SUBNET_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.subnet_id)),
);

pub const INTERFACE_HOST: Relation<Interface> = Relation::to(
    HOST_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.host_id)),
);

pub const SERVICE_HOST: Relation<Service> = Relation::to(
    HOST_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.host_id)),
);

pub const PORT_HOST: Relation<Port> = Relation::to(
    HOST_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.host_id)),
);

pub const BINDING_SERVICE: Relation<Binding> = Relation::to(
    SERVICE_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.service_id())),
);
/// Set on an IP address binding only.
pub const BINDING_IP_ADDRESS: Relation<Binding> = Relation::to(
    IP_ADDRESS_ID,
    Ownership::Owner,
    Ids::One(|r| r.ip_address_id()),
);
/// Set on a port binding only.
pub const BINDING_PORT: Relation<Binding> =
    Relation::to(PORT_ID, Ownership::Owner, Ids::One(|r| r.port_id()));

pub const NEIGHBOR_CANDIDATE_INTERFACE: Relation<InterfaceNeighborCandidate> = Relation::to(
    INTERFACE_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.interface_id)),
);

pub const NEIGHBOR_INTERFACE_LOCAL: Relation<InterfaceNeighborInterface> = Relation::to(
    INTERFACE_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.interface_id)),
);
pub const NEIGHBOR_INTERFACE_FAR: Relation<InterfaceNeighborInterface> = Relation::to(
    NEIGHBOR_INTERFACE_ID,
    Ownership::OwnerKeptWhenHidden,
    Ids::One(|r| Some(r.base.neighbor_interface_id)),
);

pub const NEIGHBOR_HOST_LOCAL: Relation<InterfaceNeighborHost> = Relation::to(
    INTERFACE_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.interface_id)),
);
pub const NEIGHBOR_HOST_FAR: Relation<InterfaceNeighborHost> = Relation::to(
    NEIGHBOR_HOST_ID,
    Ownership::OwnerKeptWhenHidden,
    Ids::One(|r| Some(r.base.neighbor_host_id)),
);

pub const SUBNET_VLAN_SUBNET: Relation<SubnetVlanRecord> = Relation::to(
    SUBNET_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.subnet_id)),
);
pub const SUBNET_VLAN_VLAN: Relation<SubnetVlanRecord> = Relation::to(
    VLAN_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.vlan_id)),
);

pub const INTERFACED_SUBNET_DAEMON: Relation<DaemonInterfacedSubnet> =
    Relation::to(DAEMON_ID, Ownership::Owner, Ids::One(|r| Some(r.daemon_id)));
pub const INTERFACED_SUBNET_SUBNET: Relation<DaemonInterfacedSubnet> =
    Relation::to(SUBNET_ID, Ownership::Owner, Ids::One(|r| Some(r.subnet_id)));

pub const SITE_CREDENTIAL_SITE: Relation<SiteCredential> =
    Relation::to(SITE_ID, Ownership::Owner, Ids::One(|r| Some(r.site_id)));
pub const SITE_CREDENTIAL_CREDENTIAL: Relation<SiteCredential> = Relation::to(
    CREDENTIAL_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.credential_id)),
);

pub const HOST_CREDENTIAL_HOST: Relation<HostCredential> =
    Relation::to(HOST_ID, Ownership::Owner, Ids::One(|r| Some(r.host_id)));
pub const HOST_CREDENTIAL_CREDENTIAL: Relation<HostCredential> = Relation::to(
    CREDENTIAL_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.credential_id)),
);

/// `entity_id`, with the entity's type in `entity_type`.
pub const ENTITY_TAG_ENTITY: Relation<EntityTag> = Relation::to_any_entity(
    "entity_id",
    Ownership::Owner,
    Ids::One(|r| Some(r.base.entity_id)),
);
pub const ENTITY_TAG_TAG: Relation<EntityTag> =
    Relation::to(TAG_ID, Ownership::Owner, Ids::One(|r| Some(r.base.tag_id)));

pub const DEPENDENCY_MEMBER_DEPENDENCY: Relation<DependencyMemberRecord> = Relation::to(
    DEPENDENCY_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.dependency_id)),
);

pub const SITE_ASSIGNMENT_ORGANIZATION: Relation<SiteAssignment> = Relation::to(
    ORGANIZATION_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.organization_id)),
);
pub const SITE_ASSIGNMENT_USER: Relation<SiteAssignment> = Relation::to(
    USER_ID,
    Ownership::Owner,
    Ids::One(|r| match r.base.principal {
        Principal::User(id) => Some(id),
        _ => None,
    }),
);
pub const SITE_ASSIGNMENT_API_KEY: Relation<SiteAssignment> = Relation::to(
    API_KEY_ID,
    Ownership::Owner,
    Ids::One(|r| match r.base.principal {
        Principal::ApiKey(id) => Some(id),
        _ => None,
    }),
);
pub const SITE_ASSIGNMENT_INVITE: Relation<SiteAssignment> = Relation::to(
    INVITE_ID,
    Ownership::Owner,
    Ids::One(|r| match r.base.principal {
        Principal::Invite(id) => Some(id),
        _ => None,
    }),
);
/// `None` for an All sites assignment.
pub const SITE_ASSIGNMENT_SITE: Relation<SiteAssignment> =
    Relation::to(SITE_ID, Ownership::Owner, Ids::One(|r| r.site_id()));

pub const DISCOVERY_WARNING_DISCOVERY: Relation<DiscoveryWarningRecord> = Relation::to(
    DISCOVERY_ID,
    Ownership::Owner,
    Ids::One(|r| Some(r.base.discovery_id)),
);

#[cfg(test)]
mod tests;
