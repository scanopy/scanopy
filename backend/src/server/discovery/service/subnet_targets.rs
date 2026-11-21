//! A scan's subnet targets: written as `discovery_subnets` rows when the scan is saved, and read
//! back onto `discovery_type`'s `subnet_ids` wherever a scan is read through this service.
//!
//! A scan always sweeps its daemon's interfaced subnets. The rows hold only the routed subnets
//! added on top of them, so a configuration's `subnet_ids` lists those, and `None` means the
//! interfaced subnets alone. Dispatch sends the daemon the union (see
//! [`resolve_scan_targets`]). A run's rows hold every subnet it swept (see
//! [`swept_subnet_ids`]), and a deleted subnet leaves the runs that swept it. See
//! [`DiscoverySubnet`].
use super::*;
use crate::server::shared::relations as R;

/// The `subnet_ids` a session is dispatched with: `None` when the scan has no routed targets,
/// so the daemon sweeps the networks its interfaces are on as it finds them at session start,
/// and otherwise the daemon's interfaced subnets plus the routed ones.
pub(crate) fn resolve_scan_targets(
    routed: Option<&[Uuid]>,
    interfaced: &[Uuid],
) -> Option<Vec<Uuid>> {
    let routed = routed.filter(|ids| !ids.is_empty())?;
    let mut ids: Vec<Uuid> = interfaced.to_vec();
    for id in routed {
        if !ids.contains(id) {
            ids.push(*id);
        }
    }
    Some(ids)
}

/// The `subnet_ids` a run's record keeps: the list it was dispatched with, or for a run
/// dispatched with none, the subnets the daemon reported sweeping in that session. `None` when
/// the daemon reported none, which an older daemon does.
pub(crate) fn swept_subnet_ids(
    dispatched: Option<&[Uuid]>,
    scanned: Option<&ScannedEntityIds>,
) -> Option<Vec<Uuid>> {
    if let Some(ids) = dispatched {
        return Some(ids.to_vec());
    }
    scanned
        .map(|s| s.subnet_ids.clone())
        .filter(|ids| !ids.is_empty())
}

/// The `discovery_type` a finished session's record keeps, with the subnets it swept. See
/// [`swept_subnet_ids`].
pub(crate) fn recorded_discovery_type(session: &DiscoveryUpdatePayload) -> DiscoveryType {
    let mut discovery_type = session.discovery_type.clone();
    if let Some(targets) = discovery_type.subnet_targets_mut() {
        *targets = swept_subnet_ids(targets.as_deref(), session.scanned.as_ref());
    }
    discovery_type
}

/// A run's swept subnets as read back: `None` when it recorded none (a run from before runs
/// recorded them, or one whose daemon reported none), and otherwise the recorded subnets that
/// still exist, which is empty once every one of them is deleted.
pub(crate) fn run_subnet_targets(recorded: Option<&[Uuid]>, rows: Vec<Uuid>) -> Option<Vec<Uuid>> {
    recorded.map(|_| rows)
}

impl DiscoveryService {
    /// Replace `discovery`'s rows with the subnets its type names. A scan's daemon's interfaced
    /// subnets are left out, because every scan sweeps them. A historical run keeps every subnet
    /// it swept, interfaced ones included, except any deleted since the run was dispatched.
    pub async fn save_subnet_targets(&self, discovery: &Discovery) -> Result<()> {
        let mut discovery_type = discovery.base.discovery_type.clone();
        let Some(targets) = discovery_type.subnet_targets_mut() else {
            return Ok(());
        };
        let listed: HashSet<Uuid> = targets.iter().flatten().copied().collect();
        let kept: HashSet<Uuid> = if listed.is_empty() {
            listed
        } else if discovery.base.run_type.is_live_config() {
            let interfaced: HashSet<Uuid> = match self.daemon_service.get() {
                Some(daemon_service) => daemon_service
                    .interfaced_subnet_ids(&discovery.base.daemon_id)
                    .await?
                    .into_iter()
                    .collect(),
                None => HashSet::new(),
            };
            listed.difference(&interfaced).copied().collect()
        } else {
            let ids: Vec<Uuid> = listed.into_iter().collect();
            let filter = StorableFilter::<Subnet>::new_from_site_ids(&[discovery.base.site_id])
                .entity_ids(&ids)
                .live();
            self.subnet_service
                .get_all(filter)
                .await?
                .into_iter()
                .map(|s| s.id)
                .collect()
        };
        let rows: Vec<DiscoverySubnet> = kept
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

    /// Set each scan's routed subnet targets from its rows: the targets whose subnets still
    /// exist, and none once the last of them is deleted. A historical run gets the subnets it
    /// swept that still exist, on its record and on the session it holds. See
    /// [`run_subnet_targets`].
    pub(crate) async fn attach_subnet_targets(&self, discoveries: &mut [Discovery]) -> Result<()> {
        let ids: Vec<Uuid> = discoveries
            .iter()
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
        for discovery in discoveries.iter_mut() {
            let rows = by_scan.remove(&discovery.id).unwrap_or_default();
            let base = &mut discovery.base;
            if let RunType::Historical { results } = &mut base.run_type {
                let Some(targets) = base.discovery_type.subnet_targets_mut() else {
                    continue;
                };
                *targets = run_subnet_targets(targets.as_deref(), rows);
                if let Some(session_targets) = results.discovery_type.subnet_targets_mut() {
                    *session_targets = targets.clone();
                }
            } else if let Some(targets) = base.discovery_type.subnet_targets_mut() {
                *targets = (!rows.is_empty()).then_some(rows);
            }
        }
        Ok(())
    }

    /// Turn a scan's routed targets into the list its session is dispatched with. See
    /// [`resolve_scan_targets`]. Fails when the daemon's interfaced subnets can't be read, since
    /// dispatching the routed ones alone would skip the daemon's own networks.
    pub(crate) async fn expand_subnet_targets(
        &self,
        discovery_type: &mut DiscoveryType,
        daemon_id: Uuid,
        site_id: Uuid,
    ) -> Result<()> {
        let Some(targets) = discovery_type.subnet_targets_mut() else {
            return Ok(());
        };
        if targets.as_ref().is_none_or(|t| t.is_empty()) {
            *targets = None;
            return Ok(());
        }
        let daemon_service = self
            .daemon_service
            .get()
            .ok_or_else(|| anyhow!("Daemon service not initialized"))?;
        let interfaced = daemon_service
            .scan_interfaced_subnet_ids(&daemon_id, site_id)
            .await?;
        *targets = resolve_scan_targets(targets.as_deref(), &interfaced);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: usize) -> Vec<Uuid> {
        (0..n).map(|_| Uuid::new_v4()).collect()
    }

    /// A scan with no routed targets leaves the daemon to sweep its interfaces as it finds them;
    /// one with routed targets sweeps those and the daemon's interfaced subnets, once each.
    #[test]
    fn a_scan_sweeps_its_daemons_networks_and_its_routed_ones() {
        let interfaced = ids(2);
        let routed = ids(1);

        assert_eq!(resolve_scan_targets(None, &interfaced), None);
        assert_eq!(resolve_scan_targets(Some(&[]), &interfaced), None);

        let with_overlap = [routed[0], interfaced[1]];
        let swept = resolve_scan_targets(Some(&with_overlap), &interfaced).unwrap();
        assert_eq!(swept.len(), 3);
        assert!(
            interfaced
                .iter()
                .chain(&routed)
                .all(|id| swept.contains(id))
        );
    }

    /// A run keeps the list it was dispatched with, and a run dispatched with none names what
    /// the daemon reported sweeping, whatever the daemon's interfaces are later.
    #[test]
    fn a_run_records_the_subnets_it_swept() {
        let dispatched = ids(2);
        let reported = ScannedEntityIds {
            subnet_ids: ids(1),
            ..Default::default()
        };

        assert_eq!(
            swept_subnet_ids(Some(&dispatched), Some(&reported)),
            Some(dispatched.clone())
        );
        assert_eq!(
            swept_subnet_ids(None, Some(&reported)),
            Some(reported.subnet_ids.clone())
        );
        assert_eq!(
            swept_subnet_ids(None, Some(&ScannedEntityIds::default())),
            None
        );
        assert_eq!(swept_subnet_ids(None, None), None);
    }

    /// A run that recorded no subnets reads back as none, which the UI shows as the daemon's
    /// interfaced subnets; one whose recorded subnets were all deleted reads back as an empty
    /// list, not as none.
    #[test]
    fn a_run_whose_subnets_were_deleted_still_reads_as_having_recorded_some() {
        let rows = ids(1);
        assert_eq!(run_subnet_targets(None, Vec::new()), None);
        assert_eq!(
            run_subnet_targets(Some(&ids(2)), Vec::new()),
            Some(Vec::new())
        );
        assert_eq!(run_subnet_targets(Some(&ids(2)), rows.clone()), Some(rows));
    }
}
