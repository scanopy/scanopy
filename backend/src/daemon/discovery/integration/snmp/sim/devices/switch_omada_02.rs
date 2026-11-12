use std::net::Ipv4Addr;
use std::time::Duration;

use mac_address::MacAddress;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::{BridgeTable, FdbEntry, VendorTree};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::{PassValue, Row};
use crate::daemon::discovery::integration::snmp::sim::{
    AgentFaults, CommunityVersions, Purpose, SimDevice, Tables,
};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::interfaces::r#impl::base::if_type;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

/// TP-Link's system-information group, the first objects after the Cisco tree a CDP walk asks
/// for. Ten scalars, then one utilization row per port.
const TPLINK_SYS_INFO: &str = "1.3.6.1.4.1.11863.6.1.1";
/// The per-port "Port RX Utilization" rows, which the switch computes when asked.
const TPLINK_PORT_UTILIZATION: &str = "1.3.6.1.4.1.11863.6.1.1.11";

/// What one utilization row costs. The reporter's `snmpbulkget -Cr20` at the CDP column read ten
/// of them and took 5.076s, against 0.064s for `-Cr10`, which stops one row short of the first.
const UTILIZATION_ROW_COST: Duration = Duration::from_millis(506);

const FIRST_PORT_IF_INDEX: i32 = 49153;
const PORTS: u32 = 10;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-omada-02",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#848",
            defect: "a GetBulk of 20 past the missing CDP subtree reads ten per-port utilization rows that take 5.06s; the answer lands after the retry is sent, and every later read on the host is served the answer to the request before it",
        },
        credential: CredentialType::SnmpCommunity {
            community: inline("public"),
        },
        community_versions: CommunityVersions::Both,
        system: SystemInfo {
            sys_descr: Some(
                "Omada 8-Port 2.5GBASE-T L2+ Managed Switch with 2 10GE SFP+ Slots".into(),
            ),
            sys_object_id: None,
            sys_name: Some("10GBS Switch".into()),
            sys_location: None,
            sys_contact: None,
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
        lldp: Some(lldp_table()),
        bridge: bridge_table(),
        vendor: vendor_tree(),
        faults: AgentFaults {
            slow_rows: vec![TPLINK_PORT_UTILIZATION],
            row_cost: UTILIZATION_ROW_COST,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn chassis() -> MacAddress {
    "24:2f:d0:00:08:48".parse().unwrap()
}

/// Ports 1 to 8 are 2.5G copper and 9 and 10 are SFP+, which is how the switch names them.
fn port_name(port: u32) -> String {
    if port <= 8 {
        format!("two-gigabitEthernet 1/0/{port}")
    } else {
        format!("ten-gigabitEthernet 1/0/{port}")
    }
}

fn port_if_index(port: u32) -> i32 {
    FIRST_PORT_IF_INDEX + port as i32 - 1
}

pub fn if_table() -> IfTable {
    let mut rows = vec![IfRow::virtual_if(1, "Vlan-interface1", if_type::L3_IPVLAN).mac(chassis())];
    rows.extend(
        (1..=PORTS).map(|port| IfRow::port(port_if_index(port), &port_name(port), Some(chassis()))),
    );
    IfTable::new(rows)
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::octets(LldpChassisId::MacAddress(chassis().to_string())),
        "10GBS Switch",
    )
    .sys_desc("Omada 8-Port 2.5GBASE-T L2+ Managed Switch with 2 10GE SFP+ Slots")
    .local_ports(
        (1..=PORTS)
            .map(|port| {
                LocalPort::new(
                    port,
                    Advertised::octets(LldpPortId::InterfaceName(port_name(port))),
                )
            })
            .collect(),
    )
    .neighbours(vec![
        RemoteNeighbour::new(
            8,
            Advertised::octets(LldpChassisId::MacAddress("24:2f:d0:00:08:08".into())),
            Advertised::octets(LldpPortId::MacAddress("24:2f:d0:00:08:08".into())),
        ),
        RemoteNeighbour::new(
            10,
            Advertised::octets(LldpChassisId::MacAddress("24:2f:d0:00:08:01".into())),
            Advertised::octets(LldpPortId::InterfaceName("eth10".into())),
        )
        .sys_name("Gateway")
        .sys_desc("Omada VPN Router with 10G Ports"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived().fdb(vec![
        FdbEntry::learned("40:ed:00:00:08:09".parse().unwrap(), 9),
        FdbEntry::learned("24:2f:d0:00:08:01".parse().unwrap(), 10),
    ])
}

fn vendor_tree() -> VendorTree {
    let text = |object: u64, value: &str| {
        Row::at(
            TPLINK_SYS_INFO,
            &[object, 0],
            PassValue::Str(value.to_string()),
        )
    };
    let mut rows = vec![
        text(
            1,
            "Omada 8-Port 2.5GBASE-T L2+ Managed Switch with 2 10GE SFP+ Slots",
        ),
        text(2, "10GBS Switch"),
        text(3, ""),
        text(4, ""),
        text(5, "SG3210X-M2 1.0"),
        text(6, "1.0.26 Build 20260509 Rel.24803"),
        text(7, "24-2F-D0-00-08-48"),
        text(8, "0000000000000"),
        Row::at(TPLINK_SYS_INFO, &[9, 0], PassValue::Integer(763_498_918)),
        Row::at(TPLINK_SYS_INFO, &[10, 0], PassValue::Integer(38400)),
    ];
    rows.extend((1..=PORTS).map(|port| {
        Row::at(
            TPLINK_PORT_UTILIZATION,
            &[1, 1, port_if_index(port) as u64],
            PassValue::Str(" Port RX Utilization  -  0.00%".to_string()),
        )
    }));
    VendorTree {
        subtree: TPLINK_SYS_INFO,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::PORTS;
    use crate::daemon::discovery::integration::snmp::oids::{cdp, oid_parts};
    use crate::daemon::discovery::integration::snmp::queries::{
        BULK_MAX_REPETITIONS, RequestTimedOut, SnmpWalkTransport, query_cdp_neighbors,
        query_lldp_local_ports,
    };
    use crate::daemon::discovery::integration::snmp::sim::harness::{self, Collected};
    use crate::daemon::discovery::integration::snmp::sim::udp::UdpAgent;
    use crate::daemon::discovery::integration::snmp::sim::{self, SimDevice};

    /// One scan through a real session. The defect is in how the session reads the socket, which
    /// the in-memory agent sits above and cannot reach.
    async fn scan_over_udp() -> (Collected, UdpAgent) {
        let device: SimDevice = sim::device("switch-omada-02");
        let agent = UdpAgent::start(&device).await;
        let mut session = agent.session(&device.credential).await;
        let collected = harness::collect_over(&mut session, &device).await;
        (collected, agent)
    }

    /// GH #848: everything read after the CDP walk came back empty, because each request was
    /// handed the answer to the one before it. The two neighbours had no local port to sit on and
    /// were discarded, and the switch had no forwarding table and no chassis ID.
    #[tokio::test]
    async fn one_scan_reads_the_tables_that_follow_the_cdp_walk() {
        let (scan, _agent) = scan_over_udp().await;

        assert_eq!(scan.neighbours.records.len(), 2);
        assert_eq!(scan.local_ports.len(), PORTS as usize);
        assert!(scan.local_ports_complete);
        assert_eq!(scan.dropped_neighbours, 0);
        assert_eq!(scan.neighbour_named("Gateway").local_port_index, 49162);

        assert!(scan.fdb.complete);
        assert_eq!(scan.fdb.records.len(), 2);
        assert!(
            scan.lldp_local.is_some(),
            "the switch's own chassis ID is the last read of the scan"
        );
    }

    /// The CDP walk finds the subtree missing without filling a page from the slow rows after it.
    #[tokio::test]
    async fn the_cdp_walk_stops_short_of_the_slow_rows() {
        let device = sim::device("switch-omada-02");
        let agent = UdpAgent::start(&device).await;
        let mut session = agent.session(&device.credential).await;

        let cdp = query_cdp_neighbors(&mut session, device.ip.into())
            .await
            .unwrap();

        assert!(cdp.records.is_empty());
        assert_eq!(agent.slow_rows_served(), 0);
    }

    /// The slow rows are the first thing after the standard MIBs too, so a page of twenty aimed
    /// at the last table the switch serves reaches them as the CDP page did. That costs the scan
    /// one timeout. Every read after it asks for the smaller page, which stops short of them.
    #[tokio::test]
    async fn one_scan_asks_for_only_one_page_the_switch_cannot_answer_in_time() {
        let (_scan, agent) = scan_over_udp().await;

        assert!(
            agent.slow_rows_served() <= PORTS as usize,
            "{} slow rows were served, more than one page of them",
            agent.slow_rows_served()
        );
    }

    /// A page the switch does take five seconds over is a timeout for the request that asked for
    /// it, and its answer, arriving while the next request is waiting, is not that request's.
    ///
    /// Takes `SNMP_TIMEOUT` of real time: the page has to be asked for and waited out.
    #[tokio::test]
    async fn an_answer_that_misses_its_deadline_is_not_served_to_the_next_request() {
        let device = sim::device("switch-omada-02");
        let agent = UdpAgent::start(&device).await;
        let mut session = agent.session(&device.credential).await;

        let slow_page = session
            .walk_getbulk(
                &oid_parts(cdp::entry::CDP_CACHE_DEVICE_ID),
                BULK_MAX_REPETITIONS,
            )
            .await
            .map(|_| ());
        assert!(
            slow_page.is_err_and(|e| e.is::<RequestTimedOut>()),
            "a page of twenty reaches the ten slow rows and outlasts the timeout"
        );

        let local_ports = query_lldp_local_ports(&mut session, device.ip.into())
            .await
            .unwrap();
        assert!(local_ports.complete);
        assert_eq!(local_ports.records.len(), PORTS as usize);
    }
}
