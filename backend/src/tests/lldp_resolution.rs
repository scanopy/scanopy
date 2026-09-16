//! `LldpResolver` against a real database.
//!
//! These run against Postgres rather than an in-memory inventory on purpose. Both defects this
//! file guards against were *SQL semantics* defects — a lookup with no `ORDER BY` and no `LIMIT`
//! returning an arbitrary row — and the fake inventory in `server::lldp`'s tests
//! re-implements those semantics in Rust, so it cannot fail the way the database did. A resolver
//! test that never touches a query plan proves the resolver agrees with itself.

use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
use crate::server::lldp::LldpInventorySnapshot;
use crate::server::shared::attribution::{AttributeSource, Attributed};
use std::net::{IpAddr, Ipv4Addr};

use uuid::Uuid;

use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
use crate::server::{
    hosts::r#impl::base::Host,
    interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, Interface, InterfaceBase, if_type},
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
    lldp::{
        IdentityResolution,
        resolver::{LldpResolver, LldpResolverImpl},
    },
    shared::storage::traits::Storage,
};

use super::{host, network, organization, subnet, test_services};
use crate::server::hosts::r#impl::attributes::{HostChassisIdValue, HostSysNameValue};
use crate::server::services::r#impl::patterns::ClientProbe;

/// What a credentialed SNMP walk claims for the fields these fixtures stand in for. Named once so
/// a fixture cannot accidentally assert a provenance no real scan produces.
const SNMP_READ: AttributeSource = AttributeSource::Probe(ClientProbe::Snmp);

/// Everything a resolution test needs: a network with hosts and interfaces in it, and a resolver
/// pointed at the same database.
struct Lab {
    resolver: LldpResolverImpl,
    storage: crate::server::shared::storage::factory::StorageFactory,
    network_id: Uuid,
    subnet_id: Uuid,
    _container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
}

impl Lab {
    async fn new() -> Self {
        let (storage, services, _container) = test_services().await;

        let org = organization();
        storage.organizations.create(&org).await.unwrap();
        let network = network(&org.id);
        storage.networks.create(&network).await.unwrap();
        let subnet = subnet(&network.id);
        storage.subnets.create(&subnet).await.unwrap();

        let resolver = LldpResolverImpl::new(
            services.interface_service.clone(),
            services.ip_address_service.clone(),
            storage.hosts.clone(),
        );

        Self {
            resolver,
            network_id: network.id,
            subnet_id: subnet.id,
            storage,
            _container,
        }
    }

    async fn host(&self, name: &str) -> Host {
        let mut h = host(&self.network_id);
        h.base.name = HostName::manual(name.to_string());
        self.storage.hosts.create(&h).await.unwrap();
        h
    }

    async fn host_with(
        &self,
        name: &str,
        chassis_id: Option<&str>,
        sys_name: Option<&str>,
    ) -> Host {
        let mut h = host(&self.network_id);
        h.base.name = HostName::manual(name.to_string());
        h.base.chassis_id =
            chassis_id.map(|v| Attributed::new(HostChassisIdValue(v.into()), SNMP_READ));
        h.base.sys_name = sys_name.map(|v| Attributed::new(HostSysNameValue(v.into()), SNMP_READ));
        self.storage.hosts.create(&h).await.unwrap();
        h
    }

    /// One interface. `if_type` decides whether the physical-only filters can see it.
    #[allow(clippy::too_many_arguments)]
    async fn interface(
        &self,
        host_id: Uuid,
        if_index: i32,
        if_descr: &str,
        if_name: Option<&str>,
        if_alias: Option<&str>,
        mac: Option<&str>,
        if_type: i32,
    ) -> Interface {
        let entry = Interface::new(InterfaceBase {
            host_id,
            network_id: self.network_id,
            if_index: Some(if_index),
            if_descr: Some(if_descr.to_string()),
            if_name: if_name.map(str::to_string),
            if_alias: if_alias.map(str::to_string),
            if_type: Some(if_type),
            mac_address: mac.map(|m| {
                MacEvidence::new(
                    MacEvidenceValue(m.parse().unwrap()),
                    AttributeSource::ArpReply,
                )
            }),
            admin_status: Some(IfAdminStatus::Up),
            oper_status: Some(IfOperStatus::Up),
            ..Default::default()
        });
        self.storage.interfaces.create(&entry).await.unwrap();
        entry
    }

    /// A physical ethernet port, which is what almost every test wants.
    async fn port(
        &self,
        host_id: Uuid,
        if_index: i32,
        descr: &str,
        mac: Option<&str>,
    ) -> Interface {
        self.interface(
            host_id,
            if_index,
            descr,
            None,
            None,
            mac,
            if_type::ETHERNET_CSMA_CD,
        )
        .await
    }

    /// A physical ethernet port the device's own SNMP `ipAddrTable` binds an address to — the
    /// tie-break `find_if_entry_by_mac` falls back to among several MAC-sharing physical rows
    /// (GH #668, a Windows NIC and its NDIS filter/LWF pseudo-interfaces).
    async fn ip_configured_port(
        &self,
        host_id: Uuid,
        if_index: i32,
        descr: &str,
        mac: Option<&str>,
    ) -> Interface {
        let entry = Interface::new(InterfaceBase {
            host_id,
            network_id: self.network_id,
            if_index: Some(if_index),
            if_descr: Some(descr.to_string()),
            if_type: Some(if_type::ETHERNET_CSMA_CD),
            mac_address: mac.map(|m| {
                MacEvidence::new(
                    MacEvidenceValue(m.parse().unwrap()),
                    AttributeSource::ArpReply,
                )
            }),
            admin_status: Some(IfAdminStatus::Up),
            oper_status: Some(IfOperStatus::Up),
            ip_configured: true,
            ..Default::default()
        });
        self.storage.interfaces.create(&entry).await.unwrap();
        entry
    }

    async fn ip(&self, host_id: Uuid, addr: Ipv4Addr, mac: Option<&str>) -> IPAddress {
        let ip = IPAddress::new(IPAddressBase {
            network_id: self.network_id,
            subnet_id: self.subnet_id,
            ip_address: IpAddr::V4(addr),
            mac_address: mac.map(|m| {
                MacEvidence::new(
                    MacEvidenceValue(m.parse().unwrap()),
                    AttributeSource::ArpReply,
                )
            }),
            position: 0,
            name: Some("eth0".to_string()),
            host_id,
        });
        self.storage.ip_addresses.create(&ip).await.unwrap();
        ip
    }

    /// Exactly what `NetworkScan::submit_dcp_host` builds: no `if_index`/`if_descr`/`if_type`/
    /// admin or oper status — a PROFINET DCP Identify carries none of that, only the MAC the
    /// device answered with. Distinct from `port`/`interface` above, which always set those
    /// fields, because a test using them here would not actually be exercising DCP's shape.
    async fn profinet_dcp_port(&self, host_id: Uuid, mac: &str) -> Interface {
        let entry = Interface::new(InterfaceBase {
            host_id,
            network_id: self.network_id,
            if_descr: None,
            mac_address: Some(MacEvidence::new(
                MacEvidenceValue(mac.parse().unwrap()),
                AttributeSource::ProfinetDcp,
            )),
            ..Default::default()
        });
        self.storage.interfaces.create(&entry).await.unwrap();
        entry
    }
}

// ============================================================================
// Host lookups
// ============================================================================

#[tokio::test]
async fn a_host_resolves_by_the_mac_on_one_of_its_addresses() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    lab.ip(
        switch.id,
        Ipv4Addr::new(192, 168, 1, 10),
        Some("00:1a:2b:00:10:01"),
    )
    .await;

    assert_eq!(
        lab.resolver
            .find_host_by_mac("00:1a:2b:00:10:01", lab.network_id)
            .await,
        IdentityResolution::Resolved(switch.id)
    );
}

/// A no-IP PROFINET device found only via DCP Identify (`AttributeSource::ProfinetDcp`) resolves
/// through the same MAC tier as any other interface — `find_host_by_mac`'s fallback query has no
/// `AttributeSource` filter at all, so a neighbouring switch's LLDP-reported chassis MAC matches
/// it exactly as it would an SNMP- or ARP-discovered one. This is the fact the whole "does a
/// managed switch's LLDP table already place a no-IP PROFINET device" story depends on — see
/// tools/dcp/DCP-TEST-ENV.md and the switch-access-01 SNMP fixture, which advertises this same
/// device's MAC as a neighbour for exactly this reason.
#[tokio::test]
async fn a_no_ip_dcp_discovered_device_resolves_through_the_mac_tier_like_any_other_source() {
    let lab = Lab::new().await;
    let device = lab.host("scanopy-dcp-sim").await;
    lab.profinet_dcp_port(device.id, "00:1a:2b:dc:90:01").await;

    assert_eq!(
        lab.resolver
            .find_host_by_mac("00:1a:2b:dc:90:01", lab.network_id)
            .await,
        IdentityResolution::Resolved(device.id),
        "a DCP-attributed MAC must resolve exactly like an SNMP- or ARP-attributed one"
    );
}

/// The fallback tier, and the reason it survives a switch that repeats one MAC everywhere: 48
/// interface rows carrying the chassis base MAC are 48 rows and *one* device, and that device is
/// the answer. Collapsing to distinct hosts before the single-match rule is what keeps this tier
/// usable on exactly the hardware that motivated the rule (GH #668).
#[tokio::test]
async fn many_ports_sharing_one_mac_still_resolve_to_their_single_host() {
    let lab = Lab::new().await;
    let switch = lab.host("d-link").await;
    for if_index in 1..=3 {
        lab.port(
            switch.id,
            if_index,
            &format!("Slot0/{if_index}"),
            Some("00:ad:24:af:4e:00"),
        )
        .await;
    }

    assert_eq!(
        lab.resolver
            .find_host_by_mac("00:ad:24:af:4e:00", lab.network_id)
            .await,
        IdentityResolution::Resolved(switch.id),
        "one MAC on many ports of one switch is still that switch"
    );
}

/// A daemon host's own NICs resolve it, even though none of them claims to be a physical port.
///
/// This is the shape the daemon reports for the machine it runs on: interface rows carrying MACs,
/// typed `propVirtual` because `pnet` gives flags rather than an IANAifType and cannot tell a real
/// NIC from a bridge, and — for the NIC that matters — no address row at all, because lldpd elects
/// a chassis MAC that need not belong to any NIC on a scanned subnet.
///
/// Both halves are load-bearing and neither is obvious. If `find_host_by_mac` ever grew an
/// `if_type` filter the way `find_if_entry_by_mac` has one, typing these rows `propVirtual` would
/// silently stop the daemon host resolving, and the only symptom would be a switch's neighbour
/// record going quietly unmatched again.
#[tokio::test]
async fn a_virtually_typed_nic_with_no_address_still_resolves_its_host() {
    let lab = Lab::new().await;
    let server = lab.host("netlab-server").await;

    // Exactly what `nic_to_interface` emits: propVirtual, MAC present, no IP row anywhere.
    lab.interface(
        server.id,
        4,
        "ens1f0np0",
        Some("ens1f0np0"),
        None,
        Some("c2:2c:ad:55:9f:ee"),
        if_type::PROP_VIRTUAL,
    )
    .await;

    assert_eq!(
        lab.resolver
            .find_host_by_mac("c2:2c:ad:55:9f:ee", lab.network_id)
            .await,
        IdentityResolution::Resolved(server.id),
        "a chassis MAC on a propVirtual NIC must still name the host it belongs to"
    );
}

/// The same MAC on two *devices* identifies neither. Picking one attaches physical links to the
/// wrong box, which reads as authoritative and is worse than reporting nothing.
#[tokio::test]
async fn one_mac_across_two_hosts_resolves_to_neither() {
    let lab = Lab::new().await;
    for name in ["switch-a", "switch-b"] {
        let h = lab.host(name).await;
        lab.port(h.id, 1, "Gi0/1", Some("00:ad:24:af:4e:00")).await;
    }

    assert_eq!(
        lab.resolver
            .find_host_by_mac("00:ad:24:af:4e:00", lab.network_id)
            .await,
        IdentityResolution::Ambiguous,
        "one MAC on two devices names neither, and says so"
    );
}

#[tokio::test]
async fn a_host_resolves_by_its_ip_and_an_unknown_ip_resolves_to_nothing() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    lab.ip(switch.id, Ipv4Addr::new(192, 168, 1, 20), None)
        .await;

    let found = lab
        .resolver
        .find_host_by_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)), lab.network_id)
        .await;
    assert_eq!(found, IdentityResolution::Resolved(switch.id));

    let missing = lab
        .resolver
        .find_host_by_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 99)), lab.network_id)
        .await;
    assert_eq!(missing, IdentityResolution::NotFound);
}

#[tokio::test]
async fn a_host_resolves_by_an_interface_description() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    lab.port(switch.id, 1, "GigabitEthernet0/1", None).await;

    assert_eq!(
        lab.resolver
            .find_host_by_if_name("GigabitEthernet0/1", lab.network_id)
            .await,
        IdentityResolution::Resolved(switch.id)
    );
}

/// `chassis_id` is how a neighbour naming a switch by its LLDP identity reaches the host record,
/// and it is not a unique column — two devices can be misconfigured with the same one.
#[tokio::test]
async fn a_chassis_id_resolves_only_when_it_names_one_host() {
    let lab = Lab::new().await;
    let netgear = lab
        .host_with("netgear", Some("00:1a:2b:00:11:00"), None)
        .await;

    assert_eq!(
        lab.resolver
            .find_host_by_chassis_id("00:1a:2b:00:11:00", lab.network_id)
            .await,
        IdentityResolution::Resolved(netgear.id)
    );

    lab.host_with("netgear-clone", Some("00:1a:2b:00:11:00"), None)
        .await;
    assert_eq!(
        lab.resolver
            .find_host_by_chassis_id("00:1a:2b:00:11:00", lab.network_id)
            .await,
        IdentityResolution::Ambiguous,
        "a chassis id on two hosts identifies neither, and reports why"
    );
}

/// Two hosts sharing an operator-assigned `sysName` is a real configuration, not a corruption.
#[tokio::test]
async fn a_sys_name_resolves_only_when_it_names_one_host() {
    let lab = Lab::new().await;
    let sw = lab.host_with("sw", None, Some("core-switch")).await;

    assert_eq!(
        lab.resolver
            .find_host_by_sys_name("core-switch", lab.network_id)
            .await,
        IdentityResolution::Resolved(sw.id)
    );

    lab.host_with("sw-2", None, Some("core-switch")).await;
    assert_eq!(
        lab.resolver
            .find_host_by_sys_name("core-switch", lab.network_id)
            .await,
        IdentityResolution::Ambiguous
    );
}

/// Every host lookup is network-scoped. A neighbour on one customer's network must never resolve
/// to an identically-addressed device on another.
#[tokio::test]
async fn host_lookups_do_not_cross_a_network_boundary() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    lab.ip(
        switch.id,
        Ipv4Addr::new(192, 168, 1, 30),
        Some("00:1a:2b:00:10:05"),
    )
    .await;

    let other_network = Uuid::new_v4();
    assert_eq!(
        lab.resolver
            .find_host_by_mac("00:1a:2b:00:10:05", other_network)
            .await,
        IdentityResolution::NotFound
    );
    assert_eq!(
        lab.resolver
            .find_host_by_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 30)), other_network)
            .await,
        IdentityResolution::NotFound
    );
}

// ============================================================================
// Interface lookups
// ============================================================================

/// GH #668, and the defect this whole type exists for. `get_one` had no `ORDER BY` and no
/// `LIMIT`, so on a switch reporting its chassis base MAC on every port it bound the neighbour to
/// whichever row Postgres returned first — not a missing link but a *wrong* one, drawn
/// port-precise and reading as authoritative.
#[tokio::test]
async fn a_mac_on_several_ports_of_one_device_is_ambiguous_not_arbitrary() {
    let lab = Lab::new().await;
    let switch = lab.host("d-link").await;
    for if_index in 1..=3 {
        lab.port(
            switch.id,
            if_index,
            &format!("Slot0/{if_index}"),
            Some("00:ad:24:af:4e:00"),
        )
        .await;
    }

    assert_eq!(
        lab.resolver
            .find_if_entry_by_mac("00:ad:24:af:4e:00", switch.id)
            .await,
        IdentityResolution::Ambiguous
    );
}

#[tokio::test]
async fn a_mac_on_exactly_one_port_resolves_to_it() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    lab.port(switch.id, 1, "eth1", Some("00:07:7c:20:01:e1"))
        .await;
    let eth3 = lab
        .port(switch.id, 3, "eth3", Some("00:07:7c:20:01:e3"))
        .await;

    assert_eq!(
        lab.resolver
            .find_if_entry_by_mac("00:07:7c:20:01:e3", switch.id)
            .await,
        IdentityResolution::Resolved(eth3.id)
    );
}

/// Virtual rows contest a lookup they can never win — a VLAN interface is not the far end of a
/// cable. On the Westermo that motivated this, six `propVirtual` VLAN rows share the chassis base
/// MAC while all ten physical ports have unique addresses; counting them turned every such lookup
/// `Ambiguous` and cost the port no physical interface had ever contested.
#[tokio::test]
async fn virtual_interfaces_sharing_a_mac_do_not_make_a_physical_port_ambiguous() {
    let lab = Lab::new().await;
    let westermo = lab.host("westermo").await;
    let eth1 = lab
        .port(westermo.id, 10, "1000-LX eth1", Some("00:07:7c:20:01:e0"))
        .await;
    for (idx, vlan) in (100..103).enumerate() {
        lab.interface(
            westermo.id,
            vlan,
            &format!("vlan{idx}"),
            None,
            None,
            Some("00:07:7c:20:01:e0"),
            if_type::PROP_VIRTUAL,
        )
        .await;
    }

    assert_eq!(
        lab.resolver
            .find_if_entry_by_mac("00:07:7c:20:01:e0", westermo.id)
            .await,
        IdentityResolution::Resolved(eth1.id),
        "only physical rows contest a port lookup"
    );
}

/// GH #668: a Windows host exposes one MAC on its real NIC and on several NDIS filter/LWF
/// pseudo-interfaces layered on top of it (WFP Native MAC Layer, QoS Packet Scheduler, WFP 802.3
/// filters). All of them report an ordinary ethernet `if_type`, so `physical_if_types()` alone
/// cannot tell them apart — before the fix this is exactly
/// `a_mac_on_several_ports_of_one_device_is_ambiguous_not_arbitrary`'s shape and stays `Ambiguous`.
/// `ipAddrTable` only ever binds the device's own IP address to the real adapter, never to a
/// filter driver riding on top of it, so `ip_configured` breaks the tie.
#[tokio::test]
async fn the_ip_configured_physical_port_resolves_among_mac_sharing_siblings() {
    let lab = Lab::new().await;
    let pc = lab.host("pc-069").await;
    let nic = lab
        .ip_configured_port(
            pc.id,
            7,
            "Realtek Gaming 2.5GbE Family Controller",
            Some("50:eb:f6:26:54:79"),
        )
        .await;
    for (idx, if_index) in (18..21).enumerate() {
        lab.port(
            pc.id,
            if_index,
            &format!("Realtek Gaming 2.5GbE Family Controller-Filter-{idx}"),
            Some("50:eb:f6:26:54:79"),
        )
        .await;
    }

    assert_eq!(
        lab.resolver
            .find_if_entry_by_mac("50:eb:f6:26:54:79", pc.id)
            .await,
        IdentityResolution::Resolved(nic.id),
        "the ipAddrTable-bound interface is the physical NIC, not one of its filter siblings"
    );
}

/// The other half of the same rule: the tie-break must not guess. Two physical rows sharing a MAC
/// where *neither* carries the device's own IP binding is exactly as unresolvable today as it was
/// before `ip_configured` existed — trading a missing link for a wrong one is the one thing this
/// fix must never do.
#[tokio::test]
async fn mac_sharing_ports_with_no_ip_configured_candidate_stay_ambiguous() {
    let lab = Lab::new().await;
    let switch = lab.host("d-link").await;
    for if_index in 1..=3 {
        lab.port(
            switch.id,
            if_index,
            &format!("Slot0/{if_index}"),
            Some("00:ad:24:af:4e:00"),
        )
        .await;
    }

    assert_eq!(
        lab.resolver
            .find_if_entry_by_mac("00:ad:24:af:4e:00", switch.id)
            .await,
        IdentityResolution::Ambiguous
    );
}

/// And the symmetric case: if the tie-break signal itself is tied — two candidates both carrying
/// an IP binding, e.g. a NIC-teaming misconfiguration or a non-native SNMP agent exposing more
/// than one adapter as IP-bound — that is still not a basis for picking one arbitrarily.
#[tokio::test]
async fn two_ip_configured_candidates_sharing_a_mac_stay_ambiguous() {
    let lab = Lab::new().await;
    let host = lab.host("teamed-nics").await;
    lab.ip_configured_port(host.id, 1, "nic-a", Some("00:1a:2b:00:10:00"))
        .await;
    lab.ip_configured_port(host.id, 2, "nic-b", Some("00:1a:2b:00:10:00"))
        .await;

    assert_eq!(
        lab.resolver
            .find_if_entry_by_mac("00:1a:2b:00:10:00", host.id)
            .await,
        IdentityResolution::Ambiguous,
        "two equally-plausible candidates must not be resolved arbitrarily"
    );
}

#[tokio::test]
async fn an_unparseable_or_unknown_mac_is_not_found_rather_than_ambiguous() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    lab.port(switch.id, 1, "eth1", Some("00:07:7c:20:01:e1"))
        .await;

    for mac in ["not-a-mac", "00:00:00:00:00:99"] {
        assert_eq!(
            lab.resolver.find_if_entry_by_mac(mac, switch.id).await,
            IdentityResolution::NotFound,
            "{mac}"
        );
    }
}

/// The name ladder: `if_descr`, then `if_name`, then `if_alias`. On Westermo WeOS the `ifDescr`
/// carries the media type in front of the name ("100-T eth9") while `ifName` and `ifAlias` hold
/// the bare "eth9", so a neighbour advertising the bare name matches only the later tiers.
#[tokio::test]
async fn a_port_name_resolves_through_descr_then_name_then_alias() {
    let lab = Lab::new().await;
    let switch = lab.host("westermo").await;

    let by_descr = lab.port(switch.id, 1, "GigabitEthernet0/1", None).await;
    let by_name = lab
        .interface(
            switch.id,
            2,
            "100-T eth9",
            Some("eth9"),
            None,
            None,
            if_type::ETHERNET_CSMA_CD,
        )
        .await;
    let by_alias = lab
        .interface(
            switch.id,
            3,
            "100-T eth8",
            None,
            Some("uplink-to-core"),
            None,
            if_type::ETHERNET_CSMA_CD,
        )
        .await;

    for (name, expected) in [
        ("GigabitEthernet0/1", by_descr.id),
        ("eth9", by_name.id),
        ("uplink-to-core", by_alias.id),
    ] {
        assert_eq!(
            lab.resolver.find_if_entry_by_name(name, switch.id).await,
            Some(expected),
            "{name}"
        );
    }
}

/// MikroTik RouterOS advertises a bridged port as `<bridge>/<port>`, which matches no stored
/// column. The suffix retry is what makes port-level resolution work there at all.
#[tokio::test]
async fn a_bridge_qualified_port_name_resolves_on_its_suffix() {
    let lab = Lab::new().await;
    let router = lab.host("mikrotik").await;
    let ether4 = lab.port(router.id, 4, "ether4-Center", None).await;

    assert_eq!(
        lab.resolver
            .find_if_entry_by_name("bridge-LAN/ether4-Center", router.id)
            .await,
        Some(ether4.id)
    );
}

/// Every tier of the name ladder is on a non-unique column, and this is the densest concentration
/// of that hazard in the file: six lookups, any of which could have returned an arbitrary row.
#[tokio::test]
async fn a_port_name_naming_two_interfaces_resolves_to_neither() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    lab.port(switch.id, 1, "Ethernet1", None).await;
    lab.port(switch.id, 2, "Ethernet1", None).await;

    assert_eq!(
        lab.resolver
            .find_if_entry_by_name("Ethernet1", switch.id)
            .await,
        None,
        "a description naming two ports names neither"
    );
}

#[tokio::test]
async fn a_port_resolves_by_if_index_within_its_own_host() {
    let lab = Lab::new().await;
    let a = lab.host("switch-a").await;
    let b = lab.host("switch-b").await;
    let a_port = lab.port(a.id, 7, "Gi0/7", None).await;
    lab.port(b.id, 7, "Gi0/7", None).await;

    assert_eq!(
        lab.resolver.find_if_entry_by_if_index(7, a.id).await,
        Some(a_port.id),
        "the same ifIndex on another host must not win"
    );
    assert_eq!(lab.resolver.find_if_entry_by_if_index(99, a.id).await, None);
}

#[tokio::test]
async fn a_port_resolves_by_the_ip_bound_to_it() {
    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    let addr = lab
        .ip(switch.id, Ipv4Addr::new(192, 168, 1, 40), None)
        .await;

    let entry = Interface::new(InterfaceBase {
        host_id: switch.id,
        network_id: lab.network_id,
        if_index: Some(1),
        if_descr: Some("Vlan10".to_string()),
        if_type: Some(if_type::ETHERNET_CSMA_CD),
        ip_address_id: Some(addr.id),
        admin_status: Some(IfAdminStatus::Up),
        oper_status: Some(IfOperStatus::Up),
        ..Default::default()
    });
    lab.storage.interfaces.create(&entry).await.unwrap();

    assert_eq!(
        lab.resolver
            .find_if_entry_by_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 40)), switch.id)
            .await,
        Some(entry.id)
    );
}

/// The SCD2 half of the same class of defect (commit `a0292545c`).
///
/// Interfaces are snapshot-tracked: an edit leaves a closed historical copy of the row beside the
/// live one, both carrying the same `if_descr` and MAC. A lookup that does not scope to live rows
/// therefore matches two, and the one it used to return was whichever the database emitted first
/// — sometimes a snapshot, which resolves a neighbour onto a version of a port that no longer
/// exists.
///
/// `LIMIT 2` alone would not save this: with the snapshot visible the honest answer is
/// `Ambiguous`, which is still not the port. `.live()` is what makes the filter identify one row,
/// and this asserts the lookups carry it.
#[tokio::test]
async fn a_closed_snapshot_copy_does_not_contest_its_own_live_row() {
    use crate::server::shared::storage::snapshot::Snapshotable;

    let lab = Lab::new().await;
    let switch = lab.host("switch").await;
    let live = lab
        .port(
            switch.id,
            1,
            "GigabitEthernet0/1",
            Some("00:1a:2b:00:10:01"),
        )
        .await;

    // A closed historical copy of the same port, exactly as an edit would leave behind.
    let mut closed = live.make_closed_copy(chrono::Utc::now());
    closed.id = Uuid::new_v4();
    lab.storage.interfaces.create(&closed).await.unwrap();

    assert_eq!(
        lab.resolver
            .find_if_entry_by_name("GigabitEthernet0/1", switch.id)
            .await,
        Some(live.id),
        "a name lookup must reach the live port, not its snapshot"
    );
    assert_eq!(
        lab.resolver
            .find_if_entry_by_mac("00:1a:2b:00:10:01", switch.id)
            .await,
        IdentityResolution::Resolved(live.id),
        "a snapshot sharing the MAC must not make the live port ambiguous"
    );
    assert_eq!(
        lab.resolver.find_if_entry_by_if_index(1, switch.id).await,
        Some(live.id)
    );
    assert_eq!(
        lab.resolver
            .find_host_by_if_name("GigabitEthernet0/1", lab.network_id)
            .await,
        IdentityResolution::Resolved(switch.id)
    );
}

/// Interface lookups are host-scoped, so a port name common across a fleet ("eth0") cannot pull
/// a neighbour onto a different device.
#[tokio::test]
async fn interface_lookups_do_not_cross_a_host_boundary() {
    let lab = Lab::new().await;
    let a = lab.host("switch-a").await;
    let b = lab.host("switch-b").await;
    lab.port(a.id, 1, "eth0", Some("00:1a:2b:00:10:01")).await;

    assert_eq!(lab.resolver.find_if_entry_by_name("eth0", b.id).await, None);
    assert_eq!(
        lab.resolver
            .find_if_entry_by_mac("00:1a:2b:00:10:01", b.id)
            .await,
        IdentityResolution::NotFound
    );
}

// ---------------------------------------------------------------------------
// The snapshot resolver must answer exactly as the queries do
// ---------------------------------------------------------------------------

/// `LldpInventorySnapshot` serves the same lookups from a preloaded network instead of a query
/// apiece, so that the resolution pass stops scaling with round-trips. That is only safe if it is
/// *indistinguishable* from the queries, and every interesting case here is one where the honest
/// answer is "this identifies nothing": a name two ports share, a MAC two hosts carry, a virtual
/// row that must not contest a physical lookup.
///
/// Asserting the two implementations against each other rather than against expected values is
/// deliberate. Restating the verdicts would fossilise today's ladder into a second place that has
/// to be edited whenever the real one changes; comparing them keeps the query implementation as
/// the specification and fails only when they disagree.
#[tokio::test]
async fn the_snapshot_resolver_answers_exactly_as_the_queries_do() {
    let lab = Lab::new().await;

    // A device identified every way the ladder knows, with a port named across all three columns.
    let switch = lab
        .host_with("switch-a", Some("00:11:22:33:44:00"), Some("switch-a"))
        .await;
    lab.interface(
        switch.id,
        1,
        "GigabitEthernet1/0/1",
        Some("Gi1/0/1"),
        Some("uplink"),
        Some("00:11:22:33:44:01"),
        if_type::ETHERNET_CSMA_CD,
    )
    .await;
    // A virtual row carrying the chassis MAC: excluded from the physical-only port lookup, but it
    // still contributes its host to the network-wide MAC tier.
    lab.interface(
        switch.id,
        2,
        "Vlan10",
        Some("Vl10"),
        None,
        Some("00:11:22:33:44:00"),
        if_type::PROP_VIRTUAL,
    )
    .await;
    // Two ports of one host sharing a MAC: ambiguous for the per-host port lookup.
    lab.port(switch.id, 3, "Gi1/0/3", Some("00:11:22:33:44:09"))
        .await;
    lab.port(switch.id, 4, "Gi1/0/4", Some("00:11:22:33:44:09"))
        .await;
    lab.ip(
        switch.id,
        Ipv4Addr::new(10, 9, 0, 1),
        Some("00:11:22:33:44:0a"),
    )
    .await;

    // A second device sharing the first's sys_name, so that tier identifies neither.
    let twin = lab.host_with("switch-b", None, Some("switch-a")).await;
    lab.port(twin.id, 1, "GigabitEthernet1/0/1", None).await;

    // A MikroTik-style bridged port, reached only by the suffix fallback.
    let mikrotik = lab.host_with("mikrotik", None, Some("mikrotik")).await;
    lab.port(mikrotik.id, 1, "ether4-Center", None).await;

    // GH #668: two physical ports sharing a MAC, one of them ipAddrTable-bound — the DB-backed
    // and in-memory resolvers must agree on the tie-break, not just on the plain-ambiguous case.
    let pc = lab.host("pc-069").await;
    lab.ip_configured_port(pc.id, 7, "Realtek NIC", Some("50:eb:f6:26:54:79"))
        .await;
    lab.port(pc.id, 18, "Realtek NIC-Filter-0", Some("50:eb:f6:26:54:79"))
        .await;

    let snapshot = LldpInventorySnapshot::new(
        &lab.storage.hosts.get_all(Default::default()).await.unwrap(),
        &lab.storage
            .interfaces
            .get_all(Default::default())
            .await
            .unwrap(),
        &lab.storage
            .ip_addresses
            .get_all(Default::default())
            .await
            .unwrap(),
    );

    let net = lab.network_id;
    for chassis in ["00:11:22:33:44:00", "does-not-exist"] {
        assert_eq!(
            snapshot.find_host_by_chassis_id(chassis, net).await,
            lab.resolver.find_host_by_chassis_id(chassis, net).await,
            "find_host_by_chassis_id({chassis})"
        );
    }
    for sys_name in ["switch-a", "mikrotik", "nobody"] {
        assert_eq!(
            snapshot.find_host_by_sys_name(sys_name, net).await,
            lab.resolver.find_host_by_sys_name(sys_name, net).await,
            "find_host_by_sys_name({sys_name})"
        );
    }
    for mac in [
        "00:11:22:33:44:00", // on a virtual row, still names its host
        "00:11:22:33:44:0a", // on an address row
        "00:11:22:33:44:09", // two ports, one host
        "00:00:00:00:00:99", // nothing
        "not-a-mac",
    ] {
        assert_eq!(
            snapshot.find_host_by_mac(mac, net).await,
            lab.resolver.find_host_by_mac(mac, net).await,
            "find_host_by_mac({mac})"
        );
    }
    for name in ["GigabitEthernet1/0/1", "Vlan10", "absent"] {
        assert_eq!(
            snapshot.find_host_by_if_name(name, net).await,
            lab.resolver.find_host_by_if_name(name, net).await,
            "find_host_by_if_name({name})"
        );
    }
    for addr in [Ipv4Addr::new(10, 9, 0, 1), Ipv4Addr::new(10, 9, 0, 254)] {
        let ip = IpAddr::V4(addr);
        assert_eq!(
            snapshot.find_host_by_ip(&ip, net).await,
            lab.resolver.find_host_by_ip(&ip, net).await,
            "find_host_by_ip({ip})"
        );
    }

    for (host_id, mac) in [
        (switch.id, "00:11:22:33:44:01"), // one physical port
        (switch.id, "00:11:22:33:44:00"), // virtual only, so invisible here
        (switch.id, "00:11:22:33:44:09"), // two physical ports
        (switch.id, "00:00:00:00:00:99"),
        (pc.id, "50:eb:f6:26:54:79"), // two physical ports, one ip_configured
    ] {
        assert_eq!(
            snapshot.find_if_entry_by_mac(mac, host_id).await,
            lab.resolver.find_if_entry_by_mac(mac, host_id).await,
            "find_if_entry_by_mac({mac})"
        );
    }
    for (host_id, name) in [
        (switch.id, "GigabitEthernet1/0/1"),       // if_descr
        (switch.id, "Gi1/0/1"),                    // if_name
        (switch.id, "uplink"),                     // if_alias
        (mikrotik.id, "bridge-LAN/ether4-Center"), // suffix fallback
        (mikrotik.id, "bridge-LAN/"),              // empty suffix
        (switch.id, "no-such-port"),
    ] {
        assert_eq!(
            snapshot.find_if_entry_by_name(name, host_id).await,
            lab.resolver.find_if_entry_by_name(name, host_id).await,
            "find_if_entry_by_name({name})"
        );
    }
    for (host_id, if_index) in [(switch.id, 1), (switch.id, 99)] {
        assert_eq!(
            snapshot.find_if_entry_by_if_index(if_index, host_id).await,
            lab.resolver
                .find_if_entry_by_if_index(if_index, host_id)
                .await,
            "find_if_entry_by_if_index({if_index})"
        );
    }
    for addr in [Ipv4Addr::new(10, 9, 0, 1), Ipv4Addr::new(10, 9, 0, 254)] {
        let ip = IpAddr::V4(addr);
        assert_eq!(
            snapshot.find_if_entry_by_ip(&ip, switch.id).await,
            lab.resolver.find_if_entry_by_ip(&ip, switch.id).await,
            "find_if_entry_by_ip({ip})"
        );
    }
}
