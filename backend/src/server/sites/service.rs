use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService},
        storage::{
            generic::GenericPostgresStorage,
            seed_data::{create_remote_subnet, create_wan_subnet},
        },
    },
    sites::r#impl::Site,
    subnets::service::SubnetService,
    tags::entity_tags::EntityTagService,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

pub struct SiteService {
    site_storage: Arc<GenericPostgresStorage<Site>>,
    subnet_service: Arc<SubnetService>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
}

impl EventBusService<Site> for SiteService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, _entity: &Site) -> Option<Uuid> {
        None
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
}

impl SiteService {
    pub fn new(
        site_storage: Arc<GenericPostgresStorage<Site>>,
        subnet_service: Arc<SubnetService>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            site_storage,
            subnet_service,
            event_bus,
            entity_tag_service,
        }
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
        let now = chrono::Utc::now();
        let sites = self
            .get_all(
                crate::server::shared::storage::filter::StorableFilter::<Site>::new_from_entity_ids(
                    site_ids,
                ),
            )
            .await?;
        Ok(sites.iter().map(|n| (n.id, n.stale_cutoff(now))).collect())
    }

    pub async fn create_organizational_subnets(
        &self,
        site_id: Uuid,
        authenticated: AuthenticatedEntity,
    ) -> Result<()> {
        let wan_subnet = create_wan_subnet(site_id);
        let remote_subnet = create_remote_subnet(site_id);
        // let (dns_host, dns_interfaces, dns_ports, dns_service) =
        //     create_public_dns_host(&wan_subnet, site_id);
        // let (web_host, web_interfaces, web_ports, web_service) =
        //     create_internet_connectivity_host(&wan_subnet, site_id);
        // let (remote_host, remote_interfaces, remote_ports, client_service) =
        //     create_remote_host(&remote_subnet, site_id);

        self.subnet_service
            .create(wan_subnet, authenticated.clone())
            .await?;
        self.subnet_service
            .create(remote_subnet, authenticated.clone())
            .await?;
        // self.host_service
        //     .discover_host(
        //         dns_host,
        //         dns_interfaces,
        //         dns_ports,
        //         vec![dns_service],
        //         authenticated.clone(),
        //     )
        //     .await?;
        // self.host_service
        //     .discover_host(
        //         web_host,
        //         web_interfaces,
        //         web_ports,
        //         vec![web_service],
        //         authenticated.clone(),
        //     )
        //     .await?;
        // self.host_service
        //     .discover_host(
        //         remote_host,
        //         remote_interfaces,
        //         remote_ports,
        //         vec![client_service],
        //         authenticated.clone(),
        //     )
        //     .await?;

        Ok(())
    }
}
