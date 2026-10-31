use anyhow::Result;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Pool, Postgres};
use std::str::FromStr;
use std::sync::Arc;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;

use crate::server::credentials::r#impl::junction::{HostCredential, SiteCredential};
use crate::server::daemons::r#impl::interfaced_subnets::DaemonInterfacedSubnet;
use crate::server::dependencies::dependency_members::DependencyMemberRecord;
use crate::server::discovery_warnings::r#impl::base::DiscoveryWarningRecord;
use crate::server::interface_neighbors::r#impl::base::{
    InterfaceNeighborCandidate, InterfaceNeighborHost, InterfaceNeighborInterface,
};
use crate::server::license::instances::{LicenseInstance, ServerInstance};
use crate::server::release_notes::storage::ReleaseAnnouncement;
use crate::server::site_assignments::r#impl::base::SiteAssignment;
use crate::server::tags::entity_tags::EntityTag;
use crate::server::tags::groups::base::TagGroup;
use crate::server::vlans::r#impl::subnet_vlans::SubnetVlanRecord;
use crate::server::{
    bindings::r#impl::base::Binding,
    credentials::r#impl::base::Credential,
    daemon_api_keys::r#impl::base::DaemonApiKey,
    daemons::r#impl::base::Daemon,
    dependencies::r#impl::base::Dependency,
    discovery::r#impl::base::Discovery,
    hosts::r#impl::base::Host,
    interfaces::r#impl::base::Interface,
    invites::r#impl::base::Invite,
    ip_addresses::r#impl::base::IPAddress,
    organizations::r#impl::base::Organization,
    ports::r#impl::base::Port,
    services::r#impl::base::Service,
    shared::storage::{child::GenericChildStorage, generic::GenericPostgresStorage},
    shares::r#impl::base::Share,
    sites::r#impl::Site,
    snapshots::types::base::Snapshot,
    subnets::r#impl::base::Subnet,
    tags::r#impl::base::Tag,
    topology::types::base::Topology,
    user_api_keys::r#impl::base::UserApiKey,
    users::r#impl::base::User,
    vlans::r#impl::base::Vlan,
};

/// Every storage the server has, built once from one pool. Services take the storages they own
/// from here; nothing outside this module can reach the pool, so nothing can build a storage of
/// its own.
pub struct StorageFactory {
    pool: PgPool,
    pub sessions: SessionManagerLayer<PostgresStore>,
    pub daemon_api_keys: Arc<GenericPostgresStorage<DaemonApiKey>>,
    pub user_api_keys: Arc<GenericPostgresStorage<UserApiKey>>,
    pub users: Arc<GenericPostgresStorage<User>>,
    pub sites: Arc<GenericPostgresStorage<Site>>,
    pub hosts: Arc<GenericPostgresStorage<Host>>,
    pub ip_addresses: Arc<GenericPostgresStorage<IPAddress>>,
    pub dependencies: Arc<GenericPostgresStorage<Dependency>>,
    pub daemons: Arc<GenericPostgresStorage<Daemon>>,
    pub subnets: Arc<GenericPostgresStorage<Subnet>>,
    pub services: Arc<GenericPostgresStorage<Service>>,
    pub organizations: Arc<GenericPostgresStorage<Organization>>,
    pub invites: Arc<GenericPostgresStorage<Invite>>,
    pub shares: Arc<GenericPostgresStorage<Share>>,
    pub discovery: Arc<GenericPostgresStorage<Discovery>>,
    pub topologies: Arc<GenericPostgresStorage<Topology>>,
    pub snapshots: Arc<GenericPostgresStorage<Snapshot>>,
    pub tags: Arc<GenericPostgresStorage<Tag>>,
    pub tag_groups: Arc<GenericPostgresStorage<TagGroup>>,
    pub release_announcements: Arc<GenericPostgresStorage<ReleaseAnnouncement>>,
    pub server_instance: Arc<GenericPostgresStorage<ServerInstance>>,
    pub license_instances: Arc<GenericPostgresStorage<LicenseInstance>>,
    pub ports: Arc<GenericPostgresStorage<Port>>,
    pub bindings: Arc<GenericPostgresStorage<Binding>>,
    pub credentials: Arc<GenericPostgresStorage<Credential>>,
    pub interfaces: Arc<GenericPostgresStorage<Interface>>,
    pub vlans: Arc<GenericPostgresStorage<Vlan>>,
    // Rows stored under a parent: junction tables, and the non-entity tables a parent owns
    pub entity_tags: Arc<GenericChildStorage<EntityTag>>,
    pub site_assignments: Arc<GenericChildStorage<SiteAssignment>>,
    pub dependency_members: Arc<GenericChildStorage<DependencyMemberRecord>>,
    pub subnet_vlan: Arc<GenericChildStorage<SubnetVlanRecord>>,
    pub site_credentials: Arc<GenericChildStorage<SiteCredential>>,
    pub host_credentials: Arc<GenericChildStorage<HostCredential>>,
    pub daemon_interfaced_subnets: Arc<GenericChildStorage<DaemonInterfacedSubnet>>,
    pub interface_neighbor_candidates: Arc<GenericChildStorage<InterfaceNeighborCandidate>>,
    pub interface_neighbor_interfaces: Arc<GenericChildStorage<InterfaceNeighborInterface>>,
    pub interface_neighbor_hosts: Arc<GenericChildStorage<InterfaceNeighborHost>>,
    pub discovery_warnings: Arc<GenericChildStorage<DiscoveryWarningRecord>>,
}

pub async fn create_session_store(
    db_pool: Pool<Postgres>,
    use_secure: bool,
) -> Result<SessionManagerLayer<PostgresStore>> {
    let session_store = PostgresStore::new(db_pool.clone());

    session_store.migrate().await?;

    Ok(SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(7)))
        .with_name("session_id")
        .with_secure(use_secure)
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax))
}

impl StorageFactory {
    pub async fn new(database_url: &str, use_secure_session_cookies: bool) -> Result<Self> {
        let connect_options = PgConnectOptions::from_str(database_url)?.statement_cache_capacity(0);
        let pool = PgPoolOptions::new().connect_with(connect_options).await?;

        crate::server::shared::storage::migration_runner::apply_embedded_migrations(&pool).await?;

        let sessions = create_session_store(pool.clone(), use_secure_session_cookies).await?;

        Ok(Self {
            pool: pool.clone(),
            sessions,
            discovery: Arc::new(GenericPostgresStorage::new(pool.clone())),
            organizations: Arc::new(GenericPostgresStorage::new(pool.clone())),
            invites: Arc::new(GenericPostgresStorage::new(pool.clone())),
            shares: Arc::new(GenericPostgresStorage::new(pool.clone())),
            daemon_api_keys: Arc::new(GenericPostgresStorage::new(pool.clone())),
            user_api_keys: Arc::new(GenericPostgresStorage::new(pool.clone())),
            users: Arc::new(GenericPostgresStorage::new(pool.clone())),
            sites: Arc::new(GenericPostgresStorage::new(pool.clone())),
            hosts: Arc::new(GenericPostgresStorage::new(pool.clone())),
            ip_addresses: Arc::new(GenericPostgresStorage::new(pool.clone())),
            dependencies: Arc::new(GenericPostgresStorage::new(pool.clone())),
            daemons: Arc::new(GenericPostgresStorage::new(pool.clone())),
            subnets: Arc::new(GenericPostgresStorage::new(pool.clone())),
            services: Arc::new(GenericPostgresStorage::new(pool.clone())),
            topologies: Arc::new(GenericPostgresStorage::new(pool.clone())),
            snapshots: Arc::new(GenericPostgresStorage::new(pool.clone())),
            tags: Arc::new(GenericPostgresStorage::new(pool.clone())),
            tag_groups: Arc::new(GenericPostgresStorage::new(pool.clone())),
            release_announcements: Arc::new(GenericPostgresStorage::new(pool.clone())),
            server_instance: Arc::new(GenericPostgresStorage::new(pool.clone())),
            license_instances: Arc::new(GenericPostgresStorage::new(pool.clone())),
            ports: Arc::new(GenericPostgresStorage::new(pool.clone())),
            bindings: Arc::new(GenericPostgresStorage::new(pool.clone())),
            credentials: Arc::new(GenericPostgresStorage::new(pool.clone())),
            interfaces: Arc::new(GenericPostgresStorage::new(pool.clone())),
            vlans: Arc::new(GenericPostgresStorage::new(pool.clone())),
            entity_tags: Arc::new(GenericChildStorage::new(pool.clone())),
            site_assignments: Arc::new(GenericChildStorage::new(pool.clone())),
            dependency_members: Arc::new(GenericChildStorage::new(pool.clone())),
            subnet_vlan: Arc::new(GenericChildStorage::new(pool.clone())),
            site_credentials: Arc::new(GenericChildStorage::new(pool.clone())),
            host_credentials: Arc::new(GenericChildStorage::new(pool.clone())),
            daemon_interfaced_subnets: Arc::new(GenericChildStorage::new(pool.clone())),
            interface_neighbor_candidates: Arc::new(GenericChildStorage::new(pool.clone())),
            interface_neighbor_interfaces: Arc::new(GenericChildStorage::new(pool.clone())),
            interface_neighbor_hosts: Arc::new(GenericChildStorage::new(pool.clone())),
            discovery_warnings: Arc::new(GenericChildStorage::new(pool.clone())),
        })
    }
}

impl StorageFactory {
    /// The pool and session layer, for the app state once the services are built. Taking
    /// `self` keeps the pool out of reach of `ServiceFactory::new`, which only borrows.
    pub fn into_parts(self) -> (PgPool, SessionManagerLayer<PostgresStore>) {
        (self.pool, self.sessions)
    }

    /// The pool, for tests that seed or read rows directly.
    #[cfg(test)]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
