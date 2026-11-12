//! TPLINK-DOT1Q-VLAN-MIB: VLANs on TP-Link switches that serve no Q-BRIDGE VLAN group (GH #847).
//!
//! Two tables. `vlanPortConfigTable` is indexed by ifIndex and gives each port's name (`1/0/1`)
//! and PVID. `vlanConfigTable` is indexed by VLAN ID and gives the VLAN's name, its status, and
//! its member ports as range strings (`1/0/4-6,1/0/8`) rather than Q-BRIDGE's port bitmaps.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::IpAddr;

use snmp2::Value;
use tracing::debug;

use super::walk::walk_column;
use super::{
    PortVlanMembership, Shortfall, SnmpCollection, SnmpWalkTransport, VlanInfo, oids, value_to_i32,
    value_to_string, value_to_u16,
};
use crate::server::snmp::generated::extract_enterprise_number;

/// `dot1qVlanStatus` of a VLAN that is in service.
const VLAN_STATUS_ACTIVE: i32 = 1;

/// The most ports one range may name. A range wider than any switch has ports is a value the
/// daemon cannot trust, and expanding it would allocate a name per claimed port.
const MAX_PORTS_PER_RANGE: u32 = 1024;

/// Whether the agent is a TP-Link device, going by the enterprise number in its sysObjectID.
/// Only these agents are asked for TP-Link's tables.
pub fn is_tplink_agent(sys_object_id: Option<&str>) -> bool {
    sys_object_id.and_then(extract_enterprise_number) == Some(oids::vlan::tplink::ENTERPRISE)
}

/// One comma-separated item of a member-port string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TplinkPortItem {
    /// `1/0/7` or `1/0/1-3`: ports `first` to `last` inclusive, each named `prefix` + number.
    Ports {
        /// Everything through the last `/`: `1/0/`.
        prefix: String,
        first: u32,
        last: u32,
    },
    /// A link aggregation group. It has no row in the port table, so no ifIndex.
    Lag(String),
    /// An item that is neither.
    Unreadable(String),
}

/// A member-port string from `vlanConfigTable`, such as `1/0/1-3,1/0/7`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TplinkPortList(pub Vec<TplinkPortItem>);

impl From<&str> for TplinkPortItem {
    fn from(item: &str) -> Self {
        if item
            .get(..3)
            .is_some_and(|head| head.eq_ignore_ascii_case("lag"))
        {
            return Self::Lag(item.to_string());
        }
        let unreadable = || Self::Unreadable(item.to_string());
        let Some((prefix, range)) = item.rsplit_once('/') else {
            return unreadable();
        };
        let (first, last) = match range.split_once('-') {
            Some((first, last)) => (first.parse::<u32>(), last.parse::<u32>()),
            None => (range.parse::<u32>(), range.parse::<u32>()),
        };
        match (first, last) {
            (Ok(first), Ok(last))
                if !prefix.is_empty() && first <= last && last - first < MAX_PORTS_PER_RANGE =>
            {
                Self::Ports {
                    prefix: format!("{prefix}/"),
                    first,
                    last,
                }
            }
            _ => unreadable(),
        }
    }
}

impl From<&str> for TplinkPortList {
    fn from(list: &str) -> Self {
        Self(
            list.split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(TplinkPortItem::from)
                .collect(),
        )
    }
}

impl TplinkPortList {
    /// Every port the list names, as the port table spells it.
    pub fn port_names(&self) -> impl Iterator<Item = String> + '_ {
        self.0
            .iter()
            .filter_map(|item| match item {
                TplinkPortItem::Ports {
                    prefix,
                    first,
                    last,
                } => Some((prefix, *first..=*last)),
                TplinkPortItem::Lag(_) | TplinkPortItem::Unreadable(_) => None,
            })
            .flat_map(|(prefix, numbers)| numbers.map(move |number| format!("{prefix}{number}")))
    }

    /// The items that name no port: LAG entries and anything unreadable.
    pub fn without_a_port(&self) -> impl Iterator<Item = &str> {
        self.0.iter().filter_map(|item| match item {
            TplinkPortItem::Ports { .. } => None,
            TplinkPortItem::Lag(text) | TplinkPortItem::Unreadable(text) => Some(text.as_str()),
        })
    }
}

/// The index of a row in either table: the VLAN ID or the ifIndex, as the last sub-id.
fn row_index<N: TryFrom<u64>>(suffix: &[u64]) -> Option<N> {
    suffix.last().and_then(|index| N::try_from(*index).ok())
}

/// Active VLANs by ID, with their names.
pub(super) async fn walk_vlan_names<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
    shortfall: &mut Shortfall,
) -> Vec<VlanInfo> {
    let mut names: BTreeMap<u16, String> = BTreeMap::new();
    walk_column(
        session,
        ip,
        oids::vlan::tplink::DOT1Q_VLAN_DESCRIPTION,
        shortfall,
        |suffix, value| {
            if let Some(vlan_id) = row_index(suffix)
                && let Some(name) = value_to_string(value)
            {
                names.insert(vlan_id, name);
            }
        },
    )
    .await;
    if names.is_empty() {
        return Vec::new();
    }

    let mut active: HashSet<u16> = HashSet::new();
    walk_column(
        session,
        ip,
        oids::vlan::tplink::DOT1Q_VLAN_STATUS,
        shortfall,
        |suffix, value| {
            if let Some(vlan_id) = row_index(suffix)
                && value_to_i32(value) == Some(VLAN_STATUS_ACTIVE)
            {
                active.insert(vlan_id);
            }
        },
    )
    .await;

    names
        .into_iter()
        .filter(|(vlan_id, _)| active.contains(vlan_id))
        .map(|(vlan_id, name)| VlanInfo { vlan_id, name })
        .collect()
}

/// Each port's native and tagged VLANs, keyed by ifIndex.
///
/// The port table's name column is the only source of a port's ifIndex. A member the VLAN table
/// names that has no row there (a LAG, or a port name the port table does not list) is left out
/// and counted in the collection's `discarded`.
pub(super) async fn query_port_vlan_membership<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> SnmpCollection<Vec<PortVlanMembership>> {
    let mut shortfall = Shortfall::default();

    let mut if_index_by_name: HashMap<String, i32> = HashMap::new();
    walk_column(
        session,
        ip,
        oids::vlan::tplink::VLAN_PORT_CONFIG_NAME,
        &mut shortfall,
        |suffix, value| {
            if let Some(if_index) = row_index(suffix)
                && let Some(name) = value_to_string(value)
            {
                if_index_by_name.insert(name, if_index);
            }
        },
    )
    .await;

    let mut members: BTreeMap<i32, PortVlanMembership> = BTreeMap::new();
    fn member(
        members: &mut BTreeMap<i32, PortVlanMembership>,
        if_index: i32,
    ) -> &mut PortVlanMembership {
        members.entry(if_index).or_insert(PortVlanMembership {
            if_index,
            native_vlan: None,
            tagged_vlans: Vec::new(),
        })
    }

    walk_column(
        session,
        ip,
        oids::vlan::tplink::VLAN_PORT_CONFIG_PVID,
        &mut shortfall,
        |suffix, value| {
            if let Some(if_index) = row_index(suffix)
                && let Some(vlan_id) = value_to_u16(value)
            {
                member(&mut members, if_index).native_vlan = Some(vlan_id);
            }
        },
    )
    .await;

    let mut tagged: Vec<(u16, TplinkPortList)> = Vec::new();
    walk_column(
        session,
        ip,
        oids::vlan::tplink::VLAN_TAG_PORT_MEMBER_ADD,
        &mut shortfall,
        |suffix, value: &Value| {
            if let Some(vlan_id) = row_index(suffix)
                && let Some(ports) = value_to_string(value)
            {
                tagged.push((vlan_id, TplinkPortList::from(ports.as_str())));
            }
        },
    )
    .await;

    // By name, so a LAG tagged in four VLANs is one unresolved member and not four.
    let mut unresolved: HashSet<String> = HashSet::new();
    for (vlan_id, ports) in &tagged {
        unresolved.extend(ports.without_a_port().map(str::to_string));
        for name in ports.port_names() {
            match if_index_by_name.get(&name) {
                Some(if_index) => member(&mut members, *if_index).tagged_vlans.push(*vlan_id),
                None => {
                    unresolved.insert(name);
                }
            }
        }
    }

    debug!(
        ip = %ip,
        ports = members.len(),
        unresolved = unresolved.len(),
        "TP-Link VLAN membership read"
    );

    let mut collection = SnmpCollection::from_walk(members.into_values().collect(), shortfall);
    collection.discarded = unresolved.len();
    collection
}
