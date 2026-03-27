use crate::server::bindings::r#impl::base::Binding;
use crate::server::credentials::r#impl::base::Credential;
use crate::server::if_entries::r#impl::base::IfEntry;
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::invites::r#impl::base::Invite;
use crate::server::ports::r#impl::base::Port;
use crate::server::services::r#impl::base::Service;
use crate::server::shares::r#impl::base::Share;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::topology::types::base::Topology;
use crate::server::{groups::r#impl::base::Group, tags::r#impl::base::Tag};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumDiscriminants, EnumIter, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::{
    daemon_api_keys::r#impl::base::DaemonApiKey,
    daemons::r#impl::base::Daemon,
    discovery::r#impl::base::Discovery,
    hosts::r#impl::base::Host,
    networks::r#impl::Network,
    organizations::r#impl::base::Organization,
    shared::types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId},
    },
    user_api_keys::r#impl::base::UserApiKey,
    users::r#impl::base::User,
};

// Trait use to determine whether a given property change on an entity should trigger a rebuild of topology
pub trait ChangeTriggersTopologyStaleness<T> {
    fn triggers_staleness(&self, _other: Option<T>) -> bool;
}

/// Single source of truth for which entity types support tagging.
/// Used by the Entity trait's default is_taggable() implementation and tag handlers.
pub fn is_entity_taggable(entity_type: EntityDiscriminants) -> bool {
    matches!(
        entity_type,
        EntityDiscriminants::Host
            | EntityDiscriminants::Service
            | EntityDiscriminants::Subnet
            | EntityDiscriminants::Group
            | EntityDiscriminants::Network
            | EntityDiscriminants::Discovery
            | EntityDiscriminants::Daemon
            | EntityDiscriminants::DaemonApiKey
            | EntityDiscriminants::UserApiKey
            | EntityDiscriminants::Credential
    )
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    EnumDiscriminants,
    IntoStaticStr,
    Serialize,
    Deserialize,
    Display,
    Default,
)]
#[strum_discriminants(derive(
    Display,
    Hash,
    EnumIter,
    IntoStaticStr,
    Serialize,
    Deserialize,
    ToSchema,
    Default
))]
pub enum Entity {
    Organization(Organization),
    Invite(Invite),
    Share(Share),
    Network(Network),
    DaemonApiKey(DaemonApiKey),
    UserApiKey(UserApiKey),
    User(User),
    Tag(Tag),

    Discovery(Discovery),
    Daemon(Daemon),

    Host(Host),
    Service(Service),
    Port(Port),
    Binding(Binding),
    Interface(Interface),
    IfEntry(IfEntry),

    Credential(Credential),
    Subnet(Subnet),
    Group(Group),
    Topology(Box<Topology>),

    #[default]
    #[strum_discriminants(default)]
    Unknown,
}

impl HasId for EntityDiscriminants {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for EntityDiscriminants {
    fn color(&self) -> Color {
        match self {
            EntityDiscriminants::Organization => Color::Blue,
            EntityDiscriminants::Network => Color::Gray,
            EntityDiscriminants::Daemon => Color::Green,
            EntityDiscriminants::Discovery => Color::Green,
            EntityDiscriminants::DaemonApiKey => Color::Yellow,
            EntityDiscriminants::UserApiKey => Color::Yellow,
            EntityDiscriminants::Credential => Color::Pink,
            EntityDiscriminants::User => Color::Blue,
            EntityDiscriminants::Invite => Color::Green,
            EntityDiscriminants::Share => Color::Teal,
            EntityDiscriminants::Tag => Color::Yellow,

            EntityDiscriminants::Host => Color::Blue,
            EntityDiscriminants::Service => Color::Purple,
            EntityDiscriminants::Interface => Color::Cyan,
            EntityDiscriminants::Port => Color::Cyan,
            EntityDiscriminants::Binding => Color::Purple,
            EntityDiscriminants::IfEntry => Color::Teal,

            EntityDiscriminants::Subnet => Color::Orange,
            EntityDiscriminants::Group => Color::Rose,
            EntityDiscriminants::Topology => Color::Pink,

            EntityDiscriminants::Unknown => Color::Gray,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            EntityDiscriminants::Organization => Icon::Building,
            EntityDiscriminants::Network => Icon::Globe,
            EntityDiscriminants::User => Icon::User,
            EntityDiscriminants::Tag => Icon::Tag,
            EntityDiscriminants::Invite => Icon::UserPlus,
            EntityDiscriminants::Share => Icon::Share2,
            EntityDiscriminants::DaemonApiKey => Icon::Key,
            EntityDiscriminants::UserApiKey => Icon::Key,
            EntityDiscriminants::Daemon => Icon::SatelliteDish,
            EntityDiscriminants::Discovery => Icon::Radar,
            EntityDiscriminants::Host => Icon::Server,
            EntityDiscriminants::Service => Icon::Layers,
            EntityDiscriminants::Interface => Icon::Binary,
            EntityDiscriminants::Port => Icon::EthernetPort,
            EntityDiscriminants::Binding => Icon::Link,
            EntityDiscriminants::IfEntry => Icon::Cable,
            EntityDiscriminants::Credential => Icon::Asterisk,
            EntityDiscriminants::Subnet => Icon::Network,
            EntityDiscriminants::Group => Icon::Group,
            EntityDiscriminants::Topology => Icon::ChartBarStacked,

            EntityDiscriminants::Unknown => Icon::CircleQuestionMark,
        }
    }
}

impl From<Organization> for Entity {
    fn from(value: Organization) -> Self {
        Self::Organization(value)
    }
}

impl From<Invite> for Entity {
    fn from(value: Invite) -> Self {
        Self::Invite(value)
    }
}

impl From<Share> for Entity {
    fn from(value: Share) -> Self {
        Self::Share(value)
    }
}

impl From<Network> for Entity {
    fn from(value: Network) -> Self {
        Self::Network(value)
    }
}

impl From<DaemonApiKey> for Entity {
    fn from(value: DaemonApiKey) -> Self {
        Self::DaemonApiKey(value)
    }
}

impl From<UserApiKey> for Entity {
    fn from(value: UserApiKey) -> Self {
        Self::UserApiKey(value)
    }
}

impl From<User> for Entity {
    fn from(value: User) -> Self {
        Self::User(value)
    }
}

impl From<Discovery> for Entity {
    fn from(value: Discovery) -> Self {
        Self::Discovery(value)
    }
}

impl From<Daemon> for Entity {
    fn from(value: Daemon) -> Self {
        Self::Daemon(value)
    }
}

impl From<Host> for Entity {
    fn from(value: Host) -> Self {
        Self::Host(value)
    }
}

impl From<Service> for Entity {
    fn from(value: Service) -> Self {
        Self::Service(value)
    }
}

impl From<Port> for Entity {
    fn from(value: Port) -> Self {
        Self::Port(value)
    }
}

impl From<Binding> for Entity {
    fn from(value: Binding) -> Self {
        Self::Binding(value)
    }
}

impl From<Interface> for Entity {
    fn from(value: Interface) -> Self {
        Self::Interface(value)
    }
}

impl From<Subnet> for Entity {
    fn from(value: Subnet) -> Self {
        Self::Subnet(value)
    }
}

impl From<Group> for Entity {
    fn from(value: Group) -> Self {
        Self::Group(value)
    }
}

impl From<Topology> for Entity {
    fn from(value: Topology) -> Self {
        Self::Topology(Box::new(value))
    }
}

impl From<Tag> for Entity {
    fn from(value: Tag) -> Self {
        Self::Tag(value)
    }
}

impl From<Credential> for Entity {
    fn from(value: Credential) -> Self {
        Self::Credential(value)
    }
}

impl From<IfEntry> for Entity {
    fn from(value: IfEntry) -> Self {
        Self::IfEntry(value)
    }
}

impl From<EntityDiscriminants> for Entity {
    fn from(d: EntityDiscriminants) -> Self {
        match d {
            EntityDiscriminants::Host => Entity::Host(Host::default()),
            EntityDiscriminants::Service => Entity::Service(Service::default()),
            EntityDiscriminants::Subnet => Entity::Subnet(Subnet::default()),
            EntityDiscriminants::Group => Entity::Group(Group::default()),
            EntityDiscriminants::Port => Entity::Port(Port::default()),
            EntityDiscriminants::Interface => Entity::Interface(Interface::default()),
            EntityDiscriminants::Binding => Entity::Binding(Binding::default()),
            EntityDiscriminants::IfEntry => Entity::IfEntry(IfEntry::default()),
            EntityDiscriminants::Tag => Entity::Tag(Tag::default()),
            EntityDiscriminants::Network => Entity::Network(Network::default()),
            EntityDiscriminants::Organization => Entity::Organization(Organization::default()),
            EntityDiscriminants::User => Entity::User(User::default()),
            EntityDiscriminants::Invite => Entity::Invite(Invite::default()),
            EntityDiscriminants::Share => Entity::Share(Share::default()),
            EntityDiscriminants::Discovery => Entity::Discovery(Discovery::default()),
            EntityDiscriminants::Daemon => Entity::Daemon(Daemon::default()),
            EntityDiscriminants::DaemonApiKey => Entity::DaemonApiKey(DaemonApiKey::default()),
            EntityDiscriminants::UserApiKey => Entity::UserApiKey(UserApiKey::default()),
            EntityDiscriminants::Credential => Entity::Credential(Credential::default()),
            EntityDiscriminants::Topology => Entity::Topology(Box::default()),
            EntityDiscriminants::Unknown => Entity::Unknown,
        }
    }
}
