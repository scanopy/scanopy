//! What the SNMP integration asks a device for, as the docs' SNMP reference renders it
//! (`snmp.json`).
//!
//! [`REQUESTED_MIBS`] names every object the queries in [`super::queries`] request, by the same
//! `oids` constants they use. The test below runs every query against every simulated device and
//! fails in both directions: on a request this catalog does not list, and on a listed object that
//! no query requests.

use std::time::Duration;

use serde::Serialize;

use super::oids;
use super::session::{
    MAX_WALK_ENTRIES, SNMP_PROBE_TIMEOUT, SNMP_SESSION_TIMEOUT, SNMP_TIMEOUT, SNMP_WALK_TIMEOUT,
};
use super::{SNMP_PORTS, SnmpIntegration};
use crate::daemon::discovery::integration::DiscoveryIntegration;
use crate::server::credentials::r#impl::types::snmp::{
    SnmpV3AuthProtocol, SnmpV3PrivProtocol, SnmpV3SecurityLevel, SnmpVersion,
};
use crate::server::shared::types::field_definition::FieldSetValue;

/// How an object is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SnmpRequest {
    /// A scalar GET of one instance.
    Get,
    /// A walk of a table column.
    Walk,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SnmpObject {
    pub name: &'static str,
    pub oid: &'static str,
    pub request: SnmpRequest,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SnmpMib {
    pub name: &'static str,
    pub standard: &'static str,
    pub objects: &'static [SnmpObject],
}

const fn get(name: &'static str, oid: &'static str) -> SnmpObject {
    SnmpObject {
        name,
        oid,
        request: SnmpRequest::Get,
    }
}

const fn walk(name: &'static str, oid: &'static str) -> SnmpObject {
    SnmpObject {
        name,
        oid,
        request: SnmpRequest::Walk,
    }
}

/// Every object the SNMP queries request, grouped by the MIB that defines it.
pub const REQUESTED_MIBS: &[SnmpMib] = &[
    SnmpMib {
        name: "SNMPv2-MIB",
        standard: "RFC 3418",
        objects: &[
            get("sysDescr", oids::system::SYS_DESCR),
            get("sysObjectID", oids::system::SYS_OBJECT_ID),
            get("sysUpTime", oids::system::SYS_UPTIME),
            get("sysContact", oids::system::SYS_CONTACT),
            get("sysName", oids::system::SYS_NAME),
            get("sysLocation", oids::system::SYS_LOCATION),
            get("sysServices", oids::system::SYS_SERVICES),
        ],
    },
    SnmpMib {
        name: "IF-MIB",
        standard: "RFC 2863",
        objects: &[
            get("ifNumber", oids::if_mib::IF_NUMBER),
            walk("ifIndex", oids::if_mib::columns::IF_INDEX),
            walk("ifDescr", oids::if_mib::columns::IF_DESCR),
            walk("ifType", oids::if_mib::columns::IF_TYPE),
            walk("ifMtu", oids::if_mib::columns::IF_MTU),
            walk("ifSpeed", oids::if_mib::columns::IF_SPEED),
            walk("ifPhysAddress", oids::if_mib::columns::IF_PHYS_ADDRESS),
            walk("ifAdminStatus", oids::if_mib::columns::IF_ADMIN_STATUS),
            walk("ifOperStatus", oids::if_mib::columns::IF_OPER_STATUS),
            walk("ifName", oids::if_mib::if_x_table::IF_NAME),
            walk("ifHighSpeed", oids::if_mib::if_x_table::IF_HIGH_SPEED),
            walk("ifAlias", oids::if_mib::if_x_table::IF_ALIAS),
        ],
    },
    SnmpMib {
        name: "IP-MIB",
        standard: "RFC 4293",
        objects: &[
            walk(
                "ipAdEntIfIndex",
                oids::ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX,
            ),
            walk(
                "ipAdEntNetMask",
                oids::ip_mib::ip_addr_entry::IP_AD_ENT_NET_MASK,
            ),
            walk(
                "ipNetToMediaIfIndex",
                oids::arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
            ),
            walk(
                "ipNetToMediaPhysAddress",
                oids::arp::entry::IP_NET_TO_MEDIA_PHYS_ADDRESS,
            ),
            walk(
                "ipNetToMediaNetAddress",
                oids::arp::entry::IP_NET_TO_MEDIA_NET_ADDRESS,
            ),
            walk("ipNetToMediaType", oids::arp::entry::IP_NET_TO_MEDIA_TYPE),
        ],
    },
    SnmpMib {
        name: "LLDP-MIB",
        standard: "IEEE 802.1AB-2005",
        objects: &[
            get(
                "lldpLocChassisIdSubtype",
                oids::lldp::local::LLDP_LOC_CHASSIS_ID_SUBTYPE,
            ),
            get("lldpLocChassisId", oids::lldp::local::LLDP_LOC_CHASSIS_ID),
            walk(
                "lldpLocPortIdSubtype",
                oids::lldp::local::LLDP_LOC_PORT_ID_SUBTYPE,
            ),
            walk("lldpLocPortId", oids::lldp::local::LLDP_LOC_PORT_ID),
            walk("lldpLocPortDesc", oids::lldp::local::LLDP_LOC_PORT_DESC),
            walk(
                "lldpRemChassisIdSubtype",
                oids::lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE,
            ),
            walk(
                "lldpRemChassisId",
                oids::lldp::remote::entry::LLDP_REM_CHASSIS_ID,
            ),
            walk(
                "lldpRemPortIdSubtype",
                oids::lldp::remote::entry::LLDP_REM_PORT_ID_SUBTYPE,
            ),
            walk("lldpRemPortId", oids::lldp::remote::entry::LLDP_REM_PORT_ID),
            walk(
                "lldpRemPortDesc",
                oids::lldp::remote::entry::LLDP_REM_PORT_DESC,
            ),
            walk(
                "lldpRemSysName",
                oids::lldp::remote::entry::LLDP_REM_SYS_NAME,
            ),
            walk(
                "lldpRemSysDesc",
                oids::lldp::remote::entry::LLDP_REM_SYS_DESC,
            ),
            walk(
                "lldpRemManAddrIfSubtype",
                oids::lldp::remote::entry::LLDP_REM_MAN_ADDR_IF_SUBTYPE,
            ),
        ],
    },
    SnmpMib {
        name: "LLDP-V2-MIB",
        standard: "IEEE 802.1AB-2009",
        objects: &[
            get(
                "lldpV2LocChassisIdSubtype",
                oids::lldp_v2::local::LLDP_V2_LOC_CHASSIS_ID_SUBTYPE,
            ),
            get(
                "lldpV2LocChassisId",
                oids::lldp_v2::local::LLDP_V2_LOC_CHASSIS_ID,
            ),
            walk(
                "lldpV2RemChassisIdSubtype",
                oids::lldp_v2::remote::entry::LLDP_V2_REM_CHASSIS_ID_SUBTYPE,
            ),
            walk(
                "lldpV2RemChassisId",
                oids::lldp_v2::remote::entry::LLDP_V2_REM_CHASSIS_ID,
            ),
            walk(
                "lldpV2RemPortIdSubtype",
                oids::lldp_v2::remote::entry::LLDP_V2_REM_PORT_ID_SUBTYPE,
            ),
            walk(
                "lldpV2RemPortId",
                oids::lldp_v2::remote::entry::LLDP_V2_REM_PORT_ID,
            ),
            walk(
                "lldpV2RemPortDesc",
                oids::lldp_v2::remote::entry::LLDP_V2_REM_PORT_DESC,
            ),
            walk(
                "lldpV2RemSysName",
                oids::lldp_v2::remote::entry::LLDP_V2_REM_SYS_NAME,
            ),
            walk(
                "lldpV2RemSysDesc",
                oids::lldp_v2::remote::entry::LLDP_V2_REM_SYS_DESC,
            ),
            walk(
                "lldpV2RemManAddrIfSubtype",
                oids::lldp_v2::remote::entry::LLDP_V2_REM_MAN_ADDR_IF_SUBTYPE,
            ),
        ],
    },
    SnmpMib {
        name: "CISCO-CDP-MIB",
        standard: "Cisco",
        objects: &[
            walk("cdpCacheDeviceId", oids::cdp::entry::CDP_CACHE_DEVICE_ID),
            walk(
                "cdpCacheDevicePort",
                oids::cdp::entry::CDP_CACHE_DEVICE_PORT,
            ),
            walk("cdpCachePlatform", oids::cdp::entry::CDP_CACHE_PLATFORM),
            walk("cdpCacheAddress", oids::cdp::entry::CDP_CACHE_ADDRESS),
        ],
    },
    SnmpMib {
        name: "ENTITY-MIB",
        standard: "RFC 6933",
        objects: &[
            walk("entPhysicalDescr", oids::entity::entry::ENT_PHYSICAL_DESCR),
            walk("entPhysicalClass", oids::entity::entry::ENT_PHYSICAL_CLASS),
            walk("entPhysicalName", oids::entity::entry::ENT_PHYSICAL_NAME),
            walk(
                "entPhysicalFirmwareRev",
                oids::entity::entry::ENT_PHYSICAL_FIRMWARE_REV,
            ),
            walk(
                "entPhysicalSoftwareRev",
                oids::entity::entry::ENT_PHYSICAL_SOFTWARE_REV,
            ),
            walk(
                "entPhysicalSerialNum",
                oids::entity::entry::ENT_PHYSICAL_SERIAL_NUM,
            ),
            walk(
                "entPhysicalMfgName",
                oids::entity::entry::ENT_PHYSICAL_MFG_NAME,
            ),
            walk(
                "entPhysicalModelName",
                oids::entity::entry::ENT_PHYSICAL_MODEL_NAME,
            ),
            walk(
                "entPhysicalAssetID",
                oids::entity::entry::ENT_PHYSICAL_ASSET_ID,
            ),
        ],
    },
    SnmpMib {
        name: "BRIDGE-MIB",
        standard: "RFC 4188",
        objects: &[
            get("dot1dBaseNumPorts", oids::bridge::DOT1D_BASE_NUM_PORTS),
            walk(
                "dot1dBasePortIfIndex",
                oids::bridge::DOT1D_BASE_PORT_IF_INDEX,
            ),
            walk(
                "dot1dTpFdbAddress",
                oids::bridge::fdb_entry::DOT1D_TP_FDB_ADDRESS,
            ),
            walk("dot1dTpFdbPort", oids::bridge::fdb_entry::DOT1D_TP_FDB_PORT),
            walk(
                "dot1dTpFdbStatus",
                oids::bridge::fdb_entry::DOT1D_TP_FDB_STATUS,
            ),
        ],
    },
    SnmpMib {
        name: "Q-BRIDGE-MIB",
        standard: "RFC 4363",
        objects: &[
            walk(
                "dot1qTpFdbPort",
                oids::bridge::q_fdb_entry::DOT1Q_TP_FDB_PORT,
            ),
            walk(
                "dot1qTpFdbStatus",
                oids::bridge::q_fdb_entry::DOT1Q_TP_FDB_STATUS,
            ),
            walk(
                "dot1qVlanStaticName",
                oids::vlan::q_bridge::DOT1Q_VLAN_STATIC_NAME,
            ),
            walk("dot1qPvid", oids::vlan::q_bridge::DOT1Q_PVID),
            walk(
                "dot1qVlanCurrentEgressPorts",
                oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_EGRESS_PORTS,
            ),
            walk(
                "dot1qVlanCurrentUntaggedPorts",
                oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_UNTAGGED_PORTS,
            ),
        ],
    },
    SnmpMib {
        name: "CISCO-VTP-MIB",
        standard: "Cisco",
        objects: &[walk("vtpVlanName", oids::vlan::cisco_vtp::VTP_VLAN_NAME)],
    },
    SnmpMib {
        name: "TPLINK-DOT1Q-VLAN-MIB",
        standard: "TP-Link",
        objects: &[
            walk(
                "dot1qVlanDescription",
                oids::vlan::tplink::DOT1Q_VLAN_DESCRIPTION,
            ),
            walk("dot1qVlanStatus", oids::vlan::tplink::DOT1Q_VLAN_STATUS),
            walk(
                "vlanPortConfigTable port name",
                oids::vlan::tplink::VLAN_PORT_CONFIG_NAME,
            ),
            walk(
                "vlanPortConfigTable PVID",
                oids::vlan::tplink::VLAN_PORT_CONFIG_PVID,
            ),
            walk(
                "vlanTagPortMemberAdd",
                oids::vlan::tplink::VLAN_TAG_PORT_MEMBER_ADD,
            ),
        ],
    },
];

/// One timeout or limit the integration runs under.
#[derive(Debug, Clone, Serialize)]
pub struct SnmpLimit {
    pub name: &'static str,
    pub value: String,
    pub description: &'static str,
}

/// The SNMP reference fixture.
#[derive(Debug, Clone, Serialize)]
pub struct SnmpReference {
    pub versions: Vec<String>,
    pub v3_security_levels: Vec<&'static str>,
    pub v3_auth_protocols: Vec<&'static str>,
    pub v3_privacy_protocols: Vec<&'static str>,
    pub ports: &'static [u16],
    pub limits: Vec<SnmpLimit>,
    pub mibs: &'static [SnmpMib],
}

fn duration_label(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs >= 60 && secs.is_multiple_of(60) {
        format!("{}m", secs / 60)
    } else {
        format!("{secs}s")
    }
}

pub fn snmp_reference() -> SnmpReference {
    use strum::VariantNames;

    SnmpReference {
        versions: SnmpVersion::VARIANTS
            .iter()
            .map(|v| format!("SNMP{}", v.to_lowercase()))
            .collect(),
        v3_security_levels: SnmpV3SecurityLevel::options()
            .iter()
            .map(|o| o.label)
            .collect(),
        v3_auth_protocols: SnmpV3AuthProtocol::OPTIONS
            .iter()
            .map(|o| o.label)
            .collect(),
        v3_privacy_protocols: SnmpV3PrivProtocol::OPTIONS
            .iter()
            .map(|o| o.label)
            .collect(),
        ports: &SNMP_PORTS,
        limits: vec![
            SnmpLimit {
                name: "Probe timeout",
                value: duration_label(SNMP_PROBE_TIMEOUT),
                description: "Liveness probe per credential and port, used to pick a working \
                              credential before collection",
            },
            SnmpLimit {
                name: "Request timeout",
                value: duration_label(SNMP_TIMEOUT),
                description: "A single SNMP request",
            },
            SnmpLimit {
                name: "Session timeout",
                value: duration_label(SNMP_SESSION_TIMEOUT),
                description: "Establishing the UDP session",
            },
            SnmpLimit {
                name: "Query timeout",
                value: duration_label(SNMP_WALK_TIMEOUT),
                description: "Any single query, including a full walk of a table",
            },
            SnmpLimit {
                name: "Max entries",
                value: MAX_WALK_ENTRIES.to_string(),
                description: "Rows read from a single table walk",
            },
            SnmpLimit {
                name: "Integration timeout",
                value: duration_label(SnmpIntegration.timeout()),
                description: "A full SNMP run against one host, probe and all queries",
            },
        ],
        mibs: REQUESTED_MIBS,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::net::IpAddr;

    use super::*;
    use crate::daemon::discovery::integration::snmp::sim::devices;
    use crate::daemon::discovery::integration::snmp::sim::harness::{Recorded, Recording};
    use crate::daemon::discovery::integration::snmp::*;

    /// Every request the queries make of every simulated device, in the order `execute` runs
    /// them, with bridge and VLAN queries sent to the bridge context where the device has one.
    async fn requests_across_sim_devices() -> Vec<(Vec<u64>, SnmpRequest)> {
        let mut all: Vec<Recorded> = Vec::new();
        for device in devices::all() {
            let ip: IpAddr = device.ip.into();
            let mut session = Recording::new(device.agent());
            let mut context = device.context_agent().map(Recording::new);

            let _ = query_system_info(&mut session, ip).await;
            let _ = walk_if_table(&mut session, ip).await;
            let _ = query_lldp_neighbors(&mut session, ip).await;
            let _ = query_cdp_neighbors(&mut session, ip).await;
            let _ = query_lldp_local_ports(&mut session, ip).await;
            let _ = query_ip_addr_table(&mut session, ip).await;
            let _ = query_arp_table(&mut session, ip).await;
            let _ = query_entity_physical(&mut session, ip).await;

            let bridge = context.as_mut().unwrap_or(&mut session);
            let ports = query_bridge_port_mapping(bridge, ip)
                .await
                .unwrap_or_default();
            let _ = query_bridge_fdb(bridge, ip, &ports).await;
            let sys_object_id = device.system.sys_object_id.as_deref();
            let _ = query_vlan_table(bridge, ip, sys_object_id).await;
            let _ = query_port_vlan_membership(bridge, ip, &ports, sys_object_id).await;

            let _ = query_lldp_local(&mut session, ip).await;

            all.extend(session.requests);
            all.extend(context.into_iter().flat_map(|c| c.requests));
        }
        all.into_iter().map(|r| (r.oid, r.request)).collect()
    }

    fn catalog_objects() -> impl Iterator<Item = (&'static SnmpMib, &'static SnmpObject)> {
        REQUESTED_MIBS
            .iter()
            .flat_map(|mib| mib.objects.iter().map(move |object| (mib, object)))
    }

    /// The request was for this object: the object itself for a GET, anything under it for a walk,
    /// whose later pages start from the last row returned.
    fn covers(object: &SnmpObject, oid: &[u64], request: SnmpRequest) -> bool {
        let object_oid = oids::oid_parts(object.oid);
        object.request == request
            && match request {
                SnmpRequest::Get => oid == object_oid.as_slice(),
                SnmpRequest::Walk => oid.starts_with(&object_oid),
            }
    }

    #[tokio::test]
    async fn catalog_matches_what_the_queries_request() {
        let requests = requests_across_sim_devices().await;

        let uncatalogued: BTreeSet<String> = requests
            .iter()
            .filter(|(oid, request)| !catalog_objects().any(|(_, o)| covers(o, oid, *request)))
            .map(|(oid, request)| {
                let dotted: Vec<String> = oid.iter().map(u64::to_string).collect();
                format!("{request:?} {}", dotted.join("."))
            })
            .collect();
        assert!(
            uncatalogued.is_empty(),
            "SNMP requests missing from REQUESTED_MIBS, so missing from the docs: {uncatalogued:?}"
        );

        let unrequested: Vec<String> = catalog_objects()
            .filter(|(_, o)| {
                !requests
                    .iter()
                    .any(|(oid, request)| covers(o, oid, *request))
            })
            .map(|(mib, o)| format!("{}::{}", mib.name, o.name))
            .collect();
        assert!(
            unrequested.is_empty(),
            "REQUESTED_MIBS lists objects no query requests from any simulated device: \
             {unrequested:?}"
        );
    }
}
