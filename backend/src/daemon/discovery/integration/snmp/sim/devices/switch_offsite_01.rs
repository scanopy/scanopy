use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::{inline, switch_mute_01};

/// The uplink neighbours' management addresses, on a range no device in the lab holds.
///
/// Deliberately outside `192.168.7.0/24`: these far ends are reachable from this switch and from
/// nowhere the daemon can see, which is the case the whole management-address tier exists for.
const OFFSITE_CORE: &str = "10.20.30.11";
const OFFSITE_EDGE: &str = "10.20.30.24";

/// What this switch's neighbour record calls it — deliberately *not* `switch-mute-01`, which is the
/// device's own sysName. Real firmware disagrees like this, and it is what makes the sysName tier
/// miss a device that is sitting in the host list.
const MUTE_NEIGHBOUR_SYS_NAME: &str = "Switch1";

/// A far end that identifies itself *by address* — chassis subtype 5, port subtype 4.
const ADDRESSED_NEIGHBOUR: &str = "10.20.30.40";

/// A far end in a *second* range nothing holds, on a port carrying no VLAN the others share.
///
/// Without it every far end here sits in one `/24` and inference produces a single range, so the
/// case where a later reading covers *several* assumed ranges — the one discovery deliberately
/// declines, because resolving it means deleting rows — cannot be reached in the lab at all.
const BRANCH_NEIGHBOUR: &str = "10.20.31.9";

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-offsite-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "GH #668",
            defect: "its neighbours publish a management address and nothing else this network \
                     holds, so before the address tier every one of them resolved to nothing and \
                     the ports drew as unconnected",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Offsite aggregation switch, 8-port".into()),
            sys_object_id: Some("1.3.6.1.4.1.99999.3.1".into()),
            sys_name: Some("switch-offsite-01".into()),
            sys_location: Some("Rack 2, offsite".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: tables(),
        arp_handler: Handler::Normal,
        suppresses: Vec::new(),
        rejects_getbulk: None,
    }
}

fn tables() -> Tables {
    Tables {
        if_table: Some(if_table()),
        lldp: Some(lldp()),
        ..Default::default()
    }
}

fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            1,
            "GigabitEthernet0/1",
            Some("00:1a:2b:00:fc:01".parse().unwrap()),
        )
        .speed(1000000000)
        .name("Gi0/1")
        .high_speed(),
        IfRow::port(
            2,
            "GigabitEthernet0/2",
            Some("00:1a:2b:00:fc:02".parse().unwrap()),
        )
        .speed(1000000000)
        .name("Gi0/2")
        .high_speed(),
        IfRow::port(
            3,
            "GigabitEthernet0/3",
            Some("00:1a:2b:00:fc:03".parse().unwrap()),
        )
        .speed(1000000000)
        .name("Gi0/3")
        .high_speed(),
        IfRow::port(
            4,
            "GigabitEthernet0/4",
            Some("00:1a:2b:00:fc:04".parse().unwrap()),
        )
        .speed(1000000000)
        .name("Gi0/4")
        .high_speed(),
        IfRow::port(
            5,
            "GigabitEthernet0/5",
            Some("00:1a:2b:00:fc:05".parse().unwrap()),
        )
        .speed(1000000000)
        .name("Gi0/5")
        .high_speed(),
        IfRow::port(
            6,
            "GigabitEthernet0/6",
            Some("00:1a:2b:00:fc:06".parse().unwrap()),
        )
        .speed(1000000000)
        .name("Gi0/6")
        .high_speed(),
    ])
}

/// Six uplinks, and the shapes the address tier has to tell apart.
///
/// Ports 1 and 2 name far ends that publish a management address, both in the same `10.20.30.0/24`
/// — that pair is what the server-side bucketing folds into one inferred subnet. Port 3 names a far
/// end that publishes none, which stays unplaceable no matter how much of the network is scanned
/// and is the control the other two are read against. Port 4 is GH #668 itself and port 5 carries
/// the address-as-identity subtypes; both are described at their neighbour below. Port 6 sits in a
/// *second* range, so the lab holds two assumed ranges rather than one — without it the case where
/// a later reading covers several of them is unreachable.
///
/// Every port here needs a row in [`if_table`] as well. A neighbour whose local port resolves to no
/// interface is discarded whole by `count_dropped_neighbours` before the server ever sees it —
/// silently, since the fixture still serves it and the device's own LLDP assertions still pass.
/// Ports 4 and 5 were added here and not there, so the two cases they exist to cover never reached
/// a scan; `no_lab_device_drops_an_lldp_neighbour` is what now holds the two tables together.
fn lldp() -> LldpTable {
    LldpTable::new(
        Advertised::octets(LldpChassisId::MacAddress("00:1a:2b:00:fc:00".to_string())),
        "switch-offsite-01",
    )
    .sys_desc("Offsite aggregation switch, 8-port")
    .local_ports(vec![
        LocalPort::new(
            1,
            Advertised::octets(LldpPortId::InterfaceName("GigabitEthernet0/1".to_string())),
        ),
        LocalPort::new(
            2,
            Advertised::octets(LldpPortId::InterfaceName("GigabitEthernet0/2".to_string())),
        ),
        LocalPort::new(
            3,
            Advertised::octets(LldpPortId::InterfaceName("GigabitEthernet0/3".to_string())),
        ),
        LocalPort::new(
            4,
            Advertised::octets(LldpPortId::InterfaceName("GigabitEthernet0/4".to_string())),
        ),
        LocalPort::new(
            5,
            Advertised::octets(LldpPortId::InterfaceName("GigabitEthernet0/5".to_string())),
        ),
        LocalPort::new(
            6,
            Advertised::octets(LldpPortId::InterfaceName("GigabitEthernet0/6".to_string())),
        ),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::octets(LldpChassisId::MacAddress("00:ad:24:c0:ff:e0".to_string())),
            Advertised::octets(LldpPortId::InterfaceName(
                "Ten-GigabitEthernet1/0/1".to_string(),
            )),
        )
        .sys_name("offsite-core-01")
        .mgmt_addr(OFFSITE_CORE.parse().unwrap()),
        RemoteNeighbour::new(
            2,
            Advertised::octets(LldpChassisId::MacAddress("00:ad:24:c0:ff:e1".to_string())),
            Advertised::octets(LldpPortId::InterfaceName(
                "GigabitEthernet1/0/8".to_string(),
            )),
        )
        .sys_name("offsite-edge-01")
        .mgmt_addr(OFFSITE_EDGE.parse().unwrap()),
        RemoteNeighbour::new(
            3,
            Advertised::octets(LldpChassisId::MacAddress("00:ad:24:c0:ff:e2".to_string())),
            Advertised::octets(LldpPortId::InterfaceName("eth0".to_string())),
        )
        .sys_name("offsite-ap-01"),
        // GH #668 itself. The far end is scanned and in the host list, and every tier above the
        // address is dead for it: the chassis MAC is on no interface because it serves no ifTable,
        // it publishes no LLDP local identity so `hosts.chassis_id` is null, and the sysName here
        // disagrees with its own. Its management address is the only thing left.
        RemoteNeighbour::new(
            4,
            Advertised::octets(LldpChassisId::MacAddress("00:ad:24:c0:ff:e3".to_string())),
            Advertised::octets(LldpPortId::InterfaceName("1".to_string())),
        )
        .sys_name(MUTE_NEIGHBOUR_SYS_NAME)
        // switch-mute-01's own address — a device this lab *does* scan, and which serves nothing
        // but its system MIB. It is the GH #668 "Switch1" shape: in the host list, no ifTable to
        // carry the chassis MAC its neighbours advertise, no LLDP local identity to fill
        // `hosts.chassis_id`, and a sysName its neighbours disagree with. Only the address it
        // publishes can place it — named rather than typed, so renumbering cannot strand it.
        .mgmt_addr_of(switch_mute_01::NAME),
        // Subtype 5 chassis id and subtype 4 port id — an address as the identity itself, rather
        // than alongside it. Both encode as raw octets and both had no fixture in the lab, so the
        // branches that read them were reachable only from unit tests.
        RemoteNeighbour::new(
            5,
            Advertised::octets(LldpChassisId::NetworkAddress(
                ADDRESSED_NEIGHBOUR.parse().unwrap(),
            )),
            Advertised::octets(LldpPortId::NetworkAddress(
                ADDRESSED_NEIGHBOUR.parse().unwrap(),
            )),
        )
        .sys_name("offsite-addressed-01"),
        // A second range. Inference buckets per `/24` and only widens on shared-VLAN evidence, so
        // this produces a range of its own rather than joining the others — which is what makes the
        // several-guesses-under-one-reading case reachable.
        RemoteNeighbour::new(
            6,
            Advertised::octets(LldpChassisId::MacAddress("00:ad:24:c0:ff:e4".to_string())),
            Advertised::octets(LldpPortId::InterfaceName(
                "GigabitEthernet1/0/1".to_string(),
            )),
        )
        .sys_name("offsite-branch-01")
        .mgmt_addr(BRANCH_NEIGHBOUR.parse().unwrap()),
    ])
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use super::{
        ADDRESSED_NEIGHBOUR, MUTE_NEIGHBOUR_SYS_NAME, OFFSITE_CORE, OFFSITE_EDGE, switch_mute_01,
    };
    use crate::daemon::discovery::integration::snmp::sim::harness;
    use crate::server::lldp::{LldpChassisId, LldpPortId};

    /// The management address arrives in the *index* of `lldpRemManAddrTable`, not in a column
    /// value, so a fixture that served it as a value would look correct and be invisible to the
    /// collector. This is the only end-to-end coverage that the index is composed the way
    /// `split_lldp_man_addr_index` reads it.
    #[tokio::test]
    async fn a_published_management_address_reaches_the_neighbour_record() {
        let scan = harness::scan("switch-offsite-01").await;

        assert!(
            scan.neighbours.complete,
            "the management-address walk must not report the neighbour walk as short"
        );

        let core = scan
            .neighbours_named("offsite-core-01")
            .into_iter()
            .next()
            .expect("the neighbour on Gi0/1");
        assert_eq!(
            core.remote_mgmt_addr,
            Some(OFFSITE_CORE.parse().unwrap()),
            "the address must survive the trip through the OID index"
        );

        let edge = scan
            .neighbours_named("offsite-edge-01")
            .into_iter()
            .next()
            .expect("the neighbour on Gi0/2");
        assert_eq!(edge.remote_mgmt_addr, Some(OFFSITE_EDGE.parse().unwrap()));
    }

    /// The GH #668 shape, at the fixture level: the neighbour naming `switch-mute-01` calls it
    /// something its own sysName is not, so the sysName tier cannot place it however much of this
    /// network is scanned. Asserted here so the fixture cannot quietly start agreeing and turn the
    /// resolution test that depends on it into one that proves nothing.
    #[tokio::test]
    async fn the_mute_far_end_is_named_by_a_sys_name_it_does_not_answer_to() {
        let scan = harness::scan("switch-offsite-01").await;
        let mute = crate::daemon::discovery::integration::snmp::sim::device(switch_mute_01::NAME);

        let neighbour = scan
            .neighbours_named(MUTE_NEIGHBOUR_SYS_NAME)
            .into_iter()
            .next()
            .expect("the neighbour on Gi0/4");

        assert_ne!(
            neighbour.remote_sys_name.as_deref(),
            mute.system.sys_name.as_deref(),
            "the advertised name must disagree, or the sysName tier would place it"
        );
        assert_eq!(neighbour.remote_mgmt_addr, Some(IpAddr::V4(mute.ip)));
        assert!(
            mute.tables.lldp.is_none() && mute.tables.if_table.is_none(),
            "the far end must serve no tables, or a tier above the address could place it"
        );
    }

    /// Subtype 5 and subtype 4 carry an address where every other subtype carries text, and both
    /// go out as raw octets. Nothing in the lab advertised either, so the decode was covered only
    /// by unit tests over bytes this simulator had never actually produced.
    #[tokio::test]
    async fn an_address_valued_chassis_and_port_id_survive_the_wire() {
        let scan = harness::scan("switch-offsite-01").await;

        let neighbour = scan
            .neighbours_named("offsite-addressed-01")
            .into_iter()
            .next()
            .expect("the neighbour on Gi0/5");

        let chassis = LldpChassisId::from_snmp(
            neighbour.remote_chassis_id_subtype.expect("a subtype"),
            neighbour.remote_chassis_id_bytes.as_ref().expect("a value"),
        );
        assert_eq!(
            chassis,
            Some(LldpChassisId::NetworkAddress(
                ADDRESSED_NEIGHBOUR.parse().unwrap()
            ))
        );

        let port = LldpPortId::from_snmp(
            neighbour.remote_port_id_subtype.expect("a subtype"),
            neighbour.remote_port_id_bytes.as_ref().expect("a value"),
        );
        assert_eq!(
            port,
            Some(LldpPortId::NetworkAddress(
                ADDRESSED_NEIGHBOUR.parse().unwrap()
            ))
        );
    }

    /// A far end that publishes no management address must come back with `None` rather than
    /// inheriting a neighbouring row's — the man-addr table is indexed per neighbour, and reading
    /// its index wrongly would smear one address across every row on the device.
    #[tokio::test]
    async fn a_neighbour_that_publishes_no_address_carries_none() {
        let scan = harness::scan("switch-offsite-01").await;

        let ap = scan
            .neighbours_named("offsite-ap-01")
            .into_iter()
            .next()
            .expect("the neighbour on Gi0/3");
        assert_eq!(ap.remote_mgmt_addr, None);
    }
}
