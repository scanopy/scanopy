use crate::daemon::{
    discovery::{
        buffer::EntityBuffer, manager::DaemonDiscoverySessionManager,
        service::base::DaemonDiscoveryService,
    },
    runtime::{service::DaemonRuntimeService, state::DaemonState},
    shared::{api_client::DaemonApiClient, config::ConfigStore},
};
use anyhow::Result;
use std::sync::Arc;

pub struct DaemonServiceFactory {
    pub discovery_service: Arc<DaemonDiscoveryService>,
    pub discovery_manager: Arc<DaemonDiscoverySessionManager>,
    pub runtime_service: Arc<DaemonRuntimeService>,
    pub entity_buffer: Arc<EntityBuffer>,
    pub daemon_state: Arc<DaemonState>,
}

impl DaemonServiceFactory {
    pub async fn new(config: Arc<ConfigStore>) -> Result<Self> {
        // Initialize services with proper dependencies

        // Create entity buffer first - shared between discovery service and daemon state
        let entity_buffer = Arc::new(EntityBuffer::new());

        let api_client = DaemonApiClient::for_mode(config.clone(), config.get_mode().await?);

        let discovery_service = Arc::new(DaemonDiscoveryService::new(
            config.clone(),
            api_client.clone(),
            entity_buffer.clone(),
        ));
        let discovery_manager = Arc::new(DaemonDiscoverySessionManager::new(
            discovery_service.clone(),
        ));
        let runtime_service = Arc::new(DaemonRuntimeService::new(
            config.clone(),
            api_client,
            discovery_manager.clone(),
        ));
        let daemon_state = Arc::new(DaemonState::new(
            config.clone(),
            discovery_service.clone(),
            entity_buffer.clone(),
            discovery_manager.clone(),
        ));

        Ok(Self {
            discovery_service,
            discovery_manager,
            runtime_service,
            entity_buffer,
            daemon_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::service::ops::DiscoveryOps;
    use crate::daemon::discovery::types::base::DiscoveryPhase;
    use crate::daemon::runtime::service::StartupOutcome;
    use crate::daemon::shared::config::AppConfig;
    use crate::server::daemons::r#impl::{api::DaemonDiscoveryRequest, base::DaemonMode};
    use crate::server::discovery::r#impl::types::DiscoveryType;
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    /// A ServerPoll daemon reaches the server through nothing it does: the session's `Started`
    /// and terminal updates and the `/api/initialize` handshake all stay local, and the terminal
    /// update waits in `terminal_payload` for the server's poll. The config also carries a
    /// `server_url` pointing at a live listener, so a client built by any route would connect.
    #[tokio::test]
    async fn a_server_poll_daemon_makes_no_outbound_request() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let site_id = Uuid::new_v4();
        let mut app_config = AppConfig::default();
        app_config.mode = DaemonMode::ServerPoll;
        app_config.server_url = Some(format!("http://{}", listener.local_addr().unwrap()));
        app_config.site_id = Some(site_id);
        app_config.daemon_api_key = Some("key".to_string());
        let config = Arc::new(ConfigStore::new(dir.path().join("config.json"), app_config));
        let services = DaemonServiceFactory::new(config).await.unwrap();

        let request = DaemonDiscoveryRequest {
            session_id: Uuid::new_v4(),
            discovery_type: DiscoveryType::default(),
            credential_mappings: Vec::new(),
            discovery_id: Uuid::new_v4(),
            subnets: Vec::new(),
        };
        let ops = DiscoveryOps::new(&services.discovery_service, request.discovery_type.clone());
        ops.start_session(&request, Vec::new()).await.unwrap();
        let session = ops.get_session().await.unwrap();
        ops.finish_session(Ok(()), CancellationToken::new())
            .await
            .unwrap();

        let outcome = services
            .runtime_service
            .initialize_services(site_id, "key".to_string())
            .await
            .unwrap();

        assert!(matches!(outcome, StartupOutcome::Ok));
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock,
            "the daemon connected to the server URL"
        );
        assert_eq!(
            session
                .consecutive_report_failures
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        let terminal = services.discovery_service.terminal_payload.read().await;
        assert_eq!(
            terminal.as_ref().map(|p| p.phase),
            Some(DiscoveryPhase::Complete)
        );
    }

    /// An interface refresh classifies the interfaced subnets, hands them to the next status
    /// report and no later one, and completes without the phases a scan runs. A first-run
    /// Unified session would self-report the daemon's host into the buffer; this leaves it empty.
    #[tokio::test]
    async fn an_interface_refresh_reports_once_and_scans_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut app_config = AppConfig::default();
        app_config.mode = DaemonMode::ServerPoll;
        app_config.site_id = Some(Uuid::new_v4());
        app_config.daemon_api_key = Some("key".to_string());
        // No NIC matches, so there is nothing to create and nothing waits on the server.
        app_config.interfaces = vec!["scanopy-none0".to_string()];
        let config = Arc::new(ConfigStore::new(dir.path().join("config.json"), app_config));
        let services = DaemonServiceFactory::new(config).await.unwrap();

        let request = DaemonDiscoveryRequest {
            session_id: Uuid::new_v4(),
            discovery_type: DiscoveryType::InterfaceRefresh {
                host_id: Uuid::new_v4(),
            },
            credential_mappings: Vec::new(),
            discovery_id: Uuid::nil(),
            subnets: Vec::new(),
        };
        services
            .discovery_manager
            .try_initiate_session(request)
            .await;
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while services
                .discovery_service
                .terminal_payload
                .read()
                .await
                .is_none()
            {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the refresh did not finish");

        let terminal = services.discovery_service.terminal_payload.read().await;
        assert_eq!(
            terminal.as_ref().map(|p| p.phase),
            Some(DiscoveryPhase::Complete)
        );
        assert!(services.entity_buffer.get_pending().await.hosts.is_empty());
        let (reported, fingerprint) = services.discovery_service.status_interfaces().await;
        assert_eq!(reported.map(|s| s.len()), Some(0));
        let (again, unchanged) = services.discovery_service.status_interfaces().await;
        assert!(again.is_none());
        assert_eq!(unchanged, fingerprint);
    }
}
