use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
use crate::server::shared::attribution::AttributeSource;
use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
use crate::server::subnets::r#impl::base::{SubnetName, SubnetTypeAttributed};
use crate::server::{
    auth::access::{AccessSet, Assignment},
    config::{AppState, ServerConfig},
    daemons::r#impl::base::{Daemon, DaemonBase, DaemonMode},
    dependencies::r#impl::{
        base::{Dependency, DependencyBase, DependencyMembers},
        types::DependencyType,
    },
    hosts::r#impl::base::{Host, HostBase},
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
    organizations::r#impl::base::{Organization, OrganizationBase},
    ports::r#impl::base::{Port, PortBase, PortType},
    services::{
        definitions::ServiceDefinitionRegistry,
        r#impl::base::{Service, ServiceBase},
    },
    shared::{
        services::factory::ServiceFactory,
        storage::{factory::StorageFactory, traits::Storable},
        types::{Color, entities::EntitySource},
    },
    sites::r#impl::{Site, SiteBase},
    subnets::r#impl::{
        base::{Subnet, SubnetBase},
        types::SubnetType,
    },
    topology::types::edges::EdgeStyle,
    users::r#impl::{
        base::{User, UserBase},
        permissions::UserOrgPermissions,
    },
};
use axum::Router;
use cidr::IpCidr;
use cidr::Ipv4Cidr;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use testcontainers::{GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner};
use tokio::sync::OnceCell;
use uuid::Uuid;

pub mod access_writes;
pub mod api_key_all_sites;
pub mod api_key_site_access;
pub mod billing_emails_zero_amount;
pub mod cloud_setup;
pub mod cross_tenant_probe;
pub mod daemon_check_in_updates;
pub mod daemon_report_scope;
pub mod default_credentials;
pub mod deletion_reasons;
pub mod demo_data_seeding;
pub mod dependencies;
pub mod dependency_member_events;
pub mod discovery_finish;
pub mod discovery_port_ownership;
pub mod discovery_removed_children;
pub mod discovery_stale_port_guards;
pub mod discovery_stale_port_holders;
pub mod discovery_warnings;
pub mod email_unsubscribe;
pub mod email_verification;
pub mod far_end_advertised_ports;
pub mod fdb_resolution;
pub mod field_values_scope;
pub mod global_search;
pub mod guest_stray_addresses;
pub mod host_create_with_children;
pub mod host_interface_sync;
pub mod host_mac_ordering;
pub mod host_merge_on_discovery;
pub mod host_naming;
pub mod interface_neighbor_candidates;
pub mod invite_registration;
pub mod junction_writes;
pub mod list_pages;
pub mod lldp_resolution;
pub mod org_provisioning;
pub mod org_reset;
pub mod organization_writes;
pub mod provision_eligibility;
pub mod self_hosted_license_emails;
pub mod self_hosted_licensing;
pub mod session_gate;
pub mod setup_items;
pub mod setup_record;
pub mod snapshot_close_and_clone;
pub mod snmp_sim_resolution;
pub mod ssh_logins;
pub mod stripe_webhook_retries;
pub mod stripe_webhooks;
pub mod subnet_placement;
pub mod subnet_vlan_inference;
pub mod tag_names;
pub mod tag_scope;
pub mod tenant_isolation;
pub mod terms_notices;
pub mod virtualization_tree;
pub mod virtualizer_cascade;
pub mod vlan_last_seen;

pub const DAEMON_CONFIG_FIXTURE: &str = "src/tests/daemon_config.json";
pub const SERVER_DB_FIXTURE: &str = "src/tests/scanopy.sql";

/// The test process's one Postgres server, started by the first test that needs it.
static TEST_SERVER_PORT: OnceCell<u16> = OnceCell::const_new();
static TEST_SERVER_CONTAINER_ID: OnceLock<String> = OnceLock::new();
/// A database with every migration applied, which tests clone rather than migrating their own.
static MIGRATED_TEMPLATE: OnceCell<()> = OnceCell::const_new();
const MIGRATED_TEMPLATE_NAME: &str = "scanopy_migrated_template";

/// A test's own database on the shared server. Databases are never dropped during a run: with
/// tests connected, `DROP DATABASE` takes about a second, or stalls for a minute when concurrent,
/// and a full run then takes twice as long. They go with the server at process exit.
pub struct TestDb {
    port: u16,
    name: String,
}

impl TestDb {
    async fn create(template: Option<&str>) -> Self {
        let port = test_server_port().await;
        let name = format!("test_{}", Uuid::new_v4().simple());
        let template = template
            .map(|t| format!(" TEMPLATE \"{t}\""))
            .unwrap_or_default();
        let mut admin = PgConnection::connect(&admin_url(port)).await.unwrap();
        admin
            .execute(format!("CREATE DATABASE \"{name}\"{template}").as_str())
            .await
            .unwrap();
        Self { port, name }
    }

    pub fn url(&self) -> String {
        database_url(self.port, &self.name)
    }
}

fn database_url(port: u16, database: &str) -> String {
    format!("postgresql://postgres:password@localhost:{port}/{database}")
}

fn admin_url(port: u16) -> String {
    database_url(port, "postgres")
}

/// Durability is off because nothing outlives the process. Data stays on the container's disk: a
/// full run's databases take about 5 GB, more than a tmpfs should take from the Docker VM's memory.
async fn start_test_server() -> u16 {
    let container = GenericImage::new("postgres", "17-alpine")
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_PASSWORD", "password")
        .with_label("net.scanopy.test-db", "true")
        .with_cmd([
            "postgres",
            "-c",
            "fsync=off",
            "-c",
            "synchronous_commit=off",
            "-c",
            "full_page_writes=off",
            "-c",
            "max_connections=1000",
            // Connections outlive the tests that opened them, and a full run reaches 1000 without
            // this. A live test's pool checks a connection before use and opens another.
            "-c",
            "idle_session_timeout=10s",
        ])
        .start()
        .await
        .unwrap();

    // Docker Desktop can report a running container with no port bindings for a moment after it
    // starts, under load. `running=true` with an empty port map is that case, not a dead container.
    let mut port = container.get_host_port_ipv4(5432).await;
    for _ in 0..40 {
        if port.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
        port = container.get_host_port_ipv4(5432).await;
    }
    let port = match port {
        Ok(port) => port,
        Err(e) => {
            let running = container.is_running().await;
            let exit_code = container.exit_code().await;
            let logs = container
                .stderr_to_vec()
                .await
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .unwrap_or_else(|e| format!("<could not read logs: {e}>"));
            panic!(
                "could not read the test database's mapped port: {e}\n\
                 container {} running={running:?} exit_code={exit_code:?}\n\
                 container stderr:\n{logs}",
                container.id()
            );
        }
    };

    // The server outlives every test, and a static is never dropped, so removal moves to process
    // exit. A killed run skips this; the label lets the prune job find what it leaves behind.
    let _ = TEST_SERVER_CONTAINER_ID.set(container.id().to_string());
    std::mem::forget(container);
    extern "C" fn remove_test_server() {
        if let Some(id) = TEST_SERVER_CONTAINER_ID.get() {
            let _ = std::process::Command::new("docker")
                .args(["rm", "-f", "-v", id])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
    }
    // SAFETY: registers a function that takes no arguments and touches only a set-once static.
    unsafe { libc::atexit(remove_test_server) };

    port
}

async fn test_server_port() -> u16 {
    *TEST_SERVER_PORT.get_or_init(start_test_server).await
}

/// Migrates once per process. Closes every connection afterwards: Postgres refuses to clone a
/// template anyone is connected to.
async fn migrated_template() {
    MIGRATED_TEMPLATE
        .get_or_init(|| async {
            let port = test_server_port().await;
            let mut admin = PgConnection::connect(&admin_url(port)).await.unwrap();
            admin
                .execute(format!("CREATE DATABASE \"{MIGRATED_TEMPLATE_NAME}\"").as_str())
                .await
                .unwrap();
            let factory = StorageFactory::new(&database_url(port, MIGRATED_TEMPLATE_NAME), false)
                .await
                .unwrap();
            factory.pool().close().await;
        })
        .await;
}

/// A new, empty database.
pub async fn setup_test_db() -> (PgPool, String, TestDb) {
    let db = TestDb::create(None).await;
    let url = db.url();
    let pool = PgPool::connect(&url).await.unwrap();
    (pool, url, db)
}

/// A new database with every migration applied.
pub async fn setup_migrated_test_db() -> (PgPool, String, TestDb) {
    migrated_template().await;
    let db = TestDb::create(Some(MIGRATED_TEMPLATE_NAME)).await;
    let url = db.url();
    let pool = PgPool::connect(&url).await.unwrap();
    (pool, url, db)
}

pub async fn test_storage() -> (StorageFactory, TestDb) {
    let (pool, database_url, _container) = setup_migrated_test_db().await;
    pool.close().await;
    let factory = StorageFactory::new(&database_url, false).await.unwrap();
    (factory, _container)
}

pub fn organization() -> Organization {
    Organization::new(OrganizationBase::default())
}

pub fn user(organization_id: &Uuid) -> User {
    let mut user = User::new(UserBase::default());
    user.base.organization_id = *organization_id;
    user
}

pub fn site(organization_id: &Uuid) -> Site {
    Site::new(SiteBase::new(*organization_id))
}

pub fn host(site_id: &Uuid) -> Host {
    Host::new(HostBase {
        name: HostName::manual("Test Host".to_string()),
        hostname: Some(crate::server::shared::attribution::Attributed::new(
            crate::server::hosts::r#impl::attributes::HostHostnameValue("test.local".to_string()),
            AttributeSource::ReverseDns,
        )),
        site_id: *site_id,
        description: None,
        source: EntitySource::System,
        virtualization_metadata: None,
        virtualization_service_id: None,
        tags: Vec::new(),
        ..Default::default()
    })
}

pub fn ip_address(site_id: &Uuid, subnet_id: &Uuid) -> IPAddress {
    IPAddress::new(IPAddressBase {
        site_id: *site_id,
        subnet_id: *subnet_id,
        ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        mac_address: None, // MAC populated during ARP discovery
        position: 0,
        name: Some("eth0".to_string()),
        host_id: Uuid::nil(), // Placeholder - tests will set correct host_id
        ..Default::default()
    })
}

pub fn port(site_id: &Uuid, host_id: &Uuid) -> Port {
    Port::new(PortBase::new(*host_id, *site_id, PortType::default()))
}

pub fn subnet(site_id: &Uuid) -> Subnet {
    Subnet::new(SubnetBase {
        vlan_ids: Vec::new(),
        name: SubnetName::unattributed("Test Subnet".to_string()),
        description: None,
        site_id: *site_id,
        cidr: SubnetCidr::new(
            SubnetCidrValue(IpCidr::V4(
                Ipv4Cidr::new(Ipv4Addr::new(192, 168, 1, 0), 24).unwrap(),
            )),
            AttributeSource::DaemonSelfReport,
        ),
        subnet_type: SubnetTypeAttributed::unattributed(SubnetType::Lan),
        virtualization_service_id: None,
        source: EntitySource::System,
        tags: Vec::new(),
    })
}

pub fn service(site_id: &Uuid, host_id: &Uuid) -> Service {
    let service_def = ServiceDefinitionRegistry::find_by_id("Dns Server")
        .unwrap_or_else(|| ServiceDefinitionRegistry::all_service_definitions()[0].clone());

    Service::new(ServiceBase {
        name: "Test Service".to_string(),
        host_id: *host_id,
        bindings: vec![],
        site_id: *site_id,
        service_definition: service_def,
        virtualization_metadata: None,
        virtualization_service_id: None,
        source: EntitySource::System,
        tags: Vec::new(),
        position: 0,
    })
}

pub fn dependency(site_id: &Uuid) -> Dependency {
    Dependency::new(DependencyBase {
        name: "Test Dependency".to_string(),
        description: None,
        site_id: *site_id,
        color: Color::default(),
        dependency_type: DependencyType::RequestPath,
        members: DependencyMembers::default(),
        source: EntitySource::System,
        edge_style: EdgeStyle::Bezier,
        tags: Vec::new(),
    })
}

pub fn daemon(site_id: &Uuid, host_id: &Uuid) -> Daemon {
    Daemon::new(DaemonBase {
        host_id: *host_id,
        site_id: *site_id,
        tags: Vec::new(),
        name: "daemon".to_string(),
        url: "http://192.168.1.50:60073".to_string(),
        last_seen: Some(crate::server::shared::clock::now()),
        mode: DaemonMode::ServerPoll,
        version: None,
        user_id: Uuid::nil(),
        api_key_id: None,
        is_unreachable: false,
        standby: false,
        standby_cleared_at: None,
        os: None,
    })
}

pub async fn test_services() -> (StorageFactory, ServiceFactory, TestDb) {
    let (storage, _container) = test_storage().await;
    let services = ServiceFactory::new(&storage, ServerConfig::default())
        .await
        .unwrap();
    (storage, services, _container)
}
/// [`test_services`] wrapped in an [`AppState`], for code that reads services through the state the
/// handlers get, such as a list page.
pub async fn test_state() -> (StorageFactory, AppState, TestDb) {
    let (storage, services, _container) = test_services().await;
    let state = AppState {
        config: ServerConfig::default(),
        services,
        session_store: storage.sessions.clone(),
        license_service: None,
        license_issuer: None,
        pool: storage.pool().clone(),
        shutdown: tokio_util::sync::CancellationToken::new(),
    };
    (storage, state, _container)
}

pub async fn setup_test_app() -> Router<Arc<AppState>> {
    let config = ServerConfig::default();

    let state = AppState::new(config).await.unwrap();

    let (router, _openapi) = crate::server::shared::handlers::factory::create_router(state.clone());
    router.with_state(state)
}

/// What a principal holding `role` on `site_ids` can do.
pub fn access_on(role: UserOrgPermissions, site_ids: &[Uuid]) -> AccessSet {
    AccessSet::new(role, &Assignment::for_api_key(role, Some(site_ids)), &[])
}

/// The scope of a caller who sees `site_ids` in `organization_id`, with no user behind it.
pub fn list_scope(
    site_ids: &[Uuid],
    organization_id: Uuid,
) -> crate::server::shared::handlers::traits::ListScope {
    crate::server::shared::handlers::traits::ListScope {
        site_ids: site_ids.to_vec(),
        organization_id,
        user_id: None,
        organization_grants: None,
    }
}
