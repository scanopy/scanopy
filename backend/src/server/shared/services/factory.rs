use crate::server::auth::email_validation::SesEmailValidator;
use crate::server::license::instances::service::LicenseInstanceService;
use crate::server::license::service::LicenseService;
use crate::server::release_notes::service::ReleaseNotesService;
use crate::server::{
    auth::{access_service::AccessService, oidc::OidcService, service::AuthService},
    billing::service::{BillingService, BillingServiceParams},
    bindings::service::BindingService,
    config::ServerConfig,
    credentials::service::CredentialService,
    daemon_api_keys::service::DaemonApiKeyService,
    daemons::service::DaemonService,
    dependencies::service::DependencyService,
    digest::service::DiscoveryDigestService,
    discovery::service::DiscoveryService,
    discovery_warnings::service::DiscoveryWarningService,
    email::{
        feedback::EmailFeedbackService,
        logging::LoggingEmailProvider,
        rate_limit::{self, RateLimitedTransport},
        service::EmailService,
        smtp::SmtpEmailProvider,
        transport::EmailTransport,
    },
    hosts::service::HostService,
    interface_neighbors::service::InterfaceNeighborService,
    interfaces::service::InterfaceService,
    invites::service::InviteService,
    ip_addresses::service::IPAddressService,
    logging::service::LoggingService,
    metrics::service::MetricsService,
    organizations::{service::OrganizationService, setup::OrgSetupService},
    ports::service::PortService,
    posthog::PosthogService,
    services::service::ServiceService,
    setup::service::SetupService,
    shared::{
        events::{
            bus::EventBus,
            registry::{CollectedServices, ServiceCollector, register_all_subscribers},
        },
        storage::factory::StorageFactory,
        trusted_ca::TrustedCaBundle,
    },
    shares::service::ShareService,
    site_assignments::service::SiteAssignmentService,
    sites::service::SiteService,
    snapshots::service::SnapshotService,
    subnets::service::SubnetService,
    tags::{entity_tags::EntityTagService, groups::service::TagGroupService, service::TagService},
    topology::service::main::TopologyService,
    user_api_keys::service::UserApiKeyService,
    users::service::UserService,
    vlans::service::VlanService,
};
use anyhow::Result;
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::{Arc, OnceLock};

// Global Prometheus handle - the recorder can only be installed once per process
static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

pub struct ServiceFactory {
    pub user_service: Arc<UserService>,
    pub auth_service: Arc<AuthService>,
    pub access_service: Arc<AccessService>,
    pub site_assignment_service: Arc<SiteAssignmentService>,
    pub site_service: Arc<SiteService>,
    pub host_service: Arc<HostService>,
    pub ip_address_service: Arc<IPAddressService>,
    pub dependency_service: Arc<DependencyService>,
    pub subnet_service: Arc<SubnetService>,
    pub daemon_service: Arc<DaemonService>,
    pub topology_service: Arc<TopologyService>,
    pub org_setup_service: Arc<OrgSetupService>,
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
    /// Present only when `email_feedback_sns_topic_arn` is configured.
    pub email_feedback_service: Option<Arc<EmailFeedbackService>>,
    /// Present only when `aws_region` and both `email_validation_aws_*` keys are configured.
    pub email_validator: Option<Arc<SesEmailValidator>>,
    pub posthog_service: Option<Arc<PosthogService>>,
    pub event_bus: Arc<EventBus>,
    pub logging_service: Arc<LoggingService>,
    pub metrics_service: Arc<MetricsService>,
    pub tag_service: Arc<TagService>,
    pub tag_group_service: Arc<TagGroupService>,
    pub entity_tag_service: Arc<EntityTagService>,
    pub port_service: Arc<PortService>,
    pub binding_service: Arc<BindingService>,
    pub credential_service: Arc<CredentialService>,
    pub interface_service: Arc<InterfaceService>,
    pub interface_neighbor_service: Arc<InterfaceNeighborService>,
    pub vlan_service: Arc<VlanService>,
    pub discovery_digest_service: Arc<DiscoveryDigestService>,
    pub release_notes_service: Arc<ReleaseNotesService>,
    pub setup_service: Arc<SetupService>,
    /// Present only when a license key applies (self-hosted commercial). A
    /// keyless deployment (community or cloud) has no license service —
    /// licensing is "not required".
    pub license_service: Option<Arc<LicenseService>>,
    /// Server instances per license key: this database's instance id on a
    /// self-hosted server, and the instances checking in on the cloud.
    pub license_instance_service: Arc<LicenseInstanceService>,
    /// Extra CA certificates from `SCANOPY_TRUSTED_CA_BUNDLE`, loaded once here and shared by
    /// every outbound client that reaches an operator-controlled host.
    pub trusted_ca: Option<Arc<TrustedCaBundle>>,
}

impl ServiceFactory {
    pub async fn new(storage: &StorageFactory, config: ServerConfig) -> Result<Self> {
        // Fail closed: a set but unusable bundle stops startup instead of surfacing later as
        // TLS failures on the first OIDC login or email.
        let trusted_ca = config
            .trusted_ca_bundle
            .as_deref()
            .map(TrustedCaBundle::load)
            .transpose()?
            .map(Arc::new);
        if let Some(bundle) = &trusted_ca {
            tracing::info!(
                path = %bundle.path().display(),
                certificates = bundle.len(),
                "Trusting extra CA certificates for outbound TLS"
            );
        }

        let event_bus = Arc::new(EventBus::new());

        let logging_service = Arc::new(LoggingService::new());

        // Initialize Prometheus metrics recorder - uses global singleton since recorder
        // can only be installed once per process (important for tests)
        let prometheus_handle = PROMETHEUS_HANDLE
            .get_or_init(|| {
                let handle = crate::server::metrics::prometheus_builder()
                    .install_recorder()
                    .expect("failed to install Prometheus recorder");
                crate::server::metrics::describe_metrics();
                handle
            })
            .clone();
        let metrics_service = Arc::new(MetricsService::new(prometheus_handle));

        let tag_group_service = Arc::new(TagGroupService::new(
            storage.tag_groups.clone(),
            event_bus.clone(),
        ));
        let tag_service = Arc::new(TagService::new(
            storage.tags.clone(),
            storage.entity_tags.clone(),
            tag_group_service.clone(),
            event_bus.clone(),
        ));
        let _ = tag_group_service.set_tag_service(tag_service.clone());
        let entity_tag_service = Arc::new(EntityTagService::new(
            storage.entity_tags.clone(),
            tag_service.clone(),
        ));

        let daemon_api_key_service = Arc::new(DaemonApiKeyService::new(
            storage.daemon_api_keys.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        let site_assignment_service =
            Arc::new(SiteAssignmentService::new(storage.site_assignments.clone()));

        let user_api_key_service = Arc::new(UserApiKeyService::new(
            storage.user_api_keys.clone(),
            site_assignment_service.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        let organization_service = Arc::new(OrganizationService::new(
            storage.organizations.clone(),
            event_bus.clone(),
        ));

        let license_instance_service = Arc::new(LicenseInstanceService::new(
            storage.license_instances.clone(),
            storage.server_instance.clone(),
        ));

        // Commercial mode is driven by the presence of a license key at
        // runtime — no separate build. No key => free community edition, and no
        // license service at all. `effective_license_key` returns None on cloud,
        // so a stray key can never validate, lock, or reconfigure a cloud
        // deployment.
        let license_service = match config.effective_license_key() {
            Some(key) => Some(Arc::new(
                LicenseService::new(
                    key,
                    organization_service.clone(),
                    config.license_server_url.clone(),
                    trusted_ca.as_deref(),
                    Some(&license_instance_service),
                    config.non_production,
                )
                .await,
            )),
            None => None,
        };

        let invite_service = Arc::new(InviteService::new(
            storage.invites.clone(),
            site_assignment_service.clone(),
            event_bus.clone(),
        ));

        let share_service = Arc::new(ShareService::new(storage.shares.clone(), event_bus.clone()));

        let port_service = Arc::new(PortService::new(storage.ports.clone(), event_bus.clone()));

        let binding_service = Arc::new(BindingService::new(
            storage.bindings.clone(),
            event_bus.clone(),
        ));

        let dependency_service = Arc::new(DependencyService::new(
            storage.dependencies.clone(),
            storage.dependency_members.clone(),
            binding_service.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        let ip_address_service = Arc::new(IPAddressService::new(
            storage.ip_addresses.clone(),
            event_bus.clone(),
        ));

        // Shared by DiscoveryService, which records and reads a run's warnings, and SubnetService,
        // which reads where an assumed range came from.
        let discovery_warning_service = Arc::new(DiscoveryWarningService::new(
            storage.discovery_warnings.clone(),
        ));

        // Built before InterfaceService, which needs it to persist candidates on the discovery ingest
        // path.
        let interface_neighbor_service = Arc::new(InterfaceNeighborService::new(
            storage.interface_neighbor_candidates.clone(),
            storage.interface_neighbor_interfaces.clone(),
            storage.interface_neighbor_hosts.clone(),
        ));

        // InterfaceService needs IPAddressService for validation and InterfaceNeighborService to
        // persist candidate evidence on discovery ingest.
        let interface_service = Arc::new(InterfaceService::new(
            storage.interfaces.clone(),
            event_bus.clone(),
            ip_address_service.clone(),
            interface_neighbor_service.clone(),
        ));

        // VlanService reads each VLAN's serving interfaces through InterfaceService. Built before
        // SubnetService, which reads each subnet's VLANs through it.
        let vlan_service = Arc::new(VlanService::new(
            storage.vlans.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            storage.subnet_vlan.clone(),
            interface_service.clone(),
        ));

        let subnet_service = Arc::new(SubnetService::new(
            storage.subnets.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            // For re-filing addresses a narrowed range displaces. No cycle — `IPAddressService`
            // depends on no service and is constructed above.
            ip_address_service.clone(),
            discovery_warning_service.clone(),
            vlan_service.clone(),
        ));

        let site_service = Arc::new(SiteService::new(
            storage.sites.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
        ));

        let user_service = Arc::new(UserService::new(
            storage.users.clone(),
            site_assignment_service.clone(),
            event_bus.clone(),
        ));

        let access_service = Arc::new(AccessService::new(
            site_assignment_service.clone(),
            user_service.clone(),
            user_api_key_service.clone(),
            invite_service.clone(),
            site_service.clone(),
        ));

        let service_service = Arc::new(ServiceService::new(
            storage.services.clone(),
            binding_service.clone(),
            dependency_service.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            subnet_service.clone(),
        ));

        let credential_service = Arc::new(CredentialService::new(
            storage.credentials.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            site_service.clone(),
            ip_address_service.clone(),
            organization_service.clone(),
            storage.site_credentials.clone(),
            storage.host_credentials.clone(),
        ));

        // Already implements Arc internally due to scheduler + sessions
        let discovery_service = DiscoveryService::new(
            storage.discovery.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            credential_service.clone(),
            site_service.clone(),
            organization_service.clone(),
            discovery_warning_service.clone(),
            crate::server::config::get_deployment_type(&config),
        )
        .await?;

        // The email transport is built before the services that gate on it. EmailService
        // itself comes later: it needs DaemonService and HostService to render content.
        let email_quota = rate_limit::per_second(config.smtp_max_per_second);
        // A configured email log directory wins over SMTP: it is a
        // testing transport you explicitly turn on, so it has to work with no
        // credentials present and override any that happen to be set.
        let email_transport: Option<Box<dyn EmailTransport>> = if let Some(ref email_log_dir) =
            config.email_log_dir
        {
            tracing::warn!(
                dir = %email_log_dir.display(),
                "SCANOPY_EMAIL_LOG_DIR is set: emails will be logged and written to disk, not delivered"
            );
            Some(Box::new(RateLimitedTransport::new(
                Box::new(LoggingEmailProvider::new(Some(email_log_dir.clone()))),
                email_quota,
            )))
        } else {
            // SMTP needs all four values. Every way this can fail used to be silent: the
            // server booted normally, reported itself healthy, and never sent a single
            // email. Say which variables are missing, and say when the transport itself
            // could not be built.
            match (
                config.smtp_username.clone(),
                config.smtp_password.clone(),
                config.smtp_email.clone(),
                config.smtp_relay.clone(),
            ) {
                (Some(username), Some(password), Some(email), Some(relay)) => {
                    match SmtpEmailProvider::new(
                        username,
                        password,
                        email,
                        relay,
                        config.smtp_port,
                        trusted_ca.as_deref(),
                    ) {
                        Ok(smtp_provider) => Some(Box::new(RateLimitedTransport::new(
                            Box::new(smtp_provider),
                            email_quota,
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
        let has_email_service = email_transport.is_some();

        // Create DaemonService with most dependencies directly (not host_service - circular)
        let daemon_service = Arc::new(DaemonService::new(
            storage.daemons.clone(),
            storage.daemon_interfaced_subnets.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            discovery_service.clone(),
            credential_service.clone(),
            subnet_service.clone(),
            vlan_service.clone(),
            site_service.clone(),
            organization_service.clone(),
            user_service.clone(),
            daemon_api_key_service.clone(),
            crate::server::config::get_deployment_type(&config),
            trusted_ca.as_deref(),
            has_email_service,
        ));

        // HostService needs DaemonService
        let host_service = Arc::new(HostService::new(
            storage.hosts.clone(),
            ip_address_service.clone(),
            port_service.clone(),
            service_service.clone(),
            interface_service.clone(),
            interface_neighbor_service.clone(),
            daemon_service.clone(),
            credential_service.clone(),
            subnet_service.clone(),
            vlan_service.clone(),
            site_service.clone(),
            organization_service.clone(),
            event_bus.clone(),
            entity_tag_service.clone(),
            discovery_warning_service.clone(),
        ));

        // Set lazy dependencies to break circular references
        let _ = service_service.set_host_service(host_service.clone());
        let _ = daemon_service.set_host_service(host_service.clone());
        let _ = credential_service.set_host_service(host_service.clone());
        let _ = site_service.set_credential_service(credential_service.clone());
        let _ = discovery_service.set_daemon_service(daemon_service.clone());
        let _ = discovery_service.set_host_service(host_service.clone());

        let topology_service = Arc::new(TopologyService::new(
            host_service.clone(),
            ip_address_service.clone(),
            subnet_service.clone(),
            dependency_service.clone(),
            service_service.clone(),
            port_service.clone(),
            binding_service.clone(),
            interface_service.clone(),
            interface_neighbor_service.clone(),
            tag_service.clone(),
            vlan_service.clone(),
            site_service.clone(),
            organization_service.clone(),
            storage.topologies.clone(),
            event_bus.clone(),
        ));

        let snapshot_service = SnapshotService::new(
            storage.snapshots.clone(),
            event_bus.clone(),
            site_service.clone(),
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
            site_service.clone(),
            discovery_service.clone(),
            credential_service.clone(),
            event_bus.clone(),
        ));
        let _ = discovery_service.set_digest_service(discovery_digest_service.clone());

        for (var, region) in config.aws_region_mismatches() {
            tracing::error!(
                setting = var,
                setting_region = %region,
                aws_region = config.aws_region.as_deref().unwrap_or_default(),
                "An SES setting names a different region than SCANOPY_AWS_REGION"
            );
        }
        let email_validator = SesEmailValidator::from_config(&config)?.map(Arc::new);
        let public_url = config.public_url.clone();
        let deployment_type = crate::server::config::get_deployment_type(&config);
        // The release email takes half, so other email keeps the rest while it goes out.
        let bulk_email_quota = rate_limit::per_second(config.smtp_max_per_second / 2);

        let email_service = email_transport.map(|transport| {
            Arc::new(EmailService::new(
                transport,
                user_service.clone(),
                organization_service.clone(),
                host_service.clone(),
                site_service.clone(),
                service_service.clone(),
                daemon_service.clone(),
                public_url,
                deployment_type,
            ))
        });

        let setup_service = Arc::new(SetupService {
            site_service: site_service.clone(),
            daemon_service: daemon_service.clone(),
            discovery_service: discovery_service.clone(),
            credential_service: credential_service.clone(),
            host_service: host_service.clone(),
            subnet_service: subnet_service.clone(),
            dependency_service: dependency_service.clone(),
            share_service: share_service.clone(),
            user_api_key_service: user_api_key_service.clone(),
            user_service: user_service.clone(),
            invite_service: invite_service.clone(),
            tag_service: tag_service.clone(),
            deployment_type,
        });

        let release_notes_service = Arc::new(ReleaseNotesService::new(
            storage.release_announcements.clone(),
            user_service.clone(),
            email_service.clone(),
            (deployment_type == crate::server::config::DeploymentType::Cloud)
                .then(|| organization_service.clone()),
            bulk_email_quota,
            config.hold_release_email,
        ));

        let email_feedback_service = config
            .email_feedback_sns_topic_arn
            .clone()
            .map(|topic_arn| {
                EmailFeedbackService::new(topic_arn, user_service.clone()).map(Arc::new)
            })
            .transpose()?;

        let billing_service = if let Some(stripe_secret) = config.stripe_secret
            && let Some(webhook_secret) = config.stripe_webhook_secret
        {
            Some(Arc::new(BillingService::new(BillingServiceParams {
                stripe_secret,
                webhook_secret,
                organization_service: organization_service.clone(),
                user_service: user_service.clone(),
                site_service: site_service.clone(),
                host_service: host_service.clone(),
                event_bus: event_bus.clone(),
            })))
        } else {
            None
        };

        let org_setup_service = Arc::new(OrgSetupService::new(
            organization_service.clone(),
            credential_service.clone(),
            tag_service.clone(),
            tag_group_service.clone(),
            site_service.clone(),
            subnet_service.clone(),
            topology_service.clone(),
            daemon_service.clone(),
            deployment_type,
            config.integrated_daemon_url.clone(),
        ));

        let auth_service = Arc::new(AuthService::new(
            user_service.clone(),
            access_service.clone(),
            organization_service.clone(),
            has_email_service,
            event_bus.clone(),
            license_service.clone(),
        ));

        let posthog_service = if let Some(api_key) = config.posthog_key {
            Some(Arc::new(
                PosthogService::new(
                    api_key,
                    "https://ph.scanopy.net".to_string(),
                    site_service.clone(),
                )
                .await,
            ))
        } else {
            None
        };
        // Per-occurrence subscriber failures go to PostHog from Scanopy Cloud only; a self-hosted
        // server with a PostHog key sends nothing it didn't before.
        if deployment_type == crate::server::config::DeploymentType::Cloud
            && let Some(posthog) = &posthog_service
        {
            event_bus.set_failure_reporter(posthog.clone());
        }

        let oidc_service = config
            .oidc_providers
            .map(|oidc_providers| {
                OidcService::new(
                    oidc_providers,
                    &config.public_url,
                    auth_service.clone(),
                    user_service.clone(),
                    event_bus.clone(),
                    trusted_ca.as_deref(),
                )
                .map(Arc::new)
            })
            .transpose()?;

        let factory = Self {
            user_service,
            auth_service,
            access_service,
            site_assignment_service,
            site_service,
            host_service,
            ip_address_service,
            dependency_service,
            subnet_service,
            daemon_service,
            topology_service,
            org_setup_service,
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
            email_feedback_service,
            email_validator,
            posthog_service,
            event_bus,
            logging_service,
            metrics_service,
            tag_service,
            tag_group_service,
            entity_tag_service,
            port_service,
            binding_service,
            credential_service,
            interface_service,
            interface_neighbor_service,
            vlan_service,
            discovery_digest_service,
            release_notes_service,
            setup_service,
            license_service,
            license_instance_service,
            trusted_ca,
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
            access_service: _, // not a Subscriber: called by handlers and auth
            site_assignment_service: _, // not a Subscriber: read by the services above
            site_service,
            host_service,
            ip_address_service,
            dependency_service,
            subnet_service,
            daemon_service,
            topology_service,
            org_setup_service,
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
            email_feedback_service: _, // not a Subscriber: driven by its HTTP endpoint
            email_validator: _,        // not a Subscriber: called by the register handler
            posthog_service,
            event_bus: _, // not a service; not subscriber-dispatched
            logging_service,
            metrics_service,
            tag_service,
            tag_group_service,
            entity_tag_service,
            port_service,
            binding_service,
            credential_service,
            interface_service,
            interface_neighbor_service: _, // not a Subscriber: bespoke service, no EventBusService impl
            vlan_service,
            discovery_digest_service,
            release_notes_service: _, // not a Subscriber: run at boot and by its handlers
            setup_service: _,         // not a Subscriber: read by the dashboard
            license_service: _,       // not a Subscriber: bespoke service, no EventBusService impl
            license_instance_service: _, // not a Subscriber: called by the license handlers
            trusted_ca: _,            // not a service; shared TLS roots
        } = self;

        ServiceCollector::new()
            .with(user_service.clone())
            .with(auth_service.clone())
            .with(site_service.clone())
            .with(host_service.clone())
            .with(ip_address_service.clone())
            .with(dependency_service.clone())
            .with(subnet_service.clone())
            .with(daemon_service.clone())
            .with(topology_service.clone())
            .with(org_setup_service.clone())
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
            .with(tag_group_service.clone())
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
            .with_optional(posthog_service.clone())
            .build()
    }
}
