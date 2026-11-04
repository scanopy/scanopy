use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    credentials::service::CredentialService,
    shared::{
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService},
        storage::generic::GenericPostgresStorage,
    },
    sites::r#impl::Site,
    tags::entity_tags::EntityTagService,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

pub struct SiteService {
    site_storage: Arc<GenericPostgresStorage<Site>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
    /// Assigns each new site its default credentials. Set after construction, since the
    /// credential service depends on this one.
    credential_service: OnceLock<Arc<CredentialService>>,
}

impl EventBusService<Site> for SiteService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, entity: &Site) -> Option<Uuid> {
        Some(entity.id)
    }
    fn get_organization_id(&self, entity: &Site) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }
}

#[async_trait]
impl CrudService<Site> for SiteService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Site>> {
        &self.site_storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    /// Creates the site and its credential assignments. A request that names no credentials
    /// (`credential_ids` omitted) starts with the organization's site-wide defaults (SNMP
    /// `public`); a list, even an empty one, is assigned exactly.
    async fn create(&self, mut entity: Site, authentication: AuthenticatedEntity) -> Result<Site> {
        let Some(credentials) = self.credential_service.get() else {
            tracing::warn!("Credential service not set; the site is created without credentials");
            return self.create_base(entity, authentication).await;
        };
        if entity.base.credential_ids.is_none() {
            entity.base.credential_ids = Some(
                credentials
                    .site_default_credential_ids(entity.base.organization_id)
                    .await?,
            );
        }
        let created = self.create_base(entity, authentication).await?;
        credentials
            .set_site_credentials(
                &created.id,
                created.base.credential_ids.as_deref().unwrap_or_default(),
            )
            .await?;
        Ok(created)
    }
}

impl SiteService {
    pub fn new(
        site_storage: Arc<GenericPostgresStorage<Site>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            site_storage,
            event_bus,
            entity_tag_service,
            credential_service: OnceLock::new(),
        }
    }

    /// Set the credential service dependency after construction (breaks circular dep).
    pub fn set_credential_service(
        &self,
        service: Arc<CredentialService>,
    ) -> Result<(), Arc<CredentialService>> {
        self.credential_service.set(service)
    }

    /// Per-site staleness cutoffs for `site_ids`, as
    /// `(site_id, instant_before_which_last_seen_at_is_stale)`.
    ///
    /// Feeds `StorableFilter::stale_by_site`. Resolved per site because
    /// each configures its own window, and the entity lists span every site
    /// the caller can reach. Sites that no longer exist are simply absent —
    /// their rows then match neither the stale nor the fresh branch, which is
    /// the safe direction for an orphaned FK.
    pub async fn stale_cutoffs(
        &self,
        site_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, chrono::DateTime<chrono::Utc>)>> {
        if site_ids.is_empty() {
            return Ok(Vec::new());
        }
        let now = crate::server::shared::clock::now();
        let sites = self
            .get_all(
                crate::server::shared::storage::filter::StorableFilter::<Site>::new_from_entity_ids(
                    site_ids,
                ),
            )
            .await?;
        Ok(sites.iter().map(|n| (n.id, n.stale_cutoff(now))).collect())
    }
}
