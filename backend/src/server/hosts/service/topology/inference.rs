//! Turning far ends the LLDP pass could not place into subnets and hosts.
//!
//! The *rule* for what range an address implies lives in
//! [`crate::server::subnets::r#impl::inference`], because a controller-reported address and a
//! neighbour-advertised one are placed by the same rule. What is here is the part only this pass
//! has: the far ends it collected, the evidence it can attach to a warning, and the hosts it mints
//! for them.

use crate::daemon::discovery::types::warnings::{ProvisionalSubnet, UnconfirmedHost};
use crate::server::daemons::r#impl::api::ScannedEntityIds;
use crate::server::interfaces::r#impl::base::InterfaceBase;
use crate::server::ip_addresses::r#impl::base::{IPAddressBase, MacEvidence, MacEvidenceValue};
use crate::server::sites::r#impl::Site;
use crate::server::subnets::r#impl::{
    base::{SubnetBase, SubnetCidr, SubnetCidrValue},
    inference::{UnplacedFarEnd, infer_ranges, placeable_subnet},
    types::SubnetType,
};

use super::*;
use crate::server::subnets::r#impl::base::{SubnetName, SubnetTypeAttributed};

impl HostService {
    /// Create a subnet for every range these far ends imply, and report each one.
    ///
    /// Runs after the resolution pass rather than inside it: the ranges are a property of the whole
    /// site's unplaced far ends, not of any one interface, and pooling them is the entire reason
    /// this lives on the server (see the module docs).
    ///
    /// A failure to create one range never fails the pass. Link resolution is the caller's actual
    /// job, and losing every resolved link because a subnet insert raced another session would be a
    /// far worse outcome than one missing range that the next scan re-proposes anyway.
    pub(super) async fn infer_far_end_subnets(
        &self,
        site_id: Uuid,
        far_ends: Vec<UnplacedFarEnd>,
        limit_ctx: Option<&HostLimitContext>,
        scan_time: DateTime<Utc>,
    ) -> Result<InferenceOutcome> {
        // No early return on an empty far-end list: the standing report below covers ranges the
        // ingest path inferred, and a scan that placed every neighbour is exactly when those are
        // the only ones left to tell an operator about.
        let live = self
            .subnet_service
            .get_all(StorableFilter::<Subnet>::new_from_site_ids(&[site_id]).live())
            .await?;

        let mut warnings = Vec::new();
        // Evidence per range this pass created, folded into the standing report below.
        let mut evidence: HashMap<Uuid, ProvisionalSubnet> = HashMap::new();
        // Which subnet each far end's address landed in, keyed the way minting dedups. Far ends
        // that published no address never appear here and are minted without one.
        let mut placements: HashMap<String, Uuid> = HashMap::new();
        // What this step stored or found, handed to the session as entities the scan touched.
        let mut observed = ScannedEntityIds::default();
        for range in infer_ranges(&far_ends, &live) {
            let mut subnet = Subnet::new(SubnetBase {
                vlan_ids: Vec::new(),
                // The whole point: a range nothing read, only inferred, so the row asks to be
                // confirmed rather than asserting itself.
                cidr: SubnetCidr::new(
                    SubnetCidrValue(range.cidr),
                    AttributeSource::LldpNeighbourAddress,
                ),
                site_id,
                name: SubnetName::unattributed(range.cidr.to_string()),
                description: None,
                // Not `Management` even though a management address is what usually produces this:
                // on a flat network that address is simply the device's LAN address, and typing the
                // subnet would be a second guess stacked on the first.
                subnet_type: SubnetTypeAttributed::unattributed(SubnetType::Unknown),
                virtualization_service_id: None,
                source: EntitySource::Inferred {
                    via: InferredVia::ReportedAddress,
                },
                tags: Vec::new(),
            });
            subnet.last_seen_at = crate::server::shared::clock::now();

            let created = match self
                .subnet_service
                .create(subnet, AuthenticatedEntity::System)
                .await
            {
                Ok(created) => created,
                Err(e) => {
                    tracing::warn!(
                        site_id = %site_id,
                        cidr = %range.cidr,
                        error = %e,
                        "Could not create inferred subnet; leaving its far ends unplaced"
                    );
                    continue;
                }
            };

            observed.found_subnet_ids.push(created.id);
            for addressed in &range.far_ends {
                placements.insert(addressed.far_end.chassis_id.clone(), created.id);
            }

            evidence.insert(
                created.id,
                ProvisionalSubnet {
                    cidr: range.cidr.to_string(),
                    subnet_id: Some(created.id),
                    addresses: range
                        .far_ends
                        .iter()
                        .map(|f| f.address.to_string())
                        .collect(),
                    sys_names: range
                        .far_ends
                        .iter()
                        .filter_map(|f| f.far_end.sys_name.clone())
                        .collect(),
                    seen_by_host_ids: range.far_ends.iter().map(|f| f.far_end.host_id).collect(),
                    widened_by_vlan: range.widened_by_vlan,
                },
            );
        }

        // A far end whose address falls in a range that already exists: `infer_ranges` leaves it
        // out, so without this it is minted with no address and the range it sits in is never
        // reported. The live rows were read before this pass created any, so a range created above
        // is never matched here.
        for far_end in &far_ends {
            if placements.contains_key(&far_end.chassis_id) {
                continue;
            }
            let Some(range) = far_end
                .address
                .and_then(|address| placeable_subnet(&live, address))
            else {
                continue;
            };
            placements.insert(far_end.chassis_id.clone(), range.id);
            if !observed.found_subnet_ids.contains(&range.id) {
                observed.found_subnet_ids.push(range.id);
            }
        }

        // Every far end, not only those a range was created for. A device that published no address
        // is still one nothing has contacted, and minting it is what turns an unresolved neighbour
        // into a host the next pass can place and the topology can draw.
        observed.merge(
            self.mint_far_end_hosts(site_id, &far_ends, &placements, limit_ctx, scan_time)
                .await,
        );

        warnings.extend(self.provisional_subnet_warnings(site_id, evidence).await?);
        Ok(InferenceOutcome { observed, warnings })
    }

    /// Report every range on this site still waiting to be confirmed, not only the ones this
    /// pass created.
    ///
    /// A provisional CIDR is a standing state, not a per-scan delta — the same reasoning that makes
    /// `LldpNeighbourNotFound` a standing population. Reporting only new ones would mean an
    /// operator who did not act on the scan that proposed a range never hears about it again, and
    /// would leave ranges inferred on the ingest path silent entirely: nothing there belongs to a
    /// session, so nothing there can ride out on one's scan record.
    ///
    /// Far-end evidence is attached where this pass has it and omitted where it does not, which is
    /// the honest shape: a range inferred from a controller-reported address has no neighbour that
    /// advertised it.
    async fn provisional_subnet_warnings(
        &self,
        site_id: Uuid,
        mut evidence: HashMap<Uuid, ProvisionalSubnet>,
    ) -> Result<Vec<DiscoveryWarning>> {
        let live = self
            .subnet_service
            .get_all(StorableFilter::<Subnet>::new_from_site_ids(&[site_id]).live())
            .await?;

        Ok(live
            .into_iter()
            .filter(|s| s.base.source.confirmation().is_unconfirmed())
            .map(|s| {
                let detail = evidence.remove(&s.id).unwrap_or(ProvisionalSubnet {
                    cidr: s.base.cidr.to_string(),
                    subnet_id: Some(s.id),
                    addresses: Vec::new(),
                    sys_names: Vec::new(),
                    seen_by_host_ids: Vec::new(),
                    widened_by_vlan: false,
                });
                DiscoveryWarning::ProvisionalSubnetInferred(detail)
            })
            .collect())
    }
}

impl HostService {
    /// Fill what the resolve modal shows on the unconfirmed hosts among `responses`: the hosts
    /// whose report each one is known from (`reported_by_host_ids`), and the scan that first
    /// reported it (`found_by_discovery_id`). A forwarding-table host is reported by the switches
    /// whose ports learned one of its MACs; a neighbour-advertised host by the hosts whose resolved
    /// neighbour rows name it or one of its interfaces.
    pub(crate) async fn fill_unconfirmed_evidence(
        &self,
        responses: &mut [HostResponse],
    ) -> Result<()> {
        let unconfirmed: Vec<(Uuid, Uuid, InferredVia)> = responses
            .iter()
            .filter_map(|r| match r.source {
                EntitySource::Inferred { via } => Some((r.id, r.site_id, via)),
                _ => None,
            })
            .collect();
        if unconfirmed.is_empty() {
            return Ok(());
        }
        let found_by = self
            .warning_service
            .unconfirmed_host_discoveries(
                &unconfirmed
                    .iter()
                    .map(|(id, site, _)| (*id, *site))
                    .collect::<Vec<_>>(),
            )
            .await?;
        for response in responses.iter_mut() {
            response.found_by_discovery_id = found_by.get(&response.id).copied();
        }
        let host_ids: Vec<Uuid> = unconfirmed.iter().map(|(id, _, _)| *id).collect();
        let own_interfaces = self
            .interface_service
            .get_all(StorableFilter::<Interface>::new_from_host_ids(&host_ids).live())
            .await?;
        let site_ids: Vec<Uuid> = unconfirmed
            .iter()
            .map(|(_, site, _)| *site)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let mut reporters: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        let mut add = |host: Uuid, reporter: Uuid| {
            let list = reporters.entry(host).or_default();
            if host != reporter && !list.contains(&reporter) {
                list.push(reporter);
            }
        };

        // Forwarding table: the ports that learned one of the host's MACs.
        let mut host_of_mac: HashMap<String, Uuid> = HashMap::new();
        for interface in &own_interfaces {
            let via = unconfirmed
                .iter()
                .find(|(id, _, _)| *id == interface.base.host_id)
                .map(|(_, _, via)| *via);
            if via == Some(InferredVia::ForwardingTable)
                && let Some(mac) = &interface.base.mac_address
            {
                host_of_mac.insert(
                    mac.value().0.to_string().to_lowercase(),
                    interface.base.host_id,
                );
            }
        }
        // A MAC can sit on an address rather than an interface (where `fdb::learned_hosts` looks
        // first), so a host reached that way is reported by the same ports.
        for response in responses.iter().filter(|r| {
            r.source
                == EntitySource::Inferred {
                    via: InferredVia::ForwardingTable,
                }
        }) {
            for ip in &response.ip_addresses {
                if let Some(mac) = &ip.base.mac_address {
                    host_of_mac.insert(mac.value().0.to_string().to_lowercase(), response.id);
                }
            }
        }
        if !host_of_mac.is_empty() {
            let macs: Vec<String> = host_of_mac.keys().cloned().collect();
            let ports = self
                .interface_service
                .get_all(
                    StorableFilter::<Interface>::new_from_site_ids(&site_ids)
                        .live()
                        .fdb_macs_contain(&macs),
                )
                .await?;
            for port in ports {
                for learned in port.base.fdb_macs().unwrap_or_default() {
                    if let Some(host) = host_of_mac.get(&learned.to_lowercase()) {
                        add(*host, port.base.host_id);
                    }
                }
            }
        }

        // Neighbours: the resolved rows that name the host, or one of its interfaces.
        if unconfirmed
            .iter()
            .any(|(_, _, via)| *via == InferredVia::Neighbour)
        {
            let host_of_interface: HashMap<Uuid, Uuid> = own_interfaces
                .iter()
                .map(|i| (i.id, i.base.host_id))
                .collect();
            let mut rows = Vec::new();
            for site_id in &site_ids {
                rows.extend(
                    self.interface_neighbor_service
                        .resolved_for_site(*site_id, None)
                        .await?,
                );
            }
            let named: Vec<(Uuid, Uuid)> = rows
                .iter()
                .filter_map(|row| {
                    let host = match row.neighbor {
                        Neighbor::Host(id) => host_ids.contains(&id).then_some(id),
                        Neighbor::Interface(id) => host_of_interface.get(&id).copied(),
                    }?;
                    Some((host, row.interface_id))
                })
                .collect();
            if !named.is_empty() {
                let reporting_ids: Vec<Uuid> = named.iter().map(|(_, i)| *i).collect();
                let reporting: HashMap<Uuid, Uuid> = self
                    .interface_service
                    .get_all(StorableFilter::<Interface>::new_from_entity_ids(
                        &reporting_ids,
                    ))
                    .await?
                    .into_iter()
                    .map(|i| (i.id, i.base.host_id))
                    .collect();
                for (host, interface_id) in named {
                    if let Some(reporter) = reporting.get(&interface_id) {
                        add(host, *reporter);
                    }
                }
            }
        }

        for response in responses.iter_mut() {
            if let Some(list) = reporters.remove(&response.id) {
                response.reported_by_host_ids = list;
            }
        }
        Ok(())
    }

    /// One warning per host on this site still waiting to be confirmed, for the reason
    /// `provisional_subnet_warnings` reports every assumed range: an unconfirmed host is a
    /// standing state, and an operator who did not act on one scan's report hears about it again.
    /// Dismissed hosts are left out.
    pub async fn unconfirmed_host_warnings(&self, site_id: Uuid) -> Result<Vec<DiscoveryWarning>> {
        let hosts = self
            .get_all(
                StorableFilter::<Host>::new_from_site_ids(&[site_id])
                    .live()
                    .not_dismissed()
                    .unconfirmed(true),
            )
            .await?;
        if hosts.is_empty() {
            return Ok(Vec::new());
        }
        let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();
        let mut mac_of_host: HashMap<Uuid, String> = HashMap::new();
        for interface in self
            .interface_service
            .get_all(StorableFilter::<Interface>::new_from_host_ids(&host_ids).live())
            .await?
        {
            if let Some(mac) = &interface.base.mac_address {
                mac_of_host
                    .entry(interface.base.host_id)
                    .or_insert_with(|| mac.value().0.to_string());
            }
        }

        Ok(hosts
            .into_iter()
            .filter_map(|host| {
                let EntitySource::Inferred { via } = host.base.source else {
                    return None;
                };
                let identifier = attribution::text_of(&host.base.chassis_id)
                    .or_else(|| mac_of_host.get(&host.id).cloned())
                    .unwrap_or_default();
                Some(DiscoveryWarning::UnconfirmedHostInferred(UnconfirmedHost {
                    host_id: Some(host.id),
                    via,
                    identifier,
                    sys_name: attribution::text_of(&host.base.sys_name),
                }))
            })
            .collect())
    }
}

/// What one inference step produced.
pub(super) struct InferenceOutcome {
    /// What this step stored or found: the hosts it minted with their ports and addresses, the
    /// ranges it created, and the existing ranges far-end addresses fell in.
    ///
    /// Ids rather than counts, because the caller hands them to the session as entities it touched.
    /// A far end minted here is a device *this scan* found, through a neighbour rather than by
    /// sweeping it, but found nonetheless, so it carries the scan's discovery FKs and is reported in
    /// its digest like anything else. Ranges go in `found_subnet_ids`, never `subnet_ids`: the scan
    /// found evidence in them without sweeping them.
    pub observed: ScannedEntityIds,
    /// Every range on the site still waiting to be confirmed, not only the ones created here.
    pub warnings: Vec<DiscoveryWarning>,
}

impl HostService {
    /// The plan limit that applies to hosts on this site, or `None` where the plan sets none.
    ///
    /// Best-effort by design: a site or organization that cannot be read yields no context, and
    /// the mint proceeds ungated rather than the whole resolution pass failing over a lookup. The
    /// alternative — treating an unreadable plan as a full one — would silently stop drawing links
    /// on a healthy fleet.
    pub(super) async fn host_limit_context(&self, site_id: Uuid) -> Option<HostLimitContext> {
        let site = self.site_service.get_by_id(&site_id).await.ok()??;
        let org_id = site.base.organization_id;
        let plan = self
            .organization_service
            .get_by_id(&org_id)
            .await
            .ok()?
            .map(|o| o.base.plan)
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);

        let limit = plan.host_limit()?;
        let org_site_ids = self
            .site_service
            .get_all(StorableFilter::<Site>::new_from_org_id(&org_id))
            .await
            .unwrap_or_default()
            .iter()
            .map(|n| n.id)
            .collect();

        Some(HostLimitContext {
            limit,
            org_id,
            org_site_ids,
            plan,
        })
    }

    /// Mint a host for each far end now that there is a subnet to place it in.
    ///
    /// The same thing `ControllerIdentity::into_host` does for a device a controller reports but
    /// the sweep never scanned, and deliberately through the same pipeline: `create_with_children`
    /// runs `select_matching_host` first, so a far end whose address or chassis id this site
    /// already holds updates that host instead of duplicating it. Minting is only ever the
    /// *fallback*, which is what keeps a device from appearing twice.
    ///
    /// `limit_ctx` is what makes the plan's host limit apply here at all. Both existing gates sit
    /// on paths this one does not take, so without it minting would quietly outrun a limit while
    /// still counting towards the number a customer is shown.
    ///
    /// Nothing here fails the pass. Link resolution is the caller's job, and a far end that cannot
    /// be minted — because the plan is full, or because two sessions raced — is one missing host,
    /// not a reason to lose every link the pass resolved.
    async fn mint_far_end_hosts(
        &self,
        site_id: Uuid,
        far_ends: &[UnplacedFarEnd],
        placements: &HashMap<String, Uuid>,
        limit_ctx: Option<&HostLimitContext>,
        scan_time: DateTime<Utc>,
    ) -> ScannedEntityIds {
        // One host per chassis id, not per address: several ports naming the same far end is one
        // device, and minting per sighting would put a row on the map for every cable. The chassis
        // id rather than the address because every far end has one — it is the hard gate in
        // `unplaced_far_end` — and because it is what `select_matching_host` later merges on.
        let mut seen: HashSet<&str> = HashSet::new();
        let mut minted = ScannedEntityIds::default();

        for far_end in far_ends {
            if !seen.insert(far_end.chassis_id.as_str()) {
                continue;
            }

            let mut host = Host::new(HostBase {
                site_id,
                // Reported by something else and never contacted. Distinct from `Discovery` so a
                // host with no ports and no services is not read as a device that is merely down,
                // and promoted the moment a scan reaches it.
                source: EntitySource::Inferred {
                    via: InferredVia::Neighbour,
                },
                // The neighbour's advertised sysName is matched against this column by the
                // resolution ladder, so recording it is what lets the *next* pass place this far
                // end without re-deriving anything.
                // Announced, not queried: a neighbour published these about itself on a link
                // anything could have spoken on, and nothing here has contacted the far end.
                sys_name: far_end
                    .sys_name
                    .clone()
                    .map(|v| Attributed::new(HostSysNameValue(v), AttributeSource::LldpChassisId)),
                chassis_id: Some(Attributed::new(
                    HostChassisIdValue(far_end.chassis_id.clone()),
                    AttributeSource::LldpChassisId,
                )),
                ..Default::default()
            });
            // Left unnamed. The sysName, chassis id and address are identifiers, each already in
            // its own column, and the display ladder titles the far end by the best of them
            // without a copy in `name` (see the placement rule in `hosts::impl::name`).
            // The scan's own timestamp, not `crate::server::shared::clock::now()`. `create.rs` derives `created_at` from
            // this via `originate_scan_timestamps`, and the digest's window closes at the session's
            // `finished_at` — so a host stamped with the wall clock at mint time lands just past the
            // end of the window that is supposed to contain it, and is never reported as added.
            // Every daemon-submitted entity already shares one timestamp for this reason.
            host.last_seen_at = scan_time;

            // An address only where a range was actually created for it. A far end that published
            // none, or whose range was refused, becomes a host with no address — legal, and what
            // `select_matching_host`'s chassis-id branch exists to match.
            let ip_addresses = match far_end
                .address
                .zip(placements.get(&far_end.chassis_id).copied())
            {
                Some((address, subnet_id)) => vec![IPAddress::new(IPAddressBase {
                    site_id,
                    host_id: Uuid::nil(), // Server assigns.
                    subnet_id,
                    ip_address: address,
                    mac_address: None,
                    name: None,
                    position: 0,
                    // Heard in the same advertisement as the far end itself.
                    ip_address_source: far_end.evidence_source,
                })],
                None => Vec::new(),
            };

            // The port the far end says it answers on. Only where it published a name for it:
            // `if_name` is what `match_existing_interface` matches on first and what the live
            // unique index covers, and a row with no name, no ifIndex and no MAC has no key at
            // all — it would match nothing on the next scan and be inserted again, forever.
            // The port the far end says it answers on, recorded whenever the advertisement
            // identifies it at all. A name and a MAC are both identities — they just key different
            // tiers of `match_existing_interface`, which is what stops the row duplicating on the
            // next scan. A far end that published neither gets no interface, because a row with no
            // name, no MAC and no ifIndex would match nothing and be inserted again forever.
            //
            // if_index, if_type, admin_status and oper_status stay `None` throughout. An
            // advertisement carries none of them, and the columns are nullable precisely so this
            // does not have to guess.
            let port_mac = far_end
                .port_mac
                .as_deref()
                .and_then(|mac| mac.parse::<MacAddress>().ok());
            let interfaces = match (&far_end.port_name, port_mac) {
                // Named: `if_name` is the key, and a later walk of this device returns the same
                // string as `ifName`, which upgrades this row in place rather than adding a second
                // one beside it. `if_descr` takes the name too, being NOT NULL and validated
                // non-empty.
                (Some(port_name), mac) => vec![Interface::new(InterfaceBase {
                    site_id,
                    host_id: Uuid::nil(), // Server assigns.
                    if_descr: Some(port_name.clone()),
                    if_name: Some(port_name.clone()),
                    mac_address: mac.map(|m| {
                        MacEvidence::new(MacEvidenceValue(m), AttributeSource::LldpChassisId)
                    }),
                    interface_source: far_end.evidence_source,
                    ..Default::default()
                })],
                // Identified by its MAC and nothing else — a `MacAddress` port id, which is a real
                // identity but not a name, so `if_name` stays empty and the MAC tier is what
                // matches it. `if_descr` carries the address because something has to, and it is
                // what the device gave us to call this port by.
                (None, Some(mac)) => vec![Interface::new(InterfaceBase {
                    site_id,
                    host_id: Uuid::nil(), // Server assigns.
                    if_descr: Some(mac.to_string()),
                    mac_address: Some(MacEvidence::new(
                        MacEvidenceValue(mac),
                        AttributeSource::LldpChassisId,
                    )),
                    interface_source: far_end.evidence_source,
                    ..Default::default()
                })],
                (None, None) => Vec::new(),
            };

            match self
                .create_with_children(
                    host,
                    ip_addresses,
                    vec![],
                    vec![],
                    interfaces,
                    vec![],
                    ConflictBehavior::Upsert,
                    AuthenticatedEntity::System,
                    limit_ctx,
                    // A neighbour sees an address and a name, never an ifTable. Claiming an
                    // authoritative empty one here would tear down interfaces a later SNMP walk of
                    // the same host collected.
                    false,
                    InterfaceDataComplete::none(),
                    &crate::server::ports::r#impl::coverage::PortCoverage::default(),
                )
                .await
            {
                // The server-assigned id, not the one this loop generated: `create_with_children`
                // matches an existing host before it creates, so what comes back may be a host this
                // site already held. Recording that id is still right — the scan did touch it.
                Ok(created) => minted.merge(ScannedEntityIds {
                    host_ids: vec![created.host.id],
                    interface_ids: created.host.interfaces.iter().map(|i| i.id).collect(),
                    ip_address_ids: created.host.ip_addresses.iter().map(|i| i.id).collect(),
                    found_subnet_ids: created.subnet_ids,
                    ..Default::default()
                }),
                Err(e) => tracing::warn!(
                    site_id = %site_id,
                    chassis_id = %far_end.chassis_id,
                    address = ?far_end.address,
                    error = %e,
                    "Could not mint a host for an unplaceable far end"
                ),
            }
        }

        minted
    }
}
