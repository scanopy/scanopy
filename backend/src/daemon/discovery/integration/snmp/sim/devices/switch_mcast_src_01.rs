use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour, TimeMark,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::BridgeTable;
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::interfaces::r#impl::base::if_type;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

/// GH #701: the other of two hosts behind the shared bridge, alongside `switch-mcast-rcv-01` and
/// `switch-segment-gw-01`. See `switch_segment_gw_01` for the shape this trio exercises.
pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-mcast-src-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "GH #701",
            defect: "a shared L2 segment's uplink hears two neighbours on one port; the old \
                     single-neighbour collection kept only the first and resolution could bind \
                     the far end to only one of them",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Multicast Source OS 1.4, simplified SNMP profile".into()),
            sys_object_id: Some("1.3.6.1.4.1.99999.1.2".into()),
            sys_name: Some("switch-mcast-src-01".into()),
            sys_location: Some("Lab segment C, shared uplink".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
            if_number: None,
        },
        tables: tables(),
        arp_handler: Handler::Normal,
        suppresses: Vec::new(),
        rejects_getbulk: None,
    }
}

pub const CHASSIS_MAC: &str = "02:00:00:00:02:01";
pub const UPLINK_PORT_MAC: &str = "02:00:00:00:02:02";
const SYS_NAME: &str = "switch-mcast-src-01";

fn tables() -> Tables {
    Tables {
        if_table: Some(if_table()),
        lldp: Some(lldp_table()),
        bridge: BridgeTable::derived(),
        ..Default::default()
    }
}

fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::virtual_if(1, "lo", if_type::SOFTWARE_LOOPBACK)
            .mac("00:00:00:00:00:00".parse().unwrap())
            .name("lo")
            .alias("lo"),
        IfRow::port(2, "uplink0", Some(UPLINK_PORT_MAC.parse().unwrap()))
            .speed(1_000_000_000)
            .name("uplink0")
            .alias("shared segment uplink"),
    ])
}

fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::octets(LldpChassisId::MacAddress(CHASSIS_MAC.into())),
        SYS_NAME,
    )
    .sys_desc("Multicast Source OS 1.4, simplified SNMP profile")
    .local_ports(vec![
        LocalPort::new(
            2,
            Advertised::octets(LldpPortId::MacAddress(UPLINK_PORT_MAC.into())),
        )
        .desc("uplink0"),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            2,
            Advertised::octets(LldpChassisId::MacAddress(
                super::switch_segment_gw_01::CHASSIS_MAC.into(),
            )),
            Advertised::octets(LldpPortId::MacAddress(
                super::switch_segment_gw_01::UPLINK_PORT_MAC.into(),
            )),
        )
        .time_mark(TimeMark::At(100))
        .index(1)
        .sys_name("switch-segment-gw-01")
        .sys_desc("Segment Gateway OS 3.2, simplified SNMP profile"),
        RemoteNeighbour::new(
            2,
            Advertised::octets(LldpChassisId::MacAddress(
                super::switch_mcast_rcv_01::CHASSIS_MAC.into(),
            )),
            Advertised::octets(LldpPortId::MacAddress(
                super::switch_mcast_rcv_01::UPLINK_PORT_MAC.into(),
            )),
        )
        .time_mark(TimeMark::At(100))
        .index(2)
        .sys_name("switch-mcast-rcv-01")
        .sys_desc("Multicast Receiver OS 1.4, simplified SNMP profile"),
    ])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// GH #701: this device's single uplink hears both other devices on the shared segment.
    #[tokio::test]
    async fn uplink_hears_both_shared_segment_neighbours() {
        let scan = harness::scan("switch-mcast-src-01").await;

        assert_eq!(
            scan.neighbours.records.len(),
            2,
            "both shared-segment neighbours must be collected, not just the first"
        );
        assert_eq!(
            scan.dropped_neighbours, 0,
            "neither neighbour names an ifIndex this device lacks, so none should be dropped"
        );
    }

    /// The trio-wide property: scanning all three devices covers 6 LLDP records total (2 per
    /// port × 3 ports) — the direct characterization of GH #701's "only 2 of 3 edges render"
    /// report at the collection layer, independent of any one device's own count.
    #[tokio::test]
    async fn the_trio_together_covers_six_lldp_records() {
        let gw = harness::scan("switch-segment-gw-01").await;
        let rcv = harness::scan("switch-mcast-rcv-01").await;
        let src = harness::scan("switch-mcast-src-01").await;

        let total = gw.neighbours.records.len()
            + rcv.neighbours.records.len()
            + src.neighbours.records.len();
        assert_eq!(
            total, 6,
            "3 devices each hearing both of the other two is 6 LLDP records total"
        );
        assert_eq!(
            gw.dropped_neighbours + rcv.dropped_neighbours + src.dropped_neighbours,
            0
        );
    }
}
