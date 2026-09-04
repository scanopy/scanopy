use crate::server::{
    auth::{oidc::OidcService, service::AuthService},
    billing::service::{BillingService, BillingServiceParams},
    bindings::service::BindingService,
    brevo::service::BrevoService,
    config::ServerConfig,
    credentials::service::CredentialService,
    daemon_api_keys::service::DaemonApiKeyService,
    daemons::service::DaemonService,
    dependencies::service::DependencyService,
    digest::service::DiscoveryDigestService,
    discovery::service::DiscoveryService,
    email::{
        brevo::BrevoEmailProvider, logging::LoggingEmailProvider, service::EmailService,
        smtp::SmtpEmailProvider,
    },
    hosts::service::HostService,
    interfaces::service::InterfaceService,
    invites::service::InviteService,
    ip_addresses::service::IPAddressService,
    logging::service::LoggingService,
    metrics::service::MetricsService,
    networks::service::NetworkService,
    organizations::service::OrganizationService,
    ports::service::PortService,
    posthog::PosthogService,
    services::service::ServiceService,
    shared::{
        events::{
            bus::EventBus,
            registry::{CollectedServices, ServiceCollector, register_all_subscribers},
        },
        storage::factory::StorageFactory,
    },
    shares::service::ShareService,
    snapshots::service::SnapshotService,
    subnets::service::SubnetService,
    tags::{entity_tags::EntityTagService, service::TagService},
    topology::service::main::TopologyService,
    user_api_keys::service::UserApiKeyService,
    users::service::UserService,
    vlans::service::VlanService,
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
    pub ip_address_service: Arc<IPAddressService>,
    pub dependency_service: Arc<DependencyService>,
    pub subnet_service: Arc<SubnetService>,
    pub daemon_service: Arc<DaemonService>,
    pub topology_service: Arc<TopologyService>,
    pub snapshot_service: Arc<SnapshotService>,
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
    pub credential_service: Arc<CredentialService>,
    pub interface_service: Arc<InterfaceService>,
    pub vlan_service: Arc<VlanService>,
    pub discovery_digest_service: Arc<DiscoveryDigestService>,
}

impl ServiceFactory {
    pub async fn new(storage: &StorageFactory, config: ServerConfig) -> Result<Self> {
        // The plan a new self-hosted org is provisioned onto: the single,
        // unrestricted self-hosted plan. Its `included_orgs` also bounds the
        // org-creation cap (unlimited by default). Unused on cloud, where new
        // orgs get no plan until Stripe checkout.
        let default_self_hosted_plan = crate::server::billing::plans::self_hosted_plan();

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
        let entity_tag_service = Arc::new(EntityTagService::new(
            storage.entity_tags.clone(),
            tag_service.clone(),
        ));

        let daemon_api_key_service = Arc::new(DaemonApiKeyService::new(
            storage.daemon_api_keys.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        let user_api_key_service = Arc::new(UserApiKeyService::new(
            storage.user_api_keys.clone(),
            storage.user_api_key_network_access.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        let dependency_service = Arc::new(DependencyService::new(
            storage.dependencies.clone(),
            storage.dependency_members.clone(),
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

        let ip_address_service = Arc::new(IPAddressService::new(
            storage.ip_addresses.clone(),
            event_bus.clone(),
        ));

        let subnet_service = Arc::new(SubnetService::new(
            storage.subnets.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            // For re-filing addresses a narrowed range displaces. No cycle — `IPAddressService`
            // depends on no service and is constructed above.
            ip_address_service.clone(),
        ));

        let vlan_service = Arc::new(VlanService::new(
            storage.vlans.clone(),
            event_bus.clone(),
            storage.subnet_vlan.clone(),
        ));

        let network_service = Arc::new(NetworkService::new(
            storage.networks.clone(),
            subnet_service.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        let user_service = Arc::new(UserService::new(
            storage.users.clone(),
            storage.user_network_access.clone(),
            event_bus.clone(),
        ));

        let service_service = Arc::new(ServiceService::new(
            storage.services.clone(),
            binding_service.clone(),
            dependency_service.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        // InterfaceService needs IPAddressService for validation
        let interface_service = Arc::new(InterfaceService::new(
            storage.interfaces.clone(),
            event_bus.clone(),
            ip_address_service.clone(),
        ));

        let credential_service = Arc::new(CredentialService::new(
            storage.credentials.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            network_service.clone(),
            ip_address_service.clone(),
            organization_service.clone(),
            storage.pool.clone(),
        ));

        // Already implements Arc internally due to scheduler + sessions
        let discovery_service = DiscoveryService::new(
            storage.discovery.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            credential_service.clone(),
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
            credential_service.clone(),
            subnet_service.clone(),
            network_service.clone(),
            organization_service.clone(),
            user_service.clone(),
            daemon_api_key_service.clone(),
            crate::server::config::get_deployment_type(&config),
        ));

        // HostService needs DaemonService
        let host_service = Arc::new(HostService::new(
            storage.hosts.clone(),
            ip_address_service.clone(),
            port_service.clone(),
            service_service.clone(),
            interface_service.clone(),
            daemon_service.clone(),
            credential_service.clone(),
            subnet_service.clone(),
            vlan_service.clone(),
            network_service.clone(),
            organization_service.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        // Set lazy dependencies to break circular references
        let _ = service_service.set_host_service(host_service.clone());
        let _ = daemon_service.set_host_service(host_service.clone());
        let _ = credential_service.set_host_service(host_service.clone());
        let _ = discovery_service.set_daemon_service(daemon_service.clone());

        let topology_service = Arc::new(TopologyService::new(
            host_service.clone(),
            ip_address_service.clone(),
            subnet_service.clone(),
            dependency_service.clone(),
            service_service.clone(),
            port_service.clone(),
            binding_service.clone(),
            interface_service.clone(),
            tag_service.clone(),
            vlan_service.clone(),
            network_service.clone(),
            storage.topologies.clone(),
            event_bus.clone(),
        ));

        let snapshot_service = SnapshotService::new(
            Arc::new(storage.pool.clone()),
            storage.snapshots.clone(),
            event_bus.clone(),
            network_service.clone(),
            organization_service.clone(),
        );

        let discovery_digest_service = Arc::new(DiscoveryDigestService::new(
            host_service.clone(),
            service_service.clone(),
            port_service.clone(),
            ip_address_service.clone(),
            interface_service.clone(),
            binding_service.clone(),
            subnet_service.clone(),
            vlan_service.clone(),
            user_service.clone(),
            network_service.clone(),
            discovery_service.clone(),
            event_bus.clone(),
        ));

        let public_url = config.public_url.clone();
        let deployment_type = crate::server::config::get_deployment_type(&config);

        // A configured email log directory wins over Brevo/SMTP: it is a
        // testing transport you explicitly turn on, so it has to work with no
        // credentials present and override any that happen to be set.
        let email_service = if let Some(ref email_log_dir) = config.email_log_dir {
            tracing::warn!(
                dir = %email_log_dir.display(),
                "SCANOPY_EMAIL_LOG_DIR is set: emails will be logged and written to disk, not delivered"
            );
            Some(Arc::new(EmailService::new(
                Box::new(LoggingEmailProvider::new(Some(email_log_dir.clone()))),
                user_service.clone(),
                organization_service.clone(),
                host_service.clone(),
                network_service.clone(),
                service_service.clone(),
                daemon_service.clone(),
                public_url,
                deployment_type,
            )))
        } else if let Some(ref brevo_api_key) = config.brevo_api_key {
            // Brevo outranks SMTP. An operator who has both configured would otherwise
            // never learn that their SMTP settings are being ignored.
            if config.smtp_relay.is_some() {
                tracing::warn!(
                    "SCANOPY_BREVO_API_KEY is set and takes precedence: the SCANOPY_SMTP_* \
                     settings will be ignored"
                );
            }
            let brevo_provider = BrevoEmailProvider::new(brevo_api_key.clone());
            Some(Arc::new(EmailService::new(
                Box::new(brevo_provider),
                user_service.clone(),
                organization_service.clone(),
                host_service.clone(),
                network_service.clone(),
                service_service.clone(),
                daemon_service.clone(),
                public_url,
                deployment_type,
            )))
        } else {
            // SMTP needs all four values. Every way this can fail used to be silent: the
            // server booted normally, reported itself healthy, and never sent a single
            // email. Say which variables are missing, and say when the transport itself
            // could not be built.
            match (
                config.smtp_username,
                config.smtp_password,
                config.smtp_email,
                config.smtp_relay,
            ) {
                (Some(username), Some(password), Some(email), Some(relay)) => {
                    match SmtpEmailProvider::new(username, password, email, relay, config.smtp_port)
                    {
                        Ok(smtp_provider) => Some(Arc::new(EmailService::new(
                            Box::new(smtp_provider),
                            user_service.clone(),
                            organization_service.clone(),
                            host_service.clone(),
                            network_service.clone(),
                            service_service.clone(),
                            daemon_service.clone(),
                            public_url,
                            deployment_type,
                        ))),
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                "SMTP is configured but the transport could not be built: \
                                 email is disabled"
                            );
                            None
                        }
                    }
                }
                (username, password, email, relay) => {
                    let missing: Vec<&str> = [
                        ("SCANOPY_SMTP_USERNAME", username.is_none()),
                        ("SCANOPY_SMTP_PASSWORD", password.is_none()),
                        ("SCANOPY_SMTP_EMAIL", email.is_none()),
                        ("SCANOPY_SMTP_RELAY", relay.is_none()),
                    ]
                    .into_iter()
                    .filter_map(|(name, absent)| absent.then_some(name))
                    .collect();

                    // All four absent is the ordinary "no email configured" case and
                    // stays quiet. Anything less is a half-finished setup.
                    if missing.len() < 4 {
                        tracing::warn!(
                            missing = ?missing,
                            "SMTP is partially configured: email is disabled until every \
                             variable is set"
                        );
                    }
                    None
                }
            }
        };

        let billing_service = if let Some(stripe_secret) = config.stripe_secret
            && let Some(webhook_secret) = config.stripe_webhook_secret
        {
            Some(Arc::new(BillingService::new(BillingServiceParams {
                stripe_secret,
                webhook_secret,
                organization_service: organization_service.clone(),
                user_service: user_service.clone(),
                network_service: network_service.clone(),
                host_service: host_service.clone(),
                event_bus: event_bus.clone(),
            })))
        } else {
            None
        };

        let auth_service = Arc::new(AuthService::new(
            user_service.clone(),
            organization_service.clone(),
            email_service.is_some(),
            event_bus.clone(),
            default_self_hosted_plan,
        ));

        // Create Brevo service if API key is configured (before config is consumed)
        let brevo_service = config.brevo_api_key.map(|api_key| {
            Arc::new(BrevoService::new(
                api_key.clone(),
                network_service.clone(),
                host_service.clone(),
                user_service.clone(),
                organization_service.clone(),
                daemon_service.clone(),
                tag_service.clone(),
                user_api_key_service.clone(),
                credential_service.clone(),
            ))
        });

        let posthog_service = if let Some(api_key) = config.posthog_key {
            Some(Arc::new(
                PosthogService::new(
                    api_key,
                    "https://ph.scanopy.net".to_string(),
                    network_service.clone(),
                )
                .await,
            ))
        } else {
            None
        };

        let oidc_service = config.oidc_providers.map(|oidc_providers| {
            Arc::new(OidcService::new(
                oidc_providers,
                &config.public_url,
                auth_service.clone(),
                user_service.clone(),
                event_bus.clone(),
            ))
        });

        let factory = Self {
            user_service,
            auth_service,
            network_service,
            host_service,
            ip_address_service,
            dependency_service,
            subnet_service,
            daemon_service,
            topology_service,
            snapshot_service,
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
            credential_service,
            interface_service,
            vlan_service,
            discovery_digest_service,
        };

        // Register every `Subscriber<Op>` impl in the codebase. Entries are
        // collected via `inventory::submit!` next to each impl block — see
        // `shared/events/registry.rs`.
        register_all_subscribers(factory.all_services(), factory.event_bus.clone()).await?;

        Ok(factory)
    }

    /// All services held by the factory, type-erased for subscriber-registry
    /// dispatch.
    ///
    /// The exhaustive destructure (no `..`) forces this method to be updated
    /// whenever a field is added to `ServiceFactory` — a missed field fails
    /// to compile with "missing field `foo` in pattern". Each binding is then
    /// consumed by the `.add(...)` chain; an unused binding (forgot to add
    /// or intentionally skipping) trips `#[deny(unused_variables)]`.
    #[deny(unused_variables)]
    fn all_services(&self) -> CollectedServices {
        let Self {
            user_service,
            auth_service,
            network_service,
            host_service,
            ip_address_service,
            dependency_service,
            subnet_service,
            daemon_service,
            topology_service,
            snapshot_service,
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
            event_bus: _, // not a service; not subscriber-dispatched
            logging_service,
            metrics_service,
            tag_service,
            entity_tag_service,
            port_service,
            binding_service,
            credential_service,
            interface_service,
            vlan_service,
            discovery_digest_service,
        } = self;

        ServiceCollector::new()
            .with(user_service.clone())
            .with(auth_service.clone())
            .with(network_service.clone())
            .with(host_service.clone())
            .with(ip_address_service.clone())
            .with(dependency_service.clone())
            .with(subnet_service.clone())
            .with(daemon_service.clone())
            .with(topology_service.clone())
            .with(snapshot_service.clone())
            .with(service_service.clone())
            .with(discovery_service.clone())
            .with(daemon_api_key_service.clone())
            .with(user_api_key_service.clone())
            .with(organization_service.clone())
            .with(invite_service.clone())
            .with(share_service.clone())
            .with(logging_service.clone())
            .with(metrics_service.clone())
            .with(tag_service.clone())
            .with(entity_tag_service.clone())
            .with(port_service.clone())
            .with(binding_service.clone())
            .with(credential_service.clone())
            .with(interface_service.clone())
            .with(vlan_service.clone())
            .with(discovery_digest_service.clone())
            .with_optional(oidc_service.clone())
            .with_optional(billing_service.clone())
            .with_optional(email_service.clone())
            .with_optional(brevo_service.clone())
            .with_optional(posthog_service.clone())
            .build()
    }
}
