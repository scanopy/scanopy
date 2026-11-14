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
    bindings::r#impl::base::Binding, credentials::r#impl::base::Credential,
    daemon_api_keys::r#impl::base::DaemonApiKey, daemons::r#impl::base::Daemon,
    dependencies::r#impl::base::Dependency, discovery::r#impl::base::Discovery,
    hosts::r#impl::base::Host, interfaces::r#impl::base::Interface, invites::r#impl::base::Invite,
    ip_addresses::r#impl::base::IPAddress, organizations::r#impl::base::Organization,
    ports::r#impl::base::Port, services::r#impl::base::Service,
    shared::storage::generic::GenericPostgresStorage, shares::r#impl::base::Share,
    sites::r#impl::Site, snapshots::types::base::Snapshot, subnets::r#impl::base::Subnet,
    tags::r#impl::base::Tag, topology::types::base::Topology,
    user_api_keys::r#impl::base::UserApiKey, users::r#impl::base::User, vlans::r#impl::base::Vlan,
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
    pub entity_tags: Arc<GenericPostgresStorage<EntityTag>>,
    pub site_assignments: Arc<GenericPostgresStorage<SiteAssignment>>,
    pub dependency_members: Arc<GenericPostgresStorage<DependencyMemberRecord>>,
    pub subnet_vlan: Arc<GenericPostgresStorage<SubnetVlanRecord>>,
    pub site_credentials: Arc<GenericPostgresStorage<SiteCredential>>,
    pub host_credentials: Arc<GenericPostgresStorage<HostCredential>>,
    pub daemon_interfaced_subnets: Arc<GenericPostgresStorage<DaemonInterfacedSubnet>>,
    pub interface_neighbor_candidates: Arc<GenericPostgresStorage<InterfaceNeighborCandidate>>,
    pub interface_neighbor_interfaces: Arc<GenericPostgresStorage<InterfaceNeighborInterface>>,
    pub interface_neighbor_hosts: Arc<GenericPostgresStorage<InterfaceNeighborHost>>,
    pub discovery_warnings: Arc<GenericPostgresStorage<DiscoveryWarningRecord>>,
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

/// sqlx gives each persistent query its own named prepared statement and closes it only when
/// the statement cache evicts it, so the cache stays at sqlx's bounded default. With the cache
/// off, every query left a prepared statement on its connection until the connection retired,
/// and a few thousand hosts of discovery grew Postgres by gigabytes.
///
/// Every query stays persistent (named). Prod reaches Postgres through Neon's transaction-mode
/// PgBouncer, which re-prepares a named statement on whichever backend runs it. An unnamed
/// statement is parsed in one round trip and bound in the next, and the pooler can send the two
/// to different backends: v0.20.0 did that and failed every query (2026-11-13). Storage selects
/// named columns ([`Storable::select_columns`]), so a migration adding a column can't change a
/// cached statement's result type.
///
/// [`Storable::select_columns`]: super::traits::Storable::select_columns
pub(crate) fn connect_options(database_url: &str) -> Result<PgConnectOptions> {
    Ok(PgConnectOptions::from_str(database_url)?)
}

impl StorageFactory {
    pub async fn new(database_url: &str, use_secure_session_cookies: bool) -> Result<Self> {
        let connect_options = connect_options(database_url)?;
        // `acquire_time_level` only makes sqlx emit an event per acquire, which
        // `metrics::db::DbTimingLayer` turns into the acquire-time metrics. Pool size and
        // timeouts stay at sqlx's defaults. It takes sqlx's `log::LevelFilter`, which the
        // parse names without adding `log` as a dependency.
        let pool = PgPoolOptions::new()
            .acquire_time_level("trace".parse().expect("trace is a log level"))
            .connect_with(connect_options)
            .await?;

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
            entity_tags: Arc::new(GenericPostgresStorage::new(pool.clone())),
            site_assignments: Arc::new(GenericPostgresStorage::new(pool.clone())),
            dependency_members: Arc::new(GenericPostgresStorage::new(pool.clone())),
            subnet_vlan: Arc::new(GenericPostgresStorage::new(pool.clone())),
            site_credentials: Arc::new(GenericPostgresStorage::new(pool.clone())),
            host_credentials: Arc::new(GenericPostgresStorage::new(pool.clone())),
            daemon_interfaced_subnets: Arc::new(GenericPostgresStorage::new(pool.clone())),
            interface_neighbor_candidates: Arc::new(GenericPostgresStorage::new(pool.clone())),
            interface_neighbor_interfaces: Arc::new(GenericPostgresStorage::new(pool.clone())),
            interface_neighbor_hosts: Arc::new(GenericPostgresStorage::new(pool.clone())),
            discovery_warnings: Arc::new(GenericPostgresStorage::new(pool.clone())),
        })
    }
}

/// Connection counts of the main pool, for the pool gauges. Holds the pool but exposes only its
/// counts, so it can't run a query.
#[derive(Clone)]
pub struct PoolStats(PgPool);

impl PoolStats {
    /// Open connections, idle or in use.
    pub fn open(&self) -> u32 {
        self.0.size()
    }

    pub fn idle(&self) -> u32 {
        u32::try_from(self.0.num_idle()).unwrap_or(u32::MAX)
    }

    pub fn max(&self) -> u32 {
        self.0.options().get_max_connections()
    }
}

impl StorageFactory {
    pub fn pool_stats(&self) -> PoolStats {
        PoolStats(self.pool.clone())
    }

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
