use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::server::{
    bindings::r#impl::base::Binding,
    credentials::r#impl::base::Credential,
    daemon_api_keys::r#impl::base::DaemonApiKey,
    daemons::r#impl::{api::DaemonResponse, base::Daemon},
    dependencies::r#impl::base::Dependency,
    discovery::r#impl::base::Discovery,
    hosts::r#impl::api::HostResponse,
    interfaces::r#impl::base::Interface,
    invites::r#impl::base::Invite,
    ip_addresses::r#impl::base::IPAddress,
    ports::r#impl::base::Port,
    services::r#impl::base::Service,
    shared::entities::{Entity, EntityDiscriminants},
    shares::r#impl::base::Share,
    sites::r#impl::Site,
    snapshots::types::base::Snapshot,
    subnets::r#impl::nesting::SubnetResponse,
    tags::r#impl::base::Tag,
    topology::types::base::Topology,
    user_api_keys::r#impl::base::UserApiKey,
    users::r#impl::base::User,
    vlans::r#impl::base::Vlan,
};

/// Query parameters for the global search.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct GlobalSearchQuery {
    /// Free text. Case-insensitive substring match against each entity type's searchable fields.
    pub q: Option<String>,
    /// Only entities carrying every one of these tags. Repeat for several.
    pub tag_ids: Option<Vec<Uuid>>,
}

/// One match, in the shape its entity's list endpoint returns it (a host as a `HostResponse`,
/// a daemon as a `DaemonResponse`), keyed by entity type.
#[derive(Serialize, Debug, Clone, ToSchema)]
#[allow(clippy::large_enum_variant)]
pub enum SearchHit {
    Invite(Invite),
    Share(Share),
    Site(Site),
    DaemonApiKey(DaemonApiKey),
    UserApiKey(UserApiKey),
    User(User),
    Tag(Tag),
    Discovery(Discovery),
    Daemon(DaemonResponse),
    Host(HostResponse),
    Service(Service),
    Port(Port),
    Binding(Binding),
    IPAddress(IPAddress),
    Interface(Interface),
    Credential(Credential),
    Subnet(SubnetResponse),
    Vlan(Vlan),
    Dependency(Dependency),
    Topology(Topology),
    Snapshot(Snapshot),
}

/// One entity type's matches.
#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct GlobalSearchGroup {
    pub entity_type: EntityDiscriminants,
    pub items: Vec<SearchHit>,
}

/// Matches grouped by entity type, in registry order. Types with no matches are left out.
#[derive(Serialize, Debug, Clone, Default, ToSchema)]
pub struct GlobalSearchResponse {
    pub groups: Vec<GlobalSearchGroup>,
}

/// Back to the stored entity, dropping what the list derived (a host's children and title, a
/// daemon's version status, a subnet's usage). Exhaustive, so a new entity type needs a hit.
impl From<SearchHit> for Entity {
    fn from(hit: SearchHit) -> Self {
        match hit {
            SearchHit::Invite(e) => Entity::Invite(e),
            SearchHit::Share(e) => Entity::Share(e),
            SearchHit::Site(e) => Entity::Site(e),
            SearchHit::DaemonApiKey(e) => Entity::DaemonApiKey(e),
            SearchHit::UserApiKey(e) => Entity::UserApiKey(e),
            SearchHit::User(e) => Entity::User(e),
            SearchHit::Tag(e) => Entity::Tag(e),
            SearchHit::Discovery(e) => Entity::Discovery(e),
            SearchHit::Daemon(DaemonResponse {
                id,
                created_at,
                updated_at,
                base,
                version_status: _,
                interfaced_subnet_ids: _,
            }) => Entity::Daemon(Daemon {
                id,
                created_at,
                updated_at,
                base,
            }),
            SearchHit::Host(e) => Entity::Host(e.to_host()),
            SearchHit::Service(e) => Entity::Service(e),
            SearchHit::Port(e) => Entity::Port(e),
            SearchHit::Binding(e) => Entity::Binding(e),
            SearchHit::IPAddress(e) => Entity::IPAddress(e),
            SearchHit::Interface(e) => Entity::Interface(e),
            SearchHit::Credential(e) => Entity::Credential(e),
            SearchHit::Subnet(e) => Entity::Subnet(e.subnet),
            SearchHit::Vlan(e) => Entity::Vlan(e),
            SearchHit::Dependency(e) => Entity::Dependency(e),
            SearchHit::Topology(e) => Entity::Topology(e),
            SearchHit::Snapshot(e) => Entity::Snapshot(e),
        }
    }
}

/// A stored entity as a hit. Fails for the types whose list returns a derived shape (hosts,
/// daemons, subnets): those hits are built from that shape, never from the bare entity, which
/// would leave its derived fields empty. Exhaustive, so a new entity type needs a hit.
impl TryFrom<Entity> for SearchHit {
    type Error = anyhow::Error;

    fn try_from(entity: Entity) -> anyhow::Result<Self> {
        let derived = |entity_type: EntityDiscriminants| {
            Err(anyhow::anyhow!(
                "{entity_type} hits come from its list's response, not the stored entity"
            ))
        };
        Ok(match entity {
            Entity::Organization(_) => {
                return Err(anyhow::anyhow!("organizations are not searched"));
            }
            Entity::Invite(e) => SearchHit::Invite(e),
            Entity::Share(e) => SearchHit::Share(e),
            Entity::Site(e) => SearchHit::Site(e),
            Entity::DaemonApiKey(e) => SearchHit::DaemonApiKey(e),
            Entity::UserApiKey(e) => SearchHit::UserApiKey(e),
            Entity::User(e) => SearchHit::User(e),
            Entity::Tag(e) => SearchHit::Tag(e),
            Entity::Discovery(e) => SearchHit::Discovery(e),
            Entity::Daemon(_) => return derived(EntityDiscriminants::Daemon),
            Entity::Host(_) => return derived(EntityDiscriminants::Host),
            Entity::Service(e) => SearchHit::Service(e),
            Entity::Port(e) => SearchHit::Port(e),
            Entity::Binding(e) => SearchHit::Binding(e),
            Entity::IPAddress(e) => SearchHit::IPAddress(e),
            Entity::Interface(e) => SearchHit::Interface(e),
            Entity::Credential(e) => SearchHit::Credential(e),
            Entity::Subnet(_) => return derived(EntityDiscriminants::Subnet),
            Entity::Vlan(e) => SearchHit::Vlan(e),
            Entity::Dependency(e) => SearchHit::Dependency(e),
            Entity::Topology(e) => SearchHit::Topology(e),
            Entity::Snapshot(e) => SearchHit::Snapshot(e),
        })
    }
}
