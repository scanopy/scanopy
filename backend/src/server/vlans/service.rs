use crate::server::{
    shared::{
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{PaginatedResult, Storable, Storage, Unique},
        },
        types::entities::EntitySource,
    },
    vlans::r#impl::{base::Vlan, subnet_vlans::SubnetVlanStorage},
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

pub struct VlanService {
    storage: Arc<GenericPostgresStorage<Vlan>>,
    event_bus: Arc<EventBus>,
    pub subnet_vlan_storage: Arc<SubnetVlanStorage>,
}

impl EventBusService<Vlan> for VlanService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, entity: &Vlan) -> Option<Uuid> {
        Some(entity.base.site_id)
    }

    fn get_organization_id(&self, entity: &Vlan) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }
}

/// Helper: hydrate associated subnet IDs for a batch of VLANs.
async fn hydrate_subnet_ids_for_batch(
    subnet_vlan_storage: &SubnetVlanStorage,
    vlans: &mut [Vlan],
) -> Result<()> {
    if vlans.is_empty() {
        return Ok(());
    }
    let ids: Vec<Uuid> = vlans.iter().map(|v| v.id).collect();
    let subnets_map = subnet_vlan_storage.get_for_vlans(&ids).await?;
    for vlan in vlans.iter_mut() {
        vlan.base.subnet_ids = subnets_map.get(&vlan.id).cloned().unwrap_or_default();
    }
    Ok(())
}

#[async_trait]
impl CrudService<Vlan> for VlanService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Vlan>> {
        &self.storage
    }

    fn entity_tag_service(
        &self,
    ) -> Option<&Arc<crate::server::tags::entity_tags::EntityTagService>> {
        None
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Vlan>, anyhow::Error> {
        match self.storage().get_by_id(id).await? {
            Some(mut vlan) => {
                vlan.base.subnet_ids = self
                    .subnet_vlan_storage
                    .get_subnet_ids_for_vlan(&vlan.id)
                    .await?;
                Ok(Some(vlan))
            }
            None => Ok(None),
        }
    }

    async fn get_all(&self, filter: StorableFilter<Vlan>) -> Result<Vec<Vlan>, anyhow::Error> {
        let mut vlans = self.storage().get_all(filter).await?;
        hydrate_subnet_ids_for_batch(&self.subnet_vlan_storage, &mut vlans).await?;
        Ok(vlans)
    }

    async fn get_unique(
        &self,
        filter: StorableFilter<Vlan>,
    ) -> Result<Unique<Vlan>, anyhow::Error> {
        match self.storage().get_unique(filter).await? {
            Unique::One(mut vlan) => {
                vlan.base.subnet_ids = self
                    .subnet_vlan_storage
                    .get_subnet_ids_for_vlan(&vlan.id)
                    .await?;
                Ok(Unique::One(vlan))
            }
            Unique::None => Ok(Unique::None),
            Unique::Multiple => Ok(Unique::Multiple),
        }
    }

    async fn get_paginated(
        &self,
        filter: StorableFilter<Vlan>,
    ) -> Result<PaginatedResult<Vlan>, anyhow::Error> {
        self.get_paginated_ordered(filter, "created_at ASC").await
    }

    async fn get_paginated_ordered(
        &self,
        filter: StorableFilter<Vlan>,
        order_by: &str,
    ) -> Result<PaginatedResult<Vlan>, anyhow::Error> {
        let mut paginated = self.storage().get_paginated(filter, order_by).await?;
        hydrate_subnet_ids_for_batch(&self.subnet_vlan_storage, &mut paginated.items).await?;
        Ok(paginated)
    }
}

impl VlanService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<Vlan>>,
        event_bus: Arc<EventBus>,
        subnet_vlan_storage: Arc<SubnetVlanStorage>,
    ) -> Self {
        Self {
            storage,
            event_bus,
            subnet_vlan_storage,
        }
    }

    /// Upsert a VLAN from discovery. Creates if new, updates name if changed.
    /// Returns the VLAN (existing or newly created).
    ///
    /// `scan_ctx` carries the per-submission `scan_time` (see `ScanContext`)
    /// and is used to stamp `valid_from` / `created_at` on freshly inserted
    /// rows so all VLANs in the same daemon submission share a consistent
    /// SCD2 origination timestamp. When `None`, defaults to `Utc::now()` for
    /// each new row (legacy callers + tests).
    pub async fn upsert_from_discovery(
        &self,
        site_id: Uuid,
        organization_id: Uuid,
        vlan_number: u16,
        name: String,
        scan_ctx: Option<&crate::server::shared::services::scan_context::ScanContext>,
    ) -> Result<Vlan> {
        use crate::server::shared::storage::snapshot::DiscoveryTracked;

        // SCD2: natural-key match (site_id + vlan_number) against live rows.
        let filter = StorableFilter::<Vlan>::new_from_uuid_column("site_id", &site_id)
            .u16_column("vlan_number", vlan_number)
            .live();

        if let Some(existing) = self.storage.get_unique(filter).await?.at_most_one()? {
            if existing.base.name != name {
                let mut updated = existing.clone();
                updated.base.name = name;
                updated.updated_at = chrono::Utc::now();
                if let Some(ctx) = scan_ctx {
                    updated.refresh_scan_timestamps(ctx.scan_time);
                }
                self.storage.update(&mut updated).await?;
                return Ok(updated);
            }
            return Ok(existing);
        }

        // Create new VLAN. Stamp originating SCD2 timestamps from scan_ctx so
        // every VLAN in this submission shares a consistent valid_from.
        let mut vlan = <Vlan as Storable>::new(crate::server::vlans::r#impl::base::VlanBase {
            vlan_number,
            name,
            description: None,
            site_id,
            organization_id,
            source: EntitySource::Discovery,
            subnet_ids: Vec::new(),
        });
        if let Some(ctx) = scan_ctx {
            vlan.originate_scan_timestamps(ctx.scan_time);
        }
        self.storage.create(&vlan).await?;
        Ok(vlan)
    }
}
