use crate::server::{
    auth::{oidc::OidcService, service::AuthService},
    billing::service::{BillingService, BillingServiceParams},
    bindings::service::BindingService,
    brevo::service::BrevoService,
    config::ServerConfig,
    daemon_api_keys::service::DaemonApiKeyService,
    daemons::service::DaemonService,
    discovery::service::DiscoveryService,
    email::{brevo::BrevoEmailProvider, smtp::SmtpEmailProvider, traits::EmailService},
    groups::{group_bindings::GroupBindingStorage, service::GroupService},
    hosts::service::HostService,
    if_entries::service::IfEntryService,
    interfaces::service::InterfaceService,
    invites::service::InviteService,
    logging::service::LoggingService,
    metrics::service::MetricsService,
    networks::service::NetworkService,
    organizations::service::OrganizationService,
    ports::service::PortService,
    posthog::PosthogService,
    services::service::ServiceService,
    shared::{events::bus::EventBus, storage::factory::StorageFactory},
    shares::service::ShareService,
    snmp_credentials::service::SnmpCredentialService,
    subnets::service::SubnetService,
    tags::{
        entity_tags::{EntityTagService, EntityTagStorage},
        service::TagService,
    },
    topology::service::main::TopologyService,
    user_api_keys::{
        r#impl::network_access::UserApiKeyNetworkAccessStorage, service::UserApiKeyService,
    },
    users::{UserNetworkAccessStorage, service::UserService},
};
use anyhow::Result;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::{Arc, OnceLock};

// Global Prometheus handle - the recorder can only be installed once per process
static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

pub struct ServiceFactory {
    pub user_service: Arc<UserService>,
    pub auth_service: Arc<AuthService>,
    pub network_service: Arc<NetworkService>,
    pub host_service: Arc<HostService>,
    pub interface_service: Arc<InterfaceService>,
    pub group_service: Arc<GroupService>,
    pub subnet_service: Arc<SubnetService>,
    pub daemon_service: Arc<DaemonService>,
    pub topology_service: Arc<TopologyService>,
    pub service_service: Arc<ServiceService>,
    pub discovery_service: Arc<DiscoveryService>,
    pub daemon_api_key_service: Arc<DaemonApiKeyService>,
    pub user_api_key_service: Arc<UserApiKeyService>,
    pub organization_service: Arc<OrganizationService>,
    pub invite_service: Arc<InviteService>,
    pub share_service: Arc<ShareService>,
    pub oidc_service: Option<Arc<OidcService>>,
    pub billing_service: Option<Arc<BillingService>>,
    pub email_service: Option<Arc<EmailService>>,
    pub brevo_service: Option<Arc<BrevoService>>,
    pub posthog_service: Option<Arc<PosthogService>>,
    pub event_bus: Arc<EventBus>,
    pub logging_service: Arc<LoggingService>,
    pub metrics_service: Arc<MetricsService>,
    pub tag_service: Arc<TagService>,
    pub entity_tag_service: Arc<EntityTagService>,
    pub port_service: Arc<PortService>,
    pub binding_service: Arc<BindingService>,
    pub snmp_credential_service: Arc<SnmpCredentialService>,
    pub if_entry_service: Arc<IfEntryService>,
}

impl ServiceFactory {
    pub async fn new(storage: &StorageFactory, config: Option<ServerConfig>) -> Result<Self> {
        let event_bus = Arc::new(EventBus::new());

        let logging_service = Arc::new(LoggingService::new());

        // Initialize Prometheus metrics recorder - uses global singleton since recorder
        // can only be installed once per process (important for tests)
        let prometheus_handle = PROMETHEUS_HANDLE
            .get_or_init(|| {
                PrometheusBuilder::new()
                    .install_recorder()
                    .expect("failed to install Prometheus recorder")
            })
            .clone();
        let metrics_service = Arc::new(MetricsService::new(prometheus_handle));

        let tag_service = Arc::new(TagService::new(storage.tags.clone(), event_bus.clone()));
        let entity_tag_storage = Arc::new(EntityTagStorage::new(storage.pool.clone()));
        let entity_tag_service = Arc::new(EntityTagService::new(
            entity_tag_storage,
            tag_service.clone(),
        ));

        let daemon_api_key_service = Arc::new(DaemonApiKeyService::new(
            storage.daemon_api_keys.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        let user_api_key_network_access_storage =
            Arc::new(UserApiKeyNetworkAccessStorage::new(storage.pool.clone()));
        let user_api_key_service = Arc::new(UserApiKeyService::new(
            storage.user_api_keys.clone(),
            user_api_key_network_access_storage,
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        let group_binding_storage = Arc::new(GroupBindingStorage::new(storage.pool.clone()));
        let group_service = Arc::new(GroupService::new(
            storage.groups.clone(),
            group_binding_storage,
            event_bus.clone(),
            entity_tag_service.clone(),
        ));
        let organization_service = Arc::new(OrganizationService::new(
            storage.organizations.clone(),
            event_bus.clone(),
        ));
        let invite_service = Arc::new(InviteService::new(
            storage.invites.clone(),
            event_bus.clone(),
        ));

        let share_service = Arc::new(ShareService::new(storage.shares.clone(), event_bus.clone()));

        let port_service = Arc::new(PortService::new(storage.ports.clone(), event_bus.clone()));

        let binding_service = Arc::new(BindingService::new(
            storage.bindings.clone(),
            event_bus.clone(),
        ));

        let interface_service = Arc::new(InterfaceService::new(
            storage.interfaces.clone(),
            event_bus.clone(),
        ));

        let subnet_service = Arc::new(SubnetService::new(
            storage.subnets.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        let network_service = Arc::new(NetworkService::new(
            storage.networks.clone(),
            subnet_service.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        let user_network_access_storage =
            Arc::new(UserNetworkAccessStorage::new(storage.pool.clone()));
        let user_service = Arc::new(UserService::new(
            storage.users.clone(),
            user_network_access_storage,
            event_bus.clone(),
        ));

        let service_service = Arc::new(ServiceService::new(
            storage.services.clone(),
            binding_service.clone(),
            group_service.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        // IfEntryService needs InterfaceService for validation
        let if_entry_service = Arc::new(IfEntryService::new(
            storage.if_entries.clone(),
            event_bus.clone(),
            interface_service.clone(),
        ));

        let snmp_credential_service = Arc::new(SnmpCredentialService::new(
            storage.snmp_credentials.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            network_service.clone(),
            interface_service.clone(),
        ));

        // Already implements Arc internally due to scheduler + sessions
        let discovery_service = DiscoveryService::new(
            storage.discovery.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            snmp_credential_service.clone(),
            network_service.clone(),
            organization_service.clone(),
        )
        .await?;

        // Create DaemonService with most dependencies directly (not host_service - circular)
        let daemon_service = Arc::new(DaemonService::new(
            storage.daemons.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            discovery_service.clone(),
            subnet_service.clone(),
            network_service.clone(),
            organization_service.clone(),
            user_service.clone(),
            daemon_api_key_service.clone(),
        ));

        // HostService needs DaemonService
        let host_service = Arc::new(HostService::new(
            storage.hosts.clone(),
            interface_service.clone(),
            port_service.clone(),
            service_service.clone(),
            if_entry_service.clone(),
            daemon_service.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        // Set HostService where there's a circular reference
        let _ = service_service.set_host_service(host_service.clone());
        let _ = daemon_service.set_host_service(host_service.clone());
        let _ = snmp_credential_service.set_host_service(host_service.clone());

        let topology_service = Arc::new(TopologyService::new(
            host_service.clone(),
            interface_service.clone(),
            subnet_service.clone(),
            group_service.clone(),
            service_service.clone(),
            port_service.clone(),
            binding_service.clone(),
            if_entry_service.clone(),
            tag_service.clone(),
            storage.topologies.clone(),
            event_bus.clone(),
        ));

        let email_service = config.clone().and_then(|c| {
            let public_url = c.public_url.clone();

            // Prefer Brevo if API key is provided
            if let Some(ref brevo_api_key) = c.brevo_api_key {
                let provider = Box::new(BrevoEmailProvider::new(brevo_api_key.clone()));
                return Some(Arc::new(EmailService::new(
                    provider,
                    user_service.clone(),
                    organization_service.clone(),
                    host_service.clone(),
                    network_service.clone(),
                    service_service.clone(),
                    public_url,
                )));
            }

            // Fall back to SMTP
            if let (Some(smtp_username), Some(smtp_password), Some(smtp_email), Some(smtp_relay)) =
                (c.smtp_username, c.smtp_password, c.smtp_email, c.smtp_relay)
            {
                let provider =
                    SmtpEmailProvider::new(smtp_username, smtp_password, smtp_email, smtp_relay)
                        .ok()?;
                return Some(Arc::new(EmailService::new(
                    Box::new(provider),
                    user_service.clone(),
                    organization_service.clone(),
                    host_service.clone(),
                    network_service.clone(),
                    service_service.clone(),
                    public_url,
                )));
            }

            None
        });

        let billing_service = config.clone().and_then(|c| {
            if let Some(stripe_secret) = c.stripe_secret
                && let Some(webhook_secret) = c.stripe_webhook_secret
            {
                return Some(Arc::new(BillingService::new(BillingServiceParams {
                    stripe_secret,
                    webhook_secret,
                    organization_service: organization_service.clone(),
                    invite_service: invite_service.clone(),
                    user_service: user_service.clone(),
                    network_service: network_service.clone(),
                    host_service: host_service.clone(),
                    daemon_service: daemon_service.clone(),
                    discovery_service: discovery_service.clone(),
                    share_service: share_service.clone(),
                    email_service: email_service.clone(),
                    event_bus: event_bus.clone(),
                })));
            }
            None
        });

        let public_url = config
            .as_ref()
            .map(|c| c.public_url.clone())
            .unwrap_or_else(|| "http://localhost:3000".to_string());

        let auth_service = Arc::new(AuthService::new(
            user_service.clone(),
            organization_service.clone(),
            email_service.clone(),
            event_bus.clone(),
            public_url,
        ));

        // Create Brevo service if API key is configured (before config is consumed)
        let brevo_service = config.as_ref().and_then(|c| {
            c.brevo_api_key.as_ref().map(|api_key| {
                Arc::new(BrevoService::new(
                    api_key.clone(),
                    network_service.clone(),
                    host_service.clone(),
                    user_service.clone(),
                    organization_service.clone(),
                    daemon_service.clone(),
                    tag_service.clone(),
                    user_api_key_service.clone(),
                    snmp_credential_service.clone(),
                ))
            })
        });

        // Create PostHog service if API key is configured
        let posthog_service =
            if let Some(posthog_key) = config.as_ref().and_then(|c| c.posthog_key.clone()) {
                Some(Arc::new(
                    PosthogService::new(
                        posthog_key,
                        "https://ph.scanopy.net".to_string(),
                        network_service.clone(),
                    )
                    .await,
                ))
            } else {
                None
            };

        let oidc_service = config.and_then(|c| {
            if let Some(oidc_providers) = c.oidc_providers {
                return Some(Arc::new(OidcService::new(
                    oidc_providers,
                    &c.public_url,
                    auth_service.clone(),
                    user_service.clone(),
                    event_bus.clone(),
                )));
            }
            None
        });

        // Register services that implement event bus subscriber
        event_bus
            .register_subscriber(topology_service.clone())
            .await;

        event_bus.register_subscriber(logging_service.clone()).await;
        event_bus.register_subscriber(metrics_service.clone()).await;
        event_bus
            .register_subscriber(organization_service.clone())
            .await;
        event_bus.register_subscriber(host_service.clone()).await;

        if let Some(billing_service) = billing_service.clone() {
            event_bus.register_subscriber(billing_service).await;
        }

        if let Some(brevo_service) = brevo_service.clone() {
            event_bus.register_subscriber(brevo_service).await;
        }

        if let Some(posthog_service) = posthog_service.clone() {
            event_bus.register_subscriber(posthog_service).await;
        }

        if let Some(email_service) = email_service.clone() {
            event_bus.register_subscriber(email_service).await;
        }

        event_bus.register_subscriber(daemon_service.clone()).await;

        Ok(Self {
            user_service,
            auth_service,
            network_service,
            host_service,
            interface_service,
            group_service,
            subnet_service,
            daemon_service,
            topology_service,
            service_service,
            discovery_service,
            daemon_api_key_service,
            user_api_key_service,
            organization_service,
            invite_service,
            share_service,
            oidc_service,
            billing_service,
            email_service,
            brevo_service,
            posthog_service,
            event_bus,
            logging_service,
            metrics_service,
            tag_service,
            entity_tag_service,
            port_service,
            binding_service,
            snmp_credential_service,
            if_entry_service,
        })
    }
}
