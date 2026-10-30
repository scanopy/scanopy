//! Daemon-discovery upsert path: interface linking and subnet/VLAN reconciliation.
use super::*;
use crate::server::credentials::service::FoundIntegration;

impl HostService {
    // =========================================================================
    // Discovery support (internal API)
    // =========================================================================

    /// Keep only the scanned rows on `site_id`. The list comes from the daemon, and everything
    /// downstream (the discovery FKs, the digest, guest removal, topology) treats it as what this
    /// site's scan touched, so it is narrowed once, as the report arrives.
    pub async fn retain_scanned_on_site(
        &self,
        scanned: &mut crate::server::daemons::r#impl::api::ScannedEntityIds,
        site_id: Uuid,
    ) -> Result<()> {
        // Filled by the server after this point, never by a daemon.
        scanned.found_subnet_ids.clear();

        self.retain_on_site(&mut scanned.host_ids, site_id).await?;
        self.subnet_service
            .retain_on_site(&mut scanned.subnet_ids, site_id)
            .await?;
        self.vlan_service
            .retain_on_site(&mut scanned.vlan_ids, site_id)
            .await?;
        self.ip_address_service
            .retain_on_site(&mut scanned.ip_address_ids, site_id)
            .await?;
        self.port_service
            .retain_on_site(&mut scanned.port_ids, site_id)
            .await?;
        self.interface_service
            .retain_on_site(&mut scanned.interface_ids, site_id)
            .await?;
        self.service_service
            .retain_scanned_on_site(scanned, site_id)
            .await
    }

    /// Point `last_discovery_id` (and `first_discovery_id`, where unset) at `discovery_id` on
    /// every row a finished session scanned: hosts and their IP addresses, ports, interfaces,
    /// subnets, VLANs, services and bindings.
    ///
    /// Each entity type is written independently, so one failing does not leave the others
    /// unstamped. The error names every type that failed.
    pub async fn stamp_discovery(
        &self,
        scanned: &crate::server::daemons::r#impl::api::ScannedEntityIds,
        discovery_id: Uuid,
    ) -> Result<()> {
        use crate::server::shared::services::traits::DiscoveryFkUpdater;
        use crate::server::vlans::r#impl::base::Vlan;

        let results = [
            (
                "host",
                <Self as DiscoveryFkUpdater<Host>>::update_discovery_fks(
                    self,
                    scanned,
                    discovery_id,
                )
                .await,
            ),
            (
                "ip_address",
                <IPAddressService as DiscoveryFkUpdater<IPAddress>>::update_discovery_fks(
                    &self.ip_address_service,
                    scanned,
                    discovery_id,
                )
                .await,
            ),
            (
                "port",
                <PortService as DiscoveryFkUpdater<Port>>::update_discovery_fks(
                    &self.port_service,
                    scanned,
                    discovery_id,
                )
                .await,
            ),
            (
                "interface",
                <InterfaceService as DiscoveryFkUpdater<Interface>>::update_discovery_fks(
                    &self.interface_service,
                    scanned,
                    discovery_id,
                )
                .await,
            ),
            (
                "subnet",
                <SubnetService as DiscoveryFkUpdater<Subnet>>::update_discovery_fks(
                    &self.subnet_service,
                    scanned,
                    discovery_id,
                )
                .await,
            ),
            (
                "vlan",
                <VlanService as DiscoveryFkUpdater<Vlan>>::update_discovery_fks(
                    &self.vlan_service,
                    scanned,
                    discovery_id,
                )
                .await,
            ),
            (
                "service and binding",
                self.service_service
                    .stamp_discovery(scanned, discovery_id)
                    .await,
            ),
        ];

        let failures: Vec<String> = results
            .into_iter()
            .filter_map(|(entity, result)| result.err().map(|e| format!("{entity}: {e:#}")))
            .collect();
        if failures.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(failures.join("; ")))
        }
    }

    /// Seed a daemon host's loopback subnet (`127.0.0.0/8`) and IP (`127.0.0.1`).
    ///
    /// A daemon-host socket/proxy credential is reached by the daemon at `127.0.0.1`,
    /// and the daemon's localhost probe only fires for a credential whose mapping has a
    /// `127.0.0.1` override. That override is built server-side from the host's persisted
    /// IP rows, but the mapping is snapshotted at the daemon's first work-poll — *before*
    /// the daemon self-reports its interfaces. Without a pre-existing loopback IP the first
    /// scan therefore has no override and never probes the local socket/proxy. Seeding the
    /// loopback at registration makes ordinary IP-based targeting work on scan 1, identical
    /// to every later scan.
    ///
    /// Idempotent: the loopback subnet dedupes by CIDR in `SubnetService::create`, and the
    /// IP dedupes on `(host_id, subnet_id, ip_address)`, so self-report re-reporting the
    /// same loopback later reuses these rows rather than duplicating them.
    pub async fn seed_loopback(
        &self,
        host_id: Uuid,
        site_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        let Some(loopback_subnet) = Subnet::from_discovery(
            "lo".to_string(),
            &pnet::ipnetwork::IpNetwork::V4(
                pnet::ipnetwork::Ipv4Network::new(std::net::Ipv4Addr::LOCALHOST, 8)
                    .map_err(|e| anyhow::anyhow!("Invalid loopback network: {e}"))?,
            ),
            site_id,
            // Stands in for the daemon's own report of its `lo`, which arrives on the next scan
            // with this exact range. Stamping it the same keeps that report from relabelling it.
            crate::server::shared::attribution::AttributeSource::DaemonSelfReport,
        ) else {
            return Ok(());
        };

        let created_subnet = self
            .subnet_service
            .create(loopback_subnet, authentication.clone())
            .await?;

        let loopback_ip =
            IPAddress::new(crate::server::ip_addresses::r#impl::base::IPAddressBase {
                site_id,
                host_id,
                subnet_id: created_subnet.id,
                ip_address: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                mac_address: None,
                name: Some("lo".to_string()),
                position: 0,
                // Stamped as the daemon's own report, for the reason the subnet above is.
                ip_address_source:
                    crate::server::shared::attribution::AttributeSource::DaemonSelfReport,
            });

        self.ip_address_service
            .create(loopback_ip, authentication)
            .await?;

        Ok(())
    }

    /// Create or update a host from daemon discovery data.
    /// This handles IP-address and port matching for host deduplication and upserts on conflict.
    #[allow(clippy::too_many_arguments)]
    pub async fn discover_host(
        &self,
        mut host: Host,
        mut ip_addresses: Vec<IPAddress>,
        mut ports: Vec<Port>,
        mut services: Vec<Service>,
        mut interfaces: Vec<crate::server::interfaces::r#impl::base::Interface>,
        mut subnets: Vec<Subnet>,
        interfaces_complete: bool,
        interface_data_complete: InterfaceDataComplete,
        port_coverage: &crate::server::ports::r#impl::coverage::PortCoverage,
        scan_ctx: Option<&crate::server::shared::services::scan_context::ScanContext>,
        authentication: AuthenticatedEntity,
        limit_ctx: Option<&HostLimitContext>,
    ) -> Result<DiscoveredHost> {
        // SCD2 scan-time normalization: stamp every entity in this
        // submission with the same `scan_time` so per-scan diff queries
        // (Added/Removed/Modified/Refreshed-unchanged buckets keyed on
        // session window) and freshness reads see one consistent timestamp.
        // Without this, each entity's per-call `Utc::now()` drifts across
        // the host+children tree by microseconds-to-milliseconds, blurring
        // session boundaries.
        //
        // Only refresh-style fields (last_seen_at, updated_at) are stamped
        // here — they advance on every observation regardless of new vs.
        // upsert. Origin-style fields (created_at, valid_from) are stamped
        // by the new-insert sites themselves (HostService::create,
        // SubnetService::create, ServiceService::create, and the no-match
        // branches in this function for Port / IPAddress / Interface) so
        // they only fire when a row is actually being inserted for the
        // first time. Each site reads scan_time off the entity's
        // already-refreshed `last_seen_at`.
        // `hosts.virtualization_service_id`, the hypervisor a guest runs on, names a service on a
        // *different* host submitted in a *different* call. Unlike a container service or a bridge
        // subnet, it cannot be remapped within this submission, so the daemon must send an id the
        // server already stored: the Proxmox integration creates each node first and reads its
        // Proxmox VE service id from the create response (both daemon modes return real child
        // ids). Anything else is dropped here rather than failing the host on the foreign key.
        //
        // An existing guest takes the incoming owner only when it has none (`upsert_host`), so a
        // hypervisor assigned through the UI survives every rescan.
        self.accept_discovered_virtualization_service(&mut host)
            .await;
        self.accept_discovered_virtualization_interface(&mut host)
            .await;

        // The name's rank is likewise server-authoritative at its top rung.
        // `AttributeSource::Manual` means "a person typed this into Scanopy", which nothing running
        // on a daemon can know; clamping here is what makes that unforgeable over the wire, rather
        // than trusting every integration author not to claim it.
        //
        // The ceiling is `Unspecified` rather than a named rung because a payload claiming `Manual`
        // has told us nothing we can believe about where its name came from — and `Unspecified`
        // keeps the name while letting the next real reading correct it. No daemon path constructs
        // `Manual`, so this is a guard against a future one rather than something that fires today.
        let claimed = host.base.name.source();
        if host.base.reject_manual_name_claim() {
            tracing::warn!(
                host_name = %host.base.name,
                %claimed,
                "Discovery payload claimed a name was typed into Scanopy; clamping — only the \
                 server can record that a person named a host"
            );
        }

        // `Inferred` is the server's verdict on evidence it pooled itself (LLDP far ends,
        // forwarding-table MACs), and the forwarding-table rung relaxes the minting gate. A
        // payload a daemon sent is a scan, whatever it claims.
        if matches!(host.base.source, EntitySource::Inferred { .. }) {
            host.base.source = EntitySource::Discovery;
        }

        if host.base.reject_manual_attribute_claims() {
            tracing::warn!(
                "Discovery payload claimed a host fact was typed into Scanopy; clamping — only \
                 the server can record that a person set it"
            );
        }

        // Identifiers are not names. Daemons up to v0.17.14 repeat the hostname as the name and
        // send the address as an `OwnAddress` name; the same payload carries each identifier in
        // its own column, so the copy is dropped (see the placement rule in `hosts::impl::name`).
        if host.base.drop_identifier_copy() {
            tracing::debug!(
                %claimed,
                "Discovery payload repeated an identifier as the host name; dropped the copy"
            );
        }

        // The same server-only rung on the sources that record how each row was found.
        if reject_manual_collection_claims(&mut ip_addresses, &mut ports, &mut interfaces) {
            tracing::warn!(
                "Discovery payload claimed a row was entered in Scanopy; recording it as \
                 unattributed"
            );
        }

        // An OS named in the SNMP system strings, matched here so every daemon version's SNMP
        // hosts get one. Ranks below anything read off the host, which the payload may also carry.
        host.base.match_os_from_system_strings();

        if let Some(ctx) = scan_ctx {
            use crate::server::shared::storage::snapshot::DiscoveryTracked;
            host.refresh_scan_timestamps(ctx.scan_time);
            for ip in ip_addresses.iter_mut() {
                ip.refresh_scan_timestamps(ctx.scan_time);
            }
            for p in ports.iter_mut() {
                p.refresh_scan_timestamps(ctx.scan_time);
            }
            for s in services.iter_mut() {
                s.refresh_scan_timestamps(ctx.scan_time);
            }
            for i in interfaces.iter_mut() {
                i.refresh_scan_timestamps(ctx.scan_time);
            }
            for s in subnets.iter_mut() {
                s.refresh_scan_timestamps(ctx.scan_time);
            }
        }

        // Capture the subnets the matched host touched before the upsert. If the
        // host migrates subnets (or an IP address stops reporting), we need to
        // revisit the old subnet during reconciliation so its stale VLAN links
        // can drop. Post-upsert `get_for_host` alone would miss subnets that
        // disappeared entirely from the host.
        let site_id = host.base.site_id;
        let matched = self
            .find_matching_host_by_ip_addresses(
                &site_id,
                &ip_addresses,
                &interfaces,
                host.base.chassis_id.as_ref().map(|c| c.value().0.as_str()),
            )
            .await?;
        // A report that reached a container at its bridge address from outside the runtime
        // describes the container, not the runtime host it matched (see `container_reports`).
        let mut interfaces_complete = interfaces_complete;
        if let Some(m) = &matched
            && let Some(reached) = self
                .reached_container(m, host.base.virtualization_metadata.as_ref(), &ip_addresses)
                .await?
        {
            reached.confine(
                &m.host,
                &mut host,
                &mut ip_addresses,
                &mut services,
                &mut interfaces,
                &mut subnets,
                &mut interfaces_complete,
            );
        }
        let mut previous_subnets: HashSet<Uuid> = matched
            .as_ref()
            .map(|m| m.ip_addresses.iter().map(|i| i.base.subnet_id).collect())
            .unwrap_or_default();
        // A switch's ports are evidence for the subnets of the hosts they learned. Those it
        // learned before this scan are revisited too, so a link drops once no port learns it.
        if !interfaces.is_empty()
            && let Some(matched) = &matched
        {
            let before = self
                .interface_service
                .get_for_host(&matched.host.id)
                .await?;
            previous_subnets.extend(
                self.subnets_learned_by(site_id, &subnet_vlans::learned_macs(&before))
                    .await?,
            );
        }

        let discovered = self
            .create_with_children(
                host,
                ip_addresses,
                ports,
                services,
                interfaces.clone(),
                subnets,
                ConflictBehavior::Upsert,
                authentication.clone(),
                limit_ctx,
                interfaces_complete,
                interface_data_complete,
                port_coverage,
            )
            .await?;

        // Link Interfaces to IPAddresses via MAC address matching (if any were created)
        if !interfaces.is_empty()
            && let Err(e) = self
                .link_interfaces_to_ip_addresses(&discovered.host.id, authentication)
                .await
        {
            tracing::warn!(error = %e, "Failed to link Interfaces to IPAddresses");
        }

        // Reconcile subnet↔VLAN junction records across previous ∪ current subnets.
        // Aggregates native_vlan_id observations from all hosts so stale links drop
        // when nobody reports them anymore.
        if !interfaces.is_empty()
            && let Err(e) = self
                .reconcile_subnet_vlans_for_host(&discovered.host.id, &previous_subnets)
                .await
        {
            tracing::warn!(error = %e, "Failed to reconcile subnet_vlans");
        }

        Ok(discovered)
    }

    /// Link Interface records (SNMP ifTable entries) to IPAddress records for a host by matching MAC addresses.
    ///
    /// For each Interface with a MAC address, finds an IPAddress on the same host with
    /// the same MAC address and sets `interface.ip_address_id = ip_address.id`.
    /// This enables PhysicalLink topology edges to have source/target Interface IDs.
    ///
    /// The decision itself is [`plan_interface_ip_links`]; this only loads and persists.
    async fn link_interfaces_to_ip_addresses(
        &self,
        host_id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        let ip_addresses = self.ip_address_service.get_for_host(host_id).await?;
        let interfaces = self.interface_service.get_for_host(host_id).await?;

        let planned: HashMap<Uuid, Uuid> = plan_interface_ip_links(&interfaces, &ip_addresses)
            .into_iter()
            .collect();

        for mut interface in interfaces {
            let Some(&ip_address_id) = planned.get(&interface.id) else {
                continue;
            };

            interface.base.ip_address_id = Some(ip_address_id);
            if let Err(e) = self
                .interface_service
                .update(
                    &mut interface,
                    authentication.clone(),
                    UpdateReason::InterfaceLinkedByMac,
                )
                .await
            {
                tracing::warn!(
                    interface_id = %interface.id,
                    error = %e,
                    "Failed to link Interface to IPAddress"
                );
            }
        }

        Ok(())
    }

    /// Reconcile subnet↔VLAN junction records after a host's discovery.
    ///
    /// For each subnet in `previous_subnets ∪ current_subnets`, aggregates two kinds of evidence
    /// across ALL hosts and replaces the junction set via `save_for_subnet`:
    ///
    /// - `native_vlan_id` on the Interfaces linked to an address on the subnet;
    /// - the native VLAN of every access port that learned a MAC whose only addresses are on the
    ///   subnet ([`subnet_vlans::access_port_vlans`]).
    ///
    /// "Current" subnets are the host's own, post-upsert, plus those of the hosts its ports now
    /// learn. `previous_subnets` carries the same two sets from before the upsert, so a host that
    /// migrated off a subnet, or a port that stopped learning a host, still triggers cleanup.
    ///
    /// Inside each reconciled subnet, aggregating across all hosts means host A's
    /// rescan preserves host B's contribution — stale (subnet, vlan) pairs drop
    /// only when NO host reports them anymore.
    ///
    /// NOTE: the reads of fresh data and the `save_for_subnet` calls are not
    /// transactionally linked. Two hosts on the same subnet discovering
    /// concurrently can briefly produce a wrong VLAN set; the next scan of any
    /// host on that subnet corrects it. Eventually consistent.
    async fn reconcile_subnet_vlans_for_host(
        &self,
        host_id: &Uuid,
        previous_subnets: &HashSet<Uuid>,
    ) -> Result<()> {
        let own_ip_addresses = self.ip_address_service.get_for_host(host_id).await?;
        let mut current: HashSet<Uuid> =
            own_ip_addresses.iter().map(|i| i.base.subnet_id).collect();
        let interfaces = self.interface_service.get_for_host(host_id).await?;
        if let Some(site_id) = interfaces.first().map(|i| i.base.site_id) {
            current.extend(
                self.subnets_learned_by(site_id, &subnet_vlans::learned_macs(&interfaces))
                    .await?,
            );
        }

        let to_reconcile: HashSet<Uuid> = previous_subnets.union(&current).copied().collect();

        let mut reconciled = 0usize;
        for subnet_id in to_reconcile {
            // All IPAddresses on this subnet across all hosts
            let subnet_ip_addresses = self.ip_address_service.get_for_subnet(&subnet_id).await?;
            if subnet_ip_addresses.is_empty() {
                // Subnet has no ip_addresses at all; drop any leftover links.
                self.vlan_service.set_subnet_vlans(&subnet_id, &[]).await?;
                reconciled += 1;
                continue;
            }

            let ip_address_ids: Vec<Uuid> = subnet_ip_addresses.iter().map(|i| i.id).collect();

            // All Interface rows linked to those IPAddresses across all hosts
            let linked_interfaces = self
                .interface_service
                .get_by_ip_address_ids(&ip_address_ids)
                .await?;

            let mut fresh_vlan_ids: Vec<Uuid> = linked_interfaces
                .iter()
                .filter_map(|iface| iface.base.native_vlan_id)
                .collect();
            fresh_vlan_ids.extend(
                self.access_port_vlans_for_subnet(subnet_id, &subnet_ip_addresses)
                    .await?,
            );
            fresh_vlan_ids.sort();
            fresh_vlan_ids.dedup();

            self.vlan_service
                .set_subnet_vlans(&subnet_id, &fresh_vlan_ids)
                .await?;
            reconciled += 1;
        }

        if reconciled > 0 {
            tracing::debug!(
                host_id = %host_id,
                subnets_reconciled = reconciled,
                "Reconciled subnet_vlans links from Interface and forwarding-database data"
            );
        }

        Ok(())
    }

    /// The integrations whose services a finished run touched, on hosts no credential of that
    /// integration covers. See `credentials/service/coverage.rs` for the rule. `started_at` is
    /// when the run began, so the site's credentials that existed then count as tried.
    pub async fn uncovered_integrations_for_run(
        &self,
        scanned: &crate::server::daemons::r#impl::api::ScannedEntityIds,
        integration_targets: &[crate::server::credentials::r#impl::mapping::IntegrationTarget],
        site_id: Uuid,
        daemon_host_id: Option<Uuid>,
        started_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<FoundIntegration>> {
        if scanned.service_ids.is_empty() {
            return Ok(Vec::new());
        }
        let found = self
            .service_service
            .get_all(StorableFilter::<Service>::new_from_entity_ids(
                &scanned.service_ids,
            ))
            .await?
            .iter()
            .filter_map(FoundIntegration::from_service)
            .collect();
        self.credential_service
            .uncovered_integrations(
                found,
                integration_targets,
                daemon_host_id,
                &HashMap::from([(site_id, started_at)]),
            )
            .await
    }
}

/// Lower every `Manual` source a discovery payload claims on its rows to `Unspecified`, as
/// [`HostBase::reject_manual_name_claim`] does for the name. Returns whether any changed.
fn reject_manual_collection_claims(
    ip_addresses: &mut [IPAddress],
    ports: &mut [Port],
    interfaces: &mut [Interface],
) -> bool {
    use crate::server::shared::attribution::AttributeSource;

    let mut rejected = false;
    for ip_address in ip_addresses {
        rejected |= ip_address.base.ip_address_source.reject_manual_claim();
    }
    for port in ports {
        rejected |= port.base.port_source.reject_manual_claim();
    }
    for interface in interfaces {
        let base = &mut interface.base;
        rejected |= base.interface_source.reject_manual_claim();
        rejected |= base.vlan_membership_source.reject_manual_claim();
        if let Some(fdb) = base.fdb_macs.take() {
            base.fdb_macs = Some(if fdb.source() == AttributeSource::Manual {
                rejected = true;
                fdb.clamped_to(AttributeSource::Unspecified)
            } else {
                fdb
            });
        }
        for candidate in &mut base.neighbor_candidates {
            rejected |= candidate.evidence_source.reject_manual_claim();
        }
    }
    rejected
}

/// Which IPAddress row each of a host's unlinked interfaces should point at.
///
/// Returns `(interface_id, ip_address_id)` pairs. Pure, so the rules below can be exercised
/// without a database — the same split `match_existing_interface` uses for interface dedup.
///
/// A MAC only links an interface when exactly one of the host's interfaces carries it. Switches
/// that report the chassis base MAC as `ifPhysAddress` on every port (GH #668) otherwise match all
/// of them against the management IP, giving every port on a 48-port switch the same
/// `ip_address_id` — which is what `resolve_ip_address_for_interface` anchors topology edges on.
/// Losing the link costs such a device nothing: that fallback already resolves a host with a single
/// IP without it.
///
/// The reverse multiplicity is deliberately *not* guarded. Several IPs on one NIC share a MAC, and
/// that is an ordinary configuration where every candidate is a correct answer rather than an
/// ambiguity — refusing to link would strip the anchor from hosts that are not affected by
/// anything here.
fn plan_interface_ip_links(
    interfaces: &[Interface],
    ip_addresses: &[IPAddress],
) -> Vec<(Uuid, Uuid)> {
    use crate::server::interfaces::r#impl::base::if_type;

    let loopback_ip_id = ip_addresses
        .iter()
        .find(|ip| ip.base.ip_address.is_loopback())
        .map(|ip| ip.id);

    let ip_by_mac: HashMap<MacAddress, Uuid> = ip_addresses
        .iter()
        .filter_map(|ip| mac_of(&ip.base.mac_address).map(|mac| (mac, ip.id)))
        .collect();

    let mut interfaces_per_mac: HashMap<MacAddress, usize> = HashMap::new();
    for mac in interfaces
        .iter()
        .filter_map(|i| mac_of(&i.base.mac_address))
    {
        *interfaces_per_mac.entry(mac).or_default() += 1;
    }

    interfaces
        .iter()
        .filter(|i| i.base.ip_address_id.is_none())
        .filter_map(|i| {
            let target = if i.base.if_type == Some(if_type::SOFTWARE_LOOPBACK) {
                loopback_ip_id
            } else {
                mac_of(&i.base.mac_address)
                    .filter(|mac| interfaces_per_mac.get(mac) == Some(&1))
                    .and_then(|mac| ip_by_mac.get(&mac).copied())
            }?;
            Some((i.id, target))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
    use crate::server::services::r#impl::patterns::ClientProbe;
    use crate::server::shared::attribution::AttributeSource;

    /// What a credentialed SNMP walk claims for a MAC it read off the device itself. Named once so
    /// a fixture cannot assert a provenance no real scan produces.
    const SNMP_MAC: AttributeSource = AttributeSource::Probe(ClientProbe::Snmp);

    use crate::server::interfaces::r#impl::base::InterfaceBase;
    use crate::server::ip_addresses::r#impl::base::IPAddressBase;

    fn iface(if_index: i32, mac: &str) -> Interface {
        Interface::new(InterfaceBase {
            if_index: Some(if_index),
            if_descr: Some(format!("Slot0/{if_index}")),
            mac_address: Some(MacEvidence::new(
                MacEvidenceValue(mac.parse::<MacAddress>().unwrap()),
                SNMP_MAC,
            )),
            ..Default::default()
        })
    }

    fn management_ip(ip: &str, mac: &str) -> IPAddress {
        IPAddress::new(IPAddressBase {
            ip_address: ip.parse().unwrap(),
            mac_address: Some(MacEvidence::new(
                MacEvidenceValue(mac.parse::<MacAddress>().unwrap()),
                SNMP_MAC,
            )),
            ..Default::default()
        })
    }

    /// GH #668: a switch reporting its chassis base MAC on every port matched all of them against
    /// the one IP it has, giving 48 ports the same `ip_address_id` — the FK topology anchors edges
    /// on. Its management port is not identifiable from the MAC alone, so none of them link, and
    /// `resolve_ip_address_for_interface`'s single-IP fallback covers the device anyway.
    #[test]
    fn a_mac_every_port_reports_links_none_of_them() {
        let chassis_mac = "00:ad:24:af:4e:00";
        let interfaces: Vec<Interface> = (1..=3).map(|n| iface(n, chassis_mac)).collect();
        let ips = vec![management_ip("10.0.0.2", chassis_mac)];

        assert!(plan_interface_ip_links(&interfaces, &ips).is_empty());
    }

    /// The guard is on repetition, not on MACs — a device whose ports each have their own address
    /// still links the one that carries the IP, and only that one.
    #[test]
    fn a_port_mac_unique_within_the_device_still_links() {
        let interfaces = vec![iface(1, "00:ad:24:af:4e:01"), iface(2, "00:ad:24:af:4e:02")];
        let ip = management_ip("10.0.0.2", "00:ad:24:af:4e:01");

        assert_eq!(
            plan_interface_ip_links(&interfaces, std::slice::from_ref(&ip)),
            vec![(interfaces[0].id, ip.id)]
        );
    }

    /// Several IPs on one NIC share its MAC. That is an ordinary configuration in which every
    /// candidate is a correct answer, not an ambiguity — the interface must still get an anchor.
    #[test]
    fn several_ips_on_one_nic_still_link_that_nic() {
        let mac = "00:ad:24:af:4e:01";
        let interfaces = vec![iface(1, mac)];
        let ips = vec![
            management_ip("10.0.0.2", mac),
            management_ip("10.0.0.3", mac),
        ];

        let planned = plan_interface_ip_links(&interfaces, &ips);
        assert_eq!(planned.len(), 1);
        assert!(ips.iter().any(|ip| planned[0] == (interfaces[0].id, ip.id)));
    }

    /// Only the server records that a person entered a row, so a daemon payload claiming it on any
    /// row source is lowered to unattributed, and an attributed source passes untouched.
    #[test]
    fn a_daemons_manual_claim_on_a_row_source_is_demoted() {
        use crate::server::interface_neighbors::r#impl::base::InterfaceNeighborEvidence;
        use crate::server::interfaces::r#impl::base::{FdbMacs, FdbMacsValue};
        use crate::server::ports::r#impl::base::PortType;

        let manual = AttributeSource::Manual;
        let mut ips = vec![IPAddress::new(IPAddressBase {
            ip_address_source: manual,
            ..Default::default()
        })];
        let mut ports = vec![
            Port::new_hostless(PortType::Ssh).with_source(manual),
            Port::new_hostless(PortType::Http).with_source(AttributeSource::PortScan),
        ];
        let mut interfaces = vec![Interface::new(InterfaceBase {
            interface_source: manual,
            vlan_membership_source: manual,
            fdb_macs: Some(FdbMacs::new(
                FdbMacsValue(vec!["00:1a:2b:00:10:00".to_string()]),
                manual,
            )),
            neighbor_candidates: vec![InterfaceNeighborEvidence {
                evidence_source: manual,
                ..Default::default()
            }],
            ..Default::default()
        })];

        assert!(reject_manual_collection_claims(
            &mut ips,
            &mut ports,
            &mut interfaces
        ));

        let unspecified = AttributeSource::Unspecified;
        assert_eq!(ips[0].base.ip_address_source, unspecified);
        assert_eq!(ports[0].base.port_source, unspecified);
        assert_eq!(ports[1].base.port_source, AttributeSource::PortScan);
        let base = &interfaces[0].base;
        assert_eq!(base.interface_source, unspecified);
        assert_eq!(base.vlan_membership_source, unspecified);
        assert_eq!(
            base.fdb_macs.as_ref().map(|f| f.source()),
            Some(unspecified)
        );
        assert_eq!(
            base.fdb_macs(),
            Some(["00:1a:2b:00:10:00".to_string()].as_slice())
        );
        assert_eq!(base.neighbor_candidates[0].evidence_source, unspecified);

        assert!(!reject_manual_collection_claims(
            &mut ips,
            &mut ports,
            &mut interfaces
        ));
    }
}
