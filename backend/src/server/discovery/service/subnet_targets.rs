//! A scan's subnet targets: written as `discovery_subnets` rows when the scan is saved, and read
//! back onto `discovery_type`'s `subnet_ids` wherever a scan is read through this service or
//! dispatched. See [`DiscoverySubnet`].
use super::*;
use crate::server::shared::relations as R;

impl DiscoveryService {
    /// Replace `discovery`'s target rows with the subnets its type names. A historical run keeps
    /// the targets it ran with in its own record, which a later subnet deletion leaves alone.
    pub async fn save_subnet_targets(&self, discovery: &Discovery) -> Result<()> {
        if !discovery.base.run_type.is_live_config() {
            return Ok(());
        }
        let mut discovery_type = discovery.base.discovery_type.clone();
        let Some(targets) = discovery_type.subnet_targets_mut() else {
            return Ok(());
        };
        let rows: Vec<DiscoverySubnet> = targets
            .iter()
            .flatten()
            .copied()
            .collect::<HashSet<Uuid>>()
            .into_iter()
            .map(|subnet_id| DiscoverySubnet {
                discovery_id: discovery.id,
                subnet_id,
            })
            .collect();
        self.subnet_target_storage
            .replace_for(&R::DISCOVERY_SUBNET_DISCOVERY.at(discovery.id), &rows)
            .await?;
        Ok(())
    }

    /// Set each scan's subnet targets from its rows: the targets whose subnets still exist, and
    /// none once the last of them is deleted. A historical run reads the targets it recorded.
    pub(crate) async fn attach_subnet_targets(&self, discoveries: &mut [Discovery]) -> Result<()> {
        let ids: Vec<Uuid> = discoveries
            .iter()
            .filter(|d| d.base.run_type.is_live_config())
            .filter(|d| d.base.discovery_type.has_subnet_targets())
            .map(|d| d.id)
            .collect();
        if ids.is_empty() {
            return Ok(());
        }
        // Ordered by hand: the junction has no `created_at` for the default order to sort by.
        let mut by_scan: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for row in self
            .subnet_target_storage
            .get_all_ordered(
                StorableFilter::new_from_uuids_column(R::DISCOVERY_SUBNET_DISCOVERY.column, &ids),
                "subnet_id ASC",
            )
            .await?
        {
            by_scan
                .entry(row.discovery_id)
                .or_default()
                .push(row.subnet_id);
        }
        for discovery in discoveries
            .iter_mut()
            .filter(|d| d.base.run_type.is_live_config())
        {
            let Some(targets) = discovery.base.discovery_type.subnet_targets_mut() else {
                continue;
            };
            let ids = by_scan.remove(&discovery.id).unwrap_or_default();
            *targets = (!ids.is_empty()).then_some(ids);
        }
        Ok(())
    }
}
