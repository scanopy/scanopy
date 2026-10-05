//! Merging records a payload proves are one device.
//!
//! Dedup resolves a payload to one host. This decides which *other* existing hosts the same
//! payload proves are that device, so discovery can merge them into it.
use super::*;

/// Other live hosts this payload proves are the same device as `matched`, each with the
/// addresses that prove it. The caller merges them into `matched`.
///
/// Dedup resolves a payload to one host, so a device whose addresses were first found apart (a
/// host with two NICs swept address by address, a switch's SVIs) stays split even after one
/// payload names them all. A host is merged only when the payload **accounts for** it: it has at
/// least one identity row, and every identity row has the IP and subnet of an incoming address and
/// carries one of the payload's MACs. A host holding any address the payload does not claim is a
/// different device that happens to share one (an HA peer holding a floating VIP), or a stale
/// record of an address's previous holder, and is left alone.
///
/// - **Payload MACs** are only those whose evidence binds the claim to the device
///   ([`binds_claim_to_subject`](crate::server::shared::attribution::AttributeMethod::binds_claim_to_subject)): an ARP reply, SNMP, a daemon's self-report, a
///   hypervisor's config. A controller or a neighbour's say-so proves nothing about identity.
/// - **Identity rows** leave out loopback, virtual-router MACs (shared by HA peers) and addresses
///   on `internal_subnet_ids` (container bridges, which only exist inside one host).
/// - Nothing merges unless `matched` is itself accounted for. A payload that landed on a host
///   through a shared VIP must not then pull its own record into that host.
/// - Never a host whose MACs are not its own (an ipvlan container), and never one in
///   `daemon_host_ids`: a host carrying a daemon is the record that daemon reports as, and is only
///   ever merged into by hand.
pub(crate) fn hosts_proven_same_device(
    incoming: &[IPAddress],
    matched: Uuid,
    candidates: &[HostCandidate],
    internal_subnet_ids: &HashSet<Uuid>,
    daemon_host_ids: &HashSet<Uuid>,
) -> Vec<SameDevice> {
    let payload_macs: HashSet<MacAddress> = incoming
        .iter()
        .filter_map(|i| i.base.mac_address.as_ref())
        .filter(|e| e.source().method().binds_claim_to_subject())
        .map(|e| e.value().0)
        .filter(|m| !is_virtual_router_mac(m))
        .collect();
    if payload_macs.is_empty() {
        return Vec::new();
    }

    // The addresses a host's identity rows hold, when the payload accounts for every one of them.
    let accounted_for = |candidate: &HostCandidate| -> Option<Vec<IpAddr>> {
        let identity_rows: Vec<&IPAddress> = candidate
            .ip_addresses
            .iter()
            .filter(|row| {
                !should_skip_for_matching(row) && !internal_subnet_ids.contains(&row.base.subnet_id)
            })
            .collect();
        if identity_rows.is_empty() {
            return None;
        }
        let all_claimed = identity_rows.iter().all(|row| {
            incoming.iter().any(|i| ip_addresses_share_address(i, row))
                && mac_of(&row.base.mac_address).is_some_and(|m| payload_macs.contains(&m))
        });
        all_claimed.then(|| identity_rows.iter().map(|r| r.base.ip_address).collect())
    };

    let Some(matched_candidate) = candidates.iter().find(|c| c.id == matched) else {
        return Vec::new();
    };
    if accounted_for(matched_candidate).is_none() {
        return Vec::new();
    }

    candidates
        .iter()
        .filter(|c| c.id != matched && c.macs_identify_host() && !daemon_host_ids.contains(&c.id))
        .filter_map(|c| {
            accounted_for(c).map(|addresses| SameDevice {
                host_id: c.id,
                addresses,
            })
        })
        .collect()
}

/// A host [`hosts_proven_same_device`] found to be the same device, and the addresses that showed
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SameDevice {
    pub host_id: Uuid,
    pub addresses: Vec<IpAddr>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::virtualization::{
        ContainerHostVirtualization, ContainerNetworkType,
    };
    use crate::server::ip_addresses::r#impl::base::{IPAddressBase, MacEvidence, MacEvidenceValue};
    use crate::server::services::r#impl::patterns::ClientProbe;

    /// VM 103's two NICs, as the lab reports them.
    fn ens18() -> MacAddress {
        "02:1c:65:32:e5:dd".parse().unwrap()
    }
    fn ens19() -> MacAddress {
        "bc:24:11:9b:24:86".parse().unwrap()
    }
    fn mac(s: &str) -> MacAddress {
        s.parse().unwrap()
    }
    /// The shared VRRP MAC for VRID 10.
    fn vrrp() -> MacAddress {
        mac("00:00:5e:00:01:0a")
    }

    fn row(ip: &str, subnet_id: Uuid, mac: MacAddress, source: AttributeSource) -> IPAddress {
        IPAddress::new(IPAddressBase {
            ip_address: ip.parse().unwrap(),
            subnet_id,
            mac_address: Some(MacEvidence::new(MacEvidenceValue(mac), source)),
            ..Default::default()
        })
    }
    fn arp(ip: &str, subnet_id: Uuid, mac: MacAddress) -> IPAddress {
        row(ip, subnet_id, mac, AttributeSource::ArpReply)
    }
    fn config(ip: &str, subnet_id: Uuid, mac: MacAddress) -> IPAddress {
        row(ip, subnet_id, mac, AttributeSource::HypervisorConfig)
    }
    fn snmp(ip: &str, subnet_id: Uuid, mac: MacAddress) -> IPAddress {
        row(
            ip,
            subnet_id,
            mac,
            AttributeSource::Probe(ClientProbe::Snmp),
        )
    }

    fn host(id: Uuid, rows: Vec<IPAddress>) -> HostCandidate {
        HostCandidate {
            id,
            chassis_id: None,
            ip_addresses: rows,
            virtualization: None,
            virtualization_interface_id: None,
        }
    }

    fn proven(
        incoming: &[IPAddress],
        matched: Uuid,
        candidates: &[HostCandidate],
        internal: &[Uuid],
    ) -> Vec<Uuid> {
        hosts_proven_same_device(
            incoming,
            matched,
            candidates,
            &internal.iter().copied().collect(),
            &HashSet::new(),
        )
        .into_iter()
        .map(|s| s.host_id)
        .collect()
    }

    /// The lab's split: the sweep found .63 (H1) and the Docker runtime .126 (H2), which also
    /// holds its container bridge addresses. VM 103's config names both NICs, so H2 is VM 103.
    /// ARP flux put ens19's MAC on .126; the rule takes any of the payload's MACs, so it still
    /// proves the match.
    #[test]
    fn a_vm_swept_address_by_address_is_proven_one_device_through_arp_flux() {
        let (lan, bridge) = (Uuid::new_v4(), Uuid::new_v4());
        let (h1, h2) = (Uuid::new_v4(), Uuid::new_v4());
        let candidates = vec![
            host(h1, vec![arp("192.168.4.63", lan, ens19())]),
            host(
                h2,
                vec![
                    arp("192.168.4.126", lan, ens19()),
                    row(
                        "172.17.0.2",
                        bridge,
                        mac("f6:af:f2:ff:a5:ff"),
                        AttributeSource::Probe(ClientProbe::Docker),
                    ),
                ],
            ),
        ];
        let payload = [
            config("192.168.4.126", lan, ens18()),
            config("192.168.4.63", lan, ens19()),
        ];

        let found = hosts_proven_same_device(
            &payload,
            h1,
            &candidates,
            &HashSet::from([bridge]),
            &HashSet::new(),
        );
        assert_eq!(
            found,
            vec![SameDevice {
                host_id: h2,
                addresses: vec!["192.168.4.126".parse().unwrap()],
            }]
        );
    }

    /// Without flux the sweep reads .126 behind ens18's own MAC; same result.
    #[test]
    fn a_vm_swept_address_by_address_is_proven_one_device_without_flux() {
        let lan = Uuid::new_v4();
        let (h1, h2) = (Uuid::new_v4(), Uuid::new_v4());
        let candidates = vec![
            host(h1, vec![arp("192.168.4.63", lan, ens19())]),
            host(h2, vec![arp("192.168.4.126", lan, ens18())]),
        ];
        let payload = [
            config("192.168.4.126", lan, ens18()),
            config("192.168.4.63", lan, ens19()),
        ];
        assert_eq!(proven(&payload, h1, &candidates, &[]), vec![h2]);
    }

    /// keepalived with no virtual MAC: each node answers for the VIP with its own MAC. Node B's
    /// payload lands on node A through the VIP (the oldest host holding it), but A also holds .1,
    /// which B's payload does not claim, so A is not B and nothing merges.
    #[test]
    fn an_ha_peer_reached_through_a_floating_vip_is_never_merged() {
        let lan = Uuid::new_v4();
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let (mac_a, mac_b) = (mac("aa:00:00:00:00:01"), mac("aa:00:00:00:00:02"));
        let candidates = vec![
            host(
                a,
                vec![snmp("10.0.0.1", lan, mac_a), snmp("10.0.0.10", lan, mac_a)],
            ),
            host(
                b,
                vec![snmp("10.0.0.2", lan, mac_b), snmp("10.0.0.10", lan, mac_b)],
            ),
        ];
        let payload_from_b = [snmp("10.0.0.2", lan, mac_b), snmp("10.0.0.10", lan, mac_b)];
        assert!(proven(&payload_from_b, a, &candidates, &[]).is_empty());
    }

    /// A record holding nothing but a VIP, seen behind the active node's MAC, is that node's
    /// address and joins it.
    #[test]
    fn a_vip_only_record_joins_the_node_whose_payload_accounts_for_it() {
        let lan = Uuid::new_v4();
        let (node, vip) = (Uuid::new_v4(), Uuid::new_v4());
        let mac_a = mac("aa:00:00:00:00:01");
        let candidates = vec![
            host(node, vec![snmp("10.0.0.1", lan, mac_a)]),
            host(vip, vec![arp("10.0.0.10", lan, mac_a)]),
        ];
        let payload = [snmp("10.0.0.1", lan, mac_a), snmp("10.0.0.10", lan, mac_a)];
        assert_eq!(proven(&payload, node, &candidates, &[]), vec![vip]);
    }

    /// CARP/VRRP VIPs carry a virtual-router MAC, which is never identity: peer A's own address
    /// stays outside B's payload and nothing merges.
    #[test]
    fn a_virtual_router_vip_never_proves_two_peers_one_device() {
        let lan = Uuid::new_v4();
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let (mac_a, mac_b) = (mac("aa:00:00:00:00:01"), mac("aa:00:00:00:00:02"));
        let candidates = vec![
            host(
                a,
                vec![snmp("10.0.0.1", lan, mac_a), snmp("10.0.0.10", lan, vrrp())],
            ),
            host(
                b,
                vec![snmp("10.0.0.2", lan, mac_b), snmp("10.0.0.10", lan, vrrp())],
            ),
        ];
        let payload_from_b = [snmp("10.0.0.2", lan, mac_b), snmp("10.0.0.10", lan, vrrp())];
        assert!(proven(&payload_from_b, b, &candidates, &[]).is_empty());
    }

    /// A stale record of an address's previous DHCP holder carries that device's MAC, which the
    /// new holder's payload does not.
    #[test]
    fn a_stale_record_of_a_reused_address_is_not_merged() {
        let lan = Uuid::new_v4();
        let (current, stale) = (Uuid::new_v4(), Uuid::new_v4());
        let (old, new_a, new_b) = (
            mac("aa:00:00:00:00:09"),
            mac("bc:24:11:00:00:01"),
            mac("bc:24:11:00:00:02"),
        );
        let candidates = vec![
            host(current, vec![arp("192.168.4.51", lan, new_b)]),
            host(stale, vec![arp("192.168.4.50", lan, old)]),
        ];
        let payload = [
            config("192.168.4.50", lan, new_a),
            config("192.168.4.51", lan, new_b),
        ];
        assert!(proven(&payload, current, &candidates, &[]).is_empty());
    }

    /// A host holding any address the payload does not claim is not accounted for.
    #[test]
    fn a_host_with_an_address_outside_the_payload_is_not_merged() {
        let lan = Uuid::new_v4();
        let (h1, h2) = (Uuid::new_v4(), Uuid::new_v4());
        let candidates = vec![
            host(h1, vec![arp("192.168.4.63", lan, ens19())]),
            host(
                h2,
                vec![
                    arp("192.168.4.126", lan, ens18()),
                    arp("192.168.4.127", lan, ens18()),
                ],
            ),
        ];
        let payload = [
            config("192.168.4.126", lan, ens18()),
            config("192.168.4.63", lan, ens19()),
        ];
        assert!(proven(&payload, h1, &candidates, &[]).is_empty());
    }

    /// A MAC a third party reports (a controller's client table, a hypervisor's guest-agent view,
    /// a router's forwarding table) does not bind the claim to the device, so it proves nothing.
    #[test]
    fn a_payload_whose_macs_a_third_party_reported_proves_nothing() {
        let lan = Uuid::new_v4();
        let (h1, h2) = (Uuid::new_v4(), Uuid::new_v4());
        let candidates = vec![
            host(h1, vec![arp("192.168.4.63", lan, ens19())]),
            host(h2, vec![arp("192.168.4.126", lan, ens18())]),
        ];
        for source in [
            AttributeSource::Probe(ClientProbe::UnifiController),
            AttributeSource::Probe(ClientProbe::Proxmox),
            AttributeSource::ForwardingTable,
        ] {
            let payload = [
                row("192.168.4.126", lan, ens18(), source),
                row("192.168.4.63", lan, ens19(), source),
            ];
            assert!(
                proven(&payload, h1, &candidates, &[]).is_empty(),
                "{source:?} must not prove a merge"
            );
        }
    }

    /// Never the matched host itself, an ipvlan container (its MAC is its runtime's), or a host
    /// carrying a daemon (only ever merged into by hand).
    #[test]
    fn the_matched_host_an_ipvlan_host_and_a_daemon_host_are_never_listed() {
        let lan = Uuid::new_v4();
        let (h1, ipvlan, daemon) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let mut ipvlan_host = host(ipvlan, vec![arp("192.168.4.126", lan, ens19())]);
        ipvlan_host.virtualization =
            Some(HostVirtualization::Docker(ContainerHostVirtualization {
                container_name: Some("ipvlan-test".to_string()),
                container_id: Some("aec2a3e9".to_string()),
                compose_project: None,
                network_type: ContainerNetworkType::IpVlan,
            }));
        let candidates = vec![
            host(h1, vec![arp("192.168.4.63", lan, ens19())]),
            ipvlan_host,
            host(daemon, vec![arp("192.168.4.64", lan, ens18())]),
        ];
        let payload = [
            config("192.168.4.63", lan, ens19()),
            config("192.168.4.126", lan, ens19()),
            config("192.168.4.64", lan, ens18()),
        ];
        let found = hosts_proven_same_device(
            &payload,
            h1,
            &candidates,
            &HashSet::new(),
            &HashSet::from([daemon]),
        );
        assert!(found.is_empty());
    }

    /// A switch whose SVIs the sweep found as three hosts: its own payload lists all three.
    #[test]
    fn a_switchs_svis_found_apart_are_proven_one_device() {
        let (v1, v2, v3) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let (s1, s2, s3) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let switch = mac("00:1b:54:00:00:01");
        let candidates = vec![
            host(s1, vec![arp("10.0.1.1", v1, switch)]),
            host(s2, vec![arp("10.0.2.1", v2, switch)]),
            host(s3, vec![arp("10.0.3.1", v3, switch)]),
        ];
        let payload = [
            snmp("10.0.1.1", v1, switch),
            snmp("10.0.2.1", v2, switch),
            snmp("10.0.3.1", v3, switch),
        ];
        assert_eq!(proven(&payload, s1, &candidates, &[]), vec![s2, s3]);
    }

    /// Two records already holding one address behind one MAC (the lab's .126 after the split)
    /// are healed by the next sweep of that address.
    #[test]
    fn a_sweep_heals_two_records_of_one_address_and_mac() {
        let lan = Uuid::new_v4();
        let (older, newer) = (Uuid::new_v4(), Uuid::new_v4());
        let candidates = vec![
            host(older, vec![arp("192.168.4.126", lan, ens18())]),
            host(newer, vec![arp("192.168.4.126", lan, ens18())]),
        ];
        let sweep = [arp("192.168.4.126", lan, ens18())];
        assert_eq!(proven(&sweep, older, &candidates, &[]), vec![newer]);
    }
}
