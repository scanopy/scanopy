use crate::server::{
    interfaces::service::InterfaceService,
    shared::services::scan_context::ScanContext,
    shared::{
        entities::EntityDiscriminants,
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService, ObservationRefresher},
        storage::{
            child::{GenericChildStorage, ParentRef},
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{PaginatedResult, Storable, Storage, Unique},
        },
        types::entities::EntitySource,
    },
    tags::entity_tags::EntityTagService,
    vlans::{
        handlers::{VlanDiscoveryItem, VlanDiscoveryResponse, VlanDiscoveryResponseItem},
        r#impl::{
            base::{Vlan, VlanName, VlanNameValue},
            subnet_vlans::{SubnetVlanRecord, SubnetVlanRecordBase},
        },
    },
};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct VlanService {
    storage: Arc<GenericPostgresStorage<Vlan>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
    subnet_vlan_storage: Arc<GenericChildStorage<SubnetVlanRecord>>,
    interface_service: Arc<InterfaceService>,
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

#[async_trait]
impl CrudService<Vlan> for VlanService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Vlan>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Vlan>, anyhow::Error> {
        match self.storage().get_by_id(id).await? {
            Some(mut vlan) => {
                self.hydrate_links(std::slice::from_mut(&mut vlan)).await?;
                self.hydrate_tags(&mut vlan).await?;
                Ok(Some(vlan))
            }
            None => Ok(None),
        }
    }

    async fn get_all(&self, filter: StorableFilter<Vlan>) -> Result<Vec<Vlan>, anyhow::Error> {
        let mut vlans = self.storage().get_all(filter).await?;
        self.hydrate_links(&mut vlans).await?;
        self.bulk_hydrate_tags(&mut vlans, None).await?;
        Ok(vlans)
    }

    async fn get_unique(
        &self,
        filter: StorableFilter<Vlan>,
    ) -> Result<Unique<Vlan>, anyhow::Error> {
        match self.storage().get_unique(filter).await? {
            Unique::One(mut vlan) => {
                self.hydrate_links(std::slice::from_mut(&mut vlan)).await?;
                self.hydrate_tags(&mut vlan).await?;
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
        self.hydrate_links(&mut paginated.items).await?;
        self.bulk_hydrate_tags(&mut paginated.items, None).await?;
        Ok(paginated)
    }
}

impl VlanService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<Vlan>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        subnet_vlan_storage: Arc<GenericChildStorage<SubnetVlanRecord>>,
        interface_service: Arc<InterfaceService>,
    ) -> Self {
        Self {
            storage,
            event_bus,
            entity_tag_service,
            subnet_vlan_storage,
            interface_service,
        }
    }

    /// Fill each VLAN's read-only `subnet_ids` (from the `subnet_vlans` junction) and
    /// `interface_ids` (interfaces naming it as native or member VLAN). Both read live links.
    async fn hydrate_links(&self, vlans: &mut [Vlan]) -> Result<()> {
        if vlans.is_empty() {
            return Ok(());
        }
        let ids: Vec<Uuid> = vlans.iter().map(|v| v.id).collect();
        let mut subnets_map = self.subnet_ids_for_vlans(&ids).await?;
        let mut interfaces_map = self.interface_service.get_ids_for_vlans(&ids).await?;
        for vlan in vlans.iter_mut() {
            vlan.base.subnet_ids = subnets_map.remove(&vlan.id).unwrap_or_default();
            vlan.base.interface_ids = interfaces_map.remove(&vlan.id).unwrap_or_default();
        }
        Ok(())
    }

    /// Subnet ID → the VLANs linked to it through the `subnet_vlans` junction (live links), for
    /// `SubnetService` to hydrate `Subnet::vlan_ids`.
    pub async fn get_ids_for_subnets(
        &self,
        subnet_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        Ok(self
            .subnet_vlan_storage
            .get_for_parents(subnet_ids)
            .await?
            .into_iter()
            .map(|(subnet_id, records)| (subnet_id, records.iter().map(|r| r.vlan_id()).collect()))
            .collect())
    }

    /// VLAN id → the subnets it is linked to, live links only.
    pub async fn subnet_ids_for_vlans(
        &self,
        vlan_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if vlan_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let filter =
            StorableFilter::<SubnetVlanRecord>::new_from_uuids_column("vlan_id", vlan_ids).live();
        let records = self.subnet_vlan_storage.inner().get_all(filter).await?;

        let mut result: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for record in records {
            result
                .entry(record.vlan_id())
                .or_default()
                .push(record.subnet_id());
        }
        Ok(result)
    }

    /// The VLANs a subnet is linked to, live links only.
    pub async fn vlan_ids_for_subnet(&self, subnet_id: &Uuid) -> Result<Vec<Uuid>> {
        let records = self.subnet_vlan_storage.get_for_parent(subnet_id).await?;
        Ok(records.iter().map(|r| r.vlan_id()).collect())
    }

    /// Make `vlan_ids` a subnet's linked VLANs. Soft-closes live links not in the set, so the
    /// "this VLAN was associated with this subnet during [valid_from, valid_to]" answer stays
    /// queryable as of any time; inserts live links for additions; leaves unchanged links alone.
    pub async fn set_subnet_vlans(&self, subnet_id: &Uuid, vlan_ids: &[Uuid]) -> Result<()> {
        self.subnet_vlan_storage
            .sync_for_parent(
                &ParentRef::new(EntityDiscriminants::Subnet, *subnet_id),
                vlan_ids,
                SubnetVlanRecord::vlan_id,
                |vlan_id| SubnetVlanRecord::new(SubnetVlanRecordBase::new(*subnet_id, vlan_id)),
            )
            .await
    }

    /// Bulk-insert pre-built subnet↔VLAN links in a single INSERT, with no per-subnet lock or
    /// delete-first pass. For seed paths (demo populate) on a freshly reset org. Callers must
    /// pre-dedup on `(subnet_id, vlan_id)` (the partial unique index allows at most one live
    /// link per pair).
    pub async fn create_subnet_links(&self, records: &[SubnetVlanRecord]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        self.subnet_vlan_storage
            .inner()
            .create_many(records)
            .await?;
        Ok(())
    }

    /// Upsert every VLAN one daemon submission reported, in order. Shared by the DaemonPoll
    /// endpoint and ServerPoll entity processing; both have already checked the daemon may write
    /// to `site_id`.
    pub async fn upsert_discovery_batch(
        &self,
        site_id: Uuid,
        organization_id: Uuid,
        items: Vec<VlanDiscoveryItem>,
        scan_ctx: Option<&ScanContext>,
    ) -> Result<VlanDiscoveryResponse> {
        let mut vlans = Vec::new();
        for item in items {
            let vlan = self
                .upsert_from_discovery(
                    site_id,
                    organization_id,
                    item.vlan_number,
                    VlanName::new(VlanNameValue(item.name), item.name_source),
                    scan_ctx,
                )
                .await?;
            vlans.push(VlanDiscoveryResponseItem {
                vlan_number: vlan.base.vlan_number,
                id: vlan.id,
            });
        }

        // A VLAN reported again unchanged is not written above, so its Last seen would stay at
        // the scan that created it and the digest would call it stale while every scan still
        // reports it. ServerPoll processing has no `ScanContext`; the time of processing stands in.
        let scan_time =
            scan_ctx.map_or_else(crate::server::shared::clock::now, |ctx| ctx.scan_time);
        let ids: Vec<Uuid> = vlans.iter().map(|v| v.id).collect();
        self.refresh_observed(
            StorableFilter::<Vlan>::new_from_uuids_column("id", &ids),
            scan_time,
        )
        .await?;

        Ok(VlanDiscoveryResponse { vlans })
    }

    /// Upsert a VLAN from discovery. Creates if new; renames an existing one only where the
    /// reported name outranks the stored one (`Attributed::apply_in_place`), so a name a person
    /// typed survives every scan.
    /// Returns the VLAN (existing or newly created).
    ///
    /// `scan_ctx` carries the per-submission `scan_time` (see `ScanContext`)
    /// and is used to stamp `valid_from` / `created_at` on freshly inserted
    /// rows so all VLANs in the same daemon submission share a consistent
    /// SCD2 origination timestamp. When `None`, defaults to `crate::server::shared::clock::now()` for
    /// each new row (legacy callers + tests).
    pub async fn upsert_from_discovery(
        &self,
        site_id: Uuid,
        organization_id: Uuid,
        vlan_number: u16,
        name: VlanName,
        scan_ctx: Option<&ScanContext>,
    ) -> Result<Vlan> {
        use crate::server::shared::storage::snapshot::DiscoveryTracked;

        // SCD2: natural-key match (site_id + vlan_number) against live rows.
        let filter = StorableFilter::<Vlan>::new_from_uuid_column("site_id", &site_id)
            .u16_column("vlan_number", vlan_number)
            .live();

        if let Some(existing) = self.storage.get_unique(filter).await?.at_most_one()? {
            let mut updated = existing.clone();
            if updated.base.name.apply_in_place(name) {
                updated.updated_at = crate::server::shared::clock::now();
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
            interface_ids: Vec::new(),
            tags: Vec::new(),
        });
        if let Some(ctx) = scan_ctx {
            vlan.originate_scan_timestamps(ctx.scan_time);
        }
        self.storage.create(&vlan).await?;
        Ok(vlan)
    }
}
