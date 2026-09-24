use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
use crate::server::shared::attribution::AttributeSource;
use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
use crate::server::{
    config::{AppState, ServerConfig},
    daemons::r#impl::base::{Daemon, DaemonBase, DaemonMode},
    dependencies::r#impl::{
        base::{Dependency, DependencyBase, DependencyMembers},
        types::DependencyType,
    },
    hosts::r#impl::base::{Host, HostBase},
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
    networks::r#impl::{Network, NetworkBase},
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
    subnets::r#impl::{
        base::{Subnet, SubnetBase},
        types::SubnetType,
    },
    topology::types::edges::EdgeStyle,
    users::r#impl::base::{User, UserBase},
};
use axum::Router;
use chrono::Utc;
use cidr::IpCidr;
use cidr::Ipv4Cidr;
use sqlx::PgPool;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner};
use uuid::Uuid;

pub mod demo_data_seeding;
pub mod dependencies;
pub mod fdb_resolution;
pub mod host_create_with_children;
pub mod host_interface_sync;
pub mod host_naming;
pub mod interface_neighbor_candidates;
pub mod lldp_resolution;
pub mod self_hosted_license_emails;
pub mod self_hosted_licensing;
pub mod snmp_sim_resolution;
pub mod subnet_placement;

pub const DAEMON_CONFIG_FIXTURE: &str = "src/tests/daemon_config.json";
pub const SERVER_DB_FIXTURE: &str = "src/tests/scanopy.sql";

pub async fn setup_test_db() -> (PgPool, String, ContainerAsync<GenericImage>) {
    let postgres_image = GenericImage::new("postgres", "17-alpine")
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_PASSWORD", "password")
        .with_env_var("POSTGRES_DB", "scanopy_test");

    let container = postgres_image.start().await.unwrap();

    // `PortNotExposed` here means the container was gone by the time its port map was read —
    // a stopped container reports no bindings at all, which is indistinguishable from one that
    // never published the port. It shows up as an intermittent failure of whichever test drew the
    // short straw, with nothing in the message saying which container or why it died.
    //
    // The wait strategy above is not the cause: the postgres image logs its ready line once per
    // stream — the temporary `initdb` server on stdout, the real server on stderr — so waiting on
    // stderr already waits for the real one. Whatever stops the container happens after that.
    //
    // So report what the container had to say rather than unwrapping. If this fires again, the
    // exit code and its own logs are what identify the cause, and they are gone the moment the
    // process exits.
    let port = match container.get_host_port_ipv4(5432).await {
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

    let database_url = format!(
        "postgresql://postgres:password@localhost:{}/scanopy_test",
        port
    );

    let pool = PgPool::connect(&database_url).await.unwrap();
    (pool, database_url, container)
}

pub async fn test_storage() -> (StorageFactory, ContainerAsync<GenericImage>) {
    let (pool, database_url, _container) = setup_test_db().await;
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

pub fn network(organization_id: &Uuid) -> Network {
    Network::new(NetworkBase::new(*organization_id))
}

pub fn host(network_id: &Uuid) -> Host {
    Host::new(HostBase {
        name: HostName::manual("Test Host".to_string()),
        hostname: Some(crate::server::shared::attribution::Attributed::new(
            crate::server::hosts::r#impl::attributes::HostHostnameValue("test.local".to_string()),
            AttributeSource::ReverseDns,
        )),
        network_id: *network_id,
        description: None,
        source: EntitySource::System,
        virtualization_metadata: None,
        virtualization_service_id: None,
        hidden: false,
        tags: Vec::new(),
        ..Default::default()
    })
}

pub fn ip_address(network_id: &Uuid, subnet_id: &Uuid) -> IPAddress {
    IPAddress::new(IPAddressBase {
        network_id: *network_id,
        subnet_id: *subnet_id,
        ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        mac_address: None, // MAC populated during ARP discovery
        position: 0,
        name: Some("eth0".to_string()),
        host_id: Uuid::nil(), // Placeholder - tests will set correct host_id
    })
}

pub fn port(network_id: &Uuid, host_id: &Uuid) -> Port {
    Port::new(PortBase {
        port_type: PortType::default(),
        host_id: *host_id,
        network_id: *network_id,
    })
}

pub fn subnet(network_id: &Uuid) -> Subnet {
    Subnet::new(SubnetBase {
        name: "Test Subnet".to_string(),
        description: None,
        network_id: *network_id,
        cidr: SubnetCidr::new(
            SubnetCidrValue(IpCidr::V4(
                Ipv4Cidr::new(Ipv4Addr::new(192, 168, 1, 0), 24).unwrap(),
            )),
            AttributeSource::DaemonSelfReport,
        ),
        subnet_type: SubnetType::Lan,
        virtualization_service_id: None,
        source: EntitySource::System,
        tags: Vec::new(),
    })
}

pub fn service(network_id: &Uuid, host_id: &Uuid) -> Service {
    let service_def = ServiceDefinitionRegistry::find_by_id("Dns Server")
        .unwrap_or_else(|| ServiceDefinitionRegistry::all_service_definitions()[0].clone());

    Service::new(ServiceBase {
        name: "Test Service".to_string(),
        host_id: *host_id,
        bindings: vec![],
        network_id: *network_id,
        service_definition: service_def,
        virtualization_metadata: None,
        virtualization_service_id: None,
        source: EntitySource::System,
        tags: Vec::new(),
        position: 0,
    })
}

pub fn dependency(network_id: &Uuid) -> Dependency {
    Dependency::new(DependencyBase {
        name: "Test Dependency".to_string(),
        description: None,
        network_id: *network_id,
        color: Color::default(),
        dependency_type: DependencyType::RequestPath,
        members: DependencyMembers::default(),
        source: EntitySource::System,
        edge_style: EdgeStyle::Bezier,
        tags: Vec::new(),
    })
}

pub fn daemon(network_id: &Uuid, host_id: &Uuid) -> Daemon {
    Daemon::new(DaemonBase {
        host_id: *host_id,
        network_id: *network_id,
        tags: Vec::new(),
        name: "daemon".to_string(),
        url: "http://192.168.1.50:60073".to_string(),
        last_seen: Some(Utc::now()),
        mode: DaemonMode::ServerPoll,
        version: None,
        user_id: Uuid::nil(),
        api_key_id: None,
        is_unreachable: false,
        standby: false,
        standby_cleared_at: None,
    })
}

pub async fn test_services() -> (StorageFactory, ServiceFactory, ContainerAsync<GenericImage>) {
    let (storage, _container) = test_storage().await;
    let services = ServiceFactory::new(&storage, ServerConfig::default())
        .await
        .unwrap();
    (storage, services, _container)
}
pub async fn setup_test_app() -> Router<Arc<AppState>> {
    let config = ServerConfig::default();

    let state = AppState::new(config).await.unwrap();

    let (router, _openapi) = crate::server::shared::handlers::factory::create_router(state.clone());
    router.with_state(state)
}
