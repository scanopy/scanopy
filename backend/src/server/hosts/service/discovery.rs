//! Daemon-discovery upsert path: interface linking and subnet/VLAN reconciliation.
use super::*;

impl HostService {
    // =========================================================================
    // Discovery support (internal API)
    // =========================================================================

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
        network_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        let Some(loopback_subnet) = Subnet::from_discovery(
            "lo".to_string(),
            &pnet::ipnetwork::IpNetwork::V4(
                pnet::ipnetwork::Ipv4Network::new(std::net::Ipv4Addr::LOCALHOST, 8)
                    .map_err(|e| anyhow::anyhow!("Invalid loopback network: {e}"))?,
            ),
            network_id,
        ) else {
            return Ok(());
        };

        let created_subnet = self
            .subnet_service
            .create(loopback_subnet, authentication.clone())
            .await?;

        let loopback_ip =
            IPAddress::new(crate::server::ip_addresses::r#impl::base::IPAddressBase {
                network_id,
                host_id,
                subnet_id: created_subnet.id,
                ip_address: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                mac_address: None,
                name: Some("lo".to_string()),
                position: 0,
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
        scan_ctx: Option<&crate::server::shared::services::scan_context::ScanContext>,
        authentication: AuthenticatedEntity,
        limit_ctx: Option<&HostLimitContext>,
    ) -> Result<HostResponse> {
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
        // `hosts.virtualization_service_id` — the hypervisor a VM runs on — is
        // server-authoritative. The daemon does not own it and must not send one.
        //
        // Unlike a container service or a bridge subnet, which name a service on the *same*
        // host in the *same* submission, a VM guest names a hypervisor service on a *different*
        // host submitted in a *different* call (one host per `discover_host`). Nothing in this
        // function could resolve such a reference, and `CreatedEntitiesPayload` returns
        // pending→real mappings for hosts and subnets only — never services — so a daemon can
        // never learn the real id to send in the first place.
        //
        // Today the field is only ever set through the UI, where the id is already a real one
        // (`HostData::with_virtualization` has no callers). Stripping it here keeps the FK
        // satisfiable and makes a future integration that wires a raw UUID loud in development
        // rather than a foreign-key failure in production. When the Proxmox integration lands it
        // should send a resolvable reference — hypervisor address plus service definition — and
        // have the server resolve it, or carry the pending→real service mapping across the
        // session; see the plan in TASK.md.
        //
        // Note this only clears what the *payload* carried: `upsert_host` never touches this
        // column, so a value set through the UI survives every rescan.
        if host.base.virtualization_service_id.take().is_some() {
            tracing::warn!(
                host_name = %host.base.name,
                "Discovery payload carried a host virtualization_service_id; ignoring it — the \
                 column is server-authoritative and a daemon-minted service id cannot be resolved \
                 across submissions"
            );
        }

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

        // Identifiers are not names. Daemons up to v0.17.14 repeat the hostname as the name and
        // send the address as an `OwnAddress` name; the same payload carries each identifier in
        // its own column, so the copy is dropped (see the placement rule in `hosts::impl::name`).
        if host.base.drop_identifier_copy() {
            tracing::debug!(
                %claimed,
                "Discovery payload repeated an identifier as the host name; dropped the copy"
            );
        }

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
        let previous_subnets: HashSet<Uuid> = self
            .find_matching_host_by_ip_addresses(
                &host.base.network_id,
                &ip_addresses,
                &interfaces,
                host.base.chassis_id.as_ref().map(|c| c.value().0.as_str()),
            )
            .await?
            .map(|(_, existing_ips)| existing_ips.iter().map(|i| i.base.subnet_id).collect())
            .unwrap_or_default();

        let host_response = self
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
            )
            .await?;

        // Link Interfaces to IPAddresses via MAC address matching (if any were created)
        if !interfaces.is_empty()
            && let Err(e) = self
                .link_interfaces_to_ip_addresses(&host_response.id, authentication)
                .await
        {
            tracing::warn!(error = %e, "Failed to link Interfaces to IPAddresses");
        }

        // Reconcile subnet↔VLAN junction records across previous ∪ current subnets.
        // Aggregates native_vlan_id observations from all hosts so stale links drop
        // when nobody reports them anymore.
        if !interfaces.is_empty()
            && let Err(e) = self
                .reconcile_subnet_vlans_for_host(&host_response.id, &previous_subnets)
                .await
        {
            tracing::warn!(error = %e, "Failed to reconcile subnet_vlans");
        }

        Ok(host_response)
    }

    /// Link Interface records (SNMP if-entries) to IPAddress records for a host by matching MAC addresses.
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

        let mut linked_count = 0;
        for mut interface in interfaces {
            let Some(&ip_address_id) = planned.get(&interface.id) else {
                continue;
            };

            interface.base.ip_address_id = Some(ip_address_id);
            if let Err(e) = self
                .interface_service
                .update(&mut interface, authentication.clone())
                .await
            {
                tracing::warn!(
                    interface_id = %interface.id,
                    error = %e,
                    "Failed to link Interface to IPAddress"
                );
            } else {
                linked_count += 1;
            }
        }

        if linked_count > 0 {
            tracing::debug!(
                host_id = %host_id,
                linked = linked_count,
                "Linked Interfaces to IPAddresses via MAC address and loopback type"
            );
        }

        Ok(())
    }

    /// Reconcile subnet↔VLAN junction records after a host's discovery.
    ///
    /// For each subnet in `previous_subnets ∪ current_subnets`, aggregates
    /// `native_vlan_id` observations across ALL hosts' Interfaces on that subnet
    /// and replaces the junction set via `save_for_subnet`. The union ensures
    /// that a host which migrated off a subnet still triggers cleanup for that
    /// old subnet; post-upsert `get_for_host` captures "current" subnets.
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
        let current: HashSet<Uuid> = self
            .ip_address_service
            .get_for_host(host_id)
            .await?
            .iter()
            .map(|i| i.base.subnet_id)
            .collect();

        let to_reconcile: HashSet<Uuid> = previous_subnets.union(&current).copied().collect();

        let mut reconciled = 0usize;
        for subnet_id in to_reconcile {
            // All IPAddresses on this subnet across all hosts
            let subnet_ip_addresses = self.ip_address_service.get_for_subnet(&subnet_id).await?;
            if subnet_ip_addresses.is_empty() {
                // Subnet has no ip_addresses at all; drop any leftover links.
                self.vlan_service
                    .subnet_vlan_storage
                    .save_for_subnet(&subnet_id, &[])
                    .await?;
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
            fresh_vlan_ids.sort();
            fresh_vlan_ids.dedup();

            self.vlan_service
                .subnet_vlan_storage
                .save_for_subnet(&subnet_id, &fresh_vlan_ids)
                .await?;
            reconciled += 1;
        }

        if reconciled > 0 {
            tracing::debug!(
                host_id = %host_id,
                subnets_reconciled = reconciled,
                "Reconciled subnet_vlans links from Interface data"
            );
        }

        Ok(())
    }
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
        let mut base = InterfaceBase::default();
        base.if_index = Some(if_index);
        base.if_descr = Some(format!("Slot0/{if_index}"));
        base.mac_address = Some(MacEvidence::new(
            MacEvidenceValue(mac.parse::<MacAddress>().unwrap()),
            SNMP_MAC,
        ));
        Interface::new(base)
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
}
