use super::proto::gnmi::{Notification, PathElem, TypedValue, typed_value};
use super::{Collection, LldpModelProfile};
use crate::server::interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, if_type};
use crate::server::lldp::{LldpChassisId, LldpPortId, canonical_mac};
use crate::server::snmp::generated::get_if_type_number;

/// A flattened update: the full path (prefix + update path, JSON keys appended) and the
/// leaf's value as text.
pub(super) struct Leaf {
    pub(super) elems: Vec<PathElem>,
    pub(super) value: String,
}

/// Strip the YANG module prefix json_ietf puts on names: `openconfig-interfaces:ifindex`.
pub(super) fn unqualified(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn scalar_to_string(v: &typed_value::Value) -> Option<String> {
    Some(match v {
        typed_value::Value::StringVal(s) | typed_value::Value::AsciiVal(s) => s.clone(),
        typed_value::Value::IntVal(i) => i.to_string(),
        typed_value::Value::UintVal(u) => u.to_string(),
        typed_value::Value::BoolVal(b) => b.to_string(),
        _ => return None,
    })
}

/// Flatten one update to leaves. PROTO encoding gives one typed leaf per update; JSON
/// encodings give a blob rooted at the update path, whose objects become path elements
/// (list entries keyed by their `name`/`id` member, the way both models key their lists).
/// `None` when a JSON blob does not parse: the update carried data that was lost, which is not
/// the same as an update with nothing in it.
fn flatten_update(prefix: &[PathElem], path: &[PathElem], val: &TypedValue) -> Option<Vec<Leaf>> {
    let mut elems: Vec<PathElem> = prefix.iter().chain(path.iter()).cloned().collect();
    let Some(value) = val.value.as_ref() else {
        return Some(vec![]);
    };
    match value {
        typed_value::Value::JsonIetfVal(bytes) | typed_value::Value::JsonVal(bytes) => {
            let json = serde_json::from_slice::<serde_json::Value>(bytes).ok()?;
            let mut out = Vec::new();
            flatten_json(&mut elems, &json, &mut out);
            Some(out)
        }
        other => Some(
            scalar_to_string(other)
                .map(|value| vec![Leaf { elems, value }])
                .unwrap_or_default(),
        ),
    }
}

fn flatten_json(elems: &mut Vec<PathElem>, json: &serde_json::Value, out: &mut Vec<Leaf>) {
    match json {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                elems.push(PathElem {
                    name: unqualified(k).to_string(),
                    ..Default::default()
                });
                flatten_json(elems, v, out);
                elems.pop();
            }
        }
        serde_json::Value::Array(items) => {
            // A list: each entry re-uses the enclosing element's name and takes its key from
            // the entry itself.
            let Some(list) = elems.pop() else { return };
            for item in items {
                let mut entry = list.clone();
                if let Some(obj) = item.as_object() {
                    for key in ["name", "id"] {
                        if let Some(v) = obj.get(key).and_then(json_scalar) {
                            entry.key.insert(key.to_string(), v);
                            break;
                        }
                    }
                }
                elems.push(entry);
                flatten_json(elems, item, out);
                elems.pop();
            }
            elems.push(list);
        }
        scalar => {
            if let Some(value) = json_scalar(scalar) {
                out.push(Leaf {
                    elems: elems.clone(),
                    value,
                });
            }
        }
    }
}

fn json_scalar(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Fold one notification's updates into the collection. Returns `false` when an update did not
/// parse, so the caller knows the subtree was not read in full.
pub(crate) fn absorb_notification(
    coll: &mut Collection,
    profile: &LldpModelProfile,
    notification: &Notification,
) -> bool {
    let prefix = notification.prefix.as_ref().map(|p| p.elem.as_slice());
    let mut parsed = true;
    for update in &notification.update {
        let Some(val) = update.val.as_ref() else {
            continue;
        };
        let path = update.path.as_ref().map(|p| p.elem.as_slice());
        match flatten_update(prefix.unwrap_or(&[]), path.unwrap_or(&[]), val) {
            Some(leaves) => {
                for leaf in leaves {
                    absorb_leaf(coll, profile, &leaf);
                }
            }
            None => parsed = false,
        }
    }
    parsed
}

/// The path element names to match on, with the device's LLDP model folded onto the openconfig
/// shape it mirrors: the profile's root stripped, its state container read as `state`. For
/// [`OPENCONFIG_LLDP`] both are no-ops and every name passes through untouched.
///
/// The fold is applied only to paths that ARE this model's LLDP tree. Applied to every leaf
/// instead, the state-container rewrite clobbers the real `state` leaves of any device that has
/// its own meaning for the name — last writer winning, silently — and `/interfaces/…/state` is
/// subscribed on exactly the devices whose profile renames the container.
pub(super) fn normalised_names<'a>(profile: &LldpModelProfile, leaf: &'a Leaf) -> Vec<&'a str> {
    let mut names: Vec<&str> = leaf.elems.iter().map(|e| unqualified(&e.name)).collect();
    if !names.starts_with(profile.root) || names.get(profile.root.len()) != Some(&"lldp") {
        return names;
    }
    names.drain(..profile.root.len());
    for name in names.iter_mut() {
        if *name == profile.state_container {
            *name = "state";
        }
    }
    names
}

fn absorb_leaf(coll: &mut Collection, profile: &LldpModelProfile, leaf: &Leaf) {
    let names = normalised_names(profile, leaf);
    let Some((&leaf_name, containers)) = names.split_last() else {
        return;
    };
    let key_of = |elem: &str, key: &str| {
        leaf.elems
            .iter()
            .find(|e| unqualified(&e.name) == elem)
            .and_then(|e| e.key.get(key))
            .cloned()
    };
    let value = leaf.value.clone();
    match containers.first().copied() {
        Some("interfaces") => {
            // Subinterfaces carry their own `ifindex`; they are not rows here (an ifTable lists
            // them on some devices and not others, and the parent's identity is what LLDP and
            // the L2 view need).
            if containers.contains(&"subinterface") {
                return;
            }
            let Some(name) = key_of("interface", "name") else {
                return;
            };
            let entry = coll.interfaces.entry(name).or_default();
            match (containers, leaf_name) {
                ([.., "state"], "ifindex") => entry.ifindex = value.parse().ok(),
                ([.., "state"], "type") => entry.if_type = Some(value),
                ([.., "state"], "description") => entry.description = Some(value),
                ([.., "state"], "admin-status") => entry.admin_status = Some(value),
                ([.., "state"], "oper-status") => entry.oper_status = Some(value),
                ([.., "ethernet", "state"], "mac-address") => entry.mac_address = Some(value),
                // The configured speed when there is one, else what autoneg settled on.
                ([.., "ethernet", "state"], "port-speed") => entry.port_speed = Some(value),
                ([.., "ethernet", "state"], "negotiated-port-speed") => {
                    entry.port_speed.get_or_insert(value);
                }
                _ => {}
            }
        }
        Some("lldp") => match (key_of("interface", "name"), key_of("neighbor", "id")) {
            (Some(ifname), Some(nbr)) => {
                let entry = coll.neighbors.entry((ifname, nbr)).or_default();
                match leaf_name {
                    "chassis-id" => entry.chassis_id = Some(value),
                    "chassis-id-type" => entry.chassis_id_type = Some(value),
                    "port-id" => entry.port_id = Some(value),
                    "port-id-type" => entry.port_id_type = Some(value),
                    "port-description" => entry.port_description = Some(value),
                    "system-name" => entry.system_name = Some(value),
                    "system-description" => entry.system_description = Some(value),
                    "management-address" => entry.management_address = Some(value),
                    _ => {}
                }
            }
            // `/lldp/state` leaves carry the device's own identity.
            (None, None) => match leaf_name {
                "chassis-id" => coll.local_chassis_id = Some(value),
                "chassis-id-type" => coll.local_chassis_id_type = Some(value),
                _ => {}
            },
            _ => {}
        },
        _ => {}
    }
}

/// `iana-if-type` identity → IF-MIB ifType number: strip the YANG module prefix
/// (`iana-if-type:`, `ianaift:`), look the enumerator name up in the generated IANA registry
/// table (the identity names are literally the IANAifType labels). Vendor-private identities
/// (DNOS `irb`, `mgmt-ncx-member`) and anything else the registry lacks fall to `other(1)` —
/// the row is kept, never dropped. Not the `if_type` constants: those name the VLAN entries
/// one off from IANA (`L2_VLAN` = 136 where IANA's l2vlan is 135).
pub(crate) fn if_type_from_identity(identity: &str) -> i32 {
    get_if_type_number(unqualified(identity)).unwrap_or(if_type::OTHER)
}

/// `openconfig-if-ethernet` `ETHERNET_SPEED` identity (`SPEED_10GB`, `SPEED_2500MB`) → bits
/// per second. `SPEED_UNKNOWN` and anything unrecognised are "no speed", never a guess.
pub(crate) fn speed_from_identity(identity: &str) -> Option<i64> {
    let s = unqualified(identity).strip_prefix("SPEED_")?;
    let digits = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let n: i64 = digits.parse().ok()?;
    let per: i64 = match &s[digits.len()..] {
        "MB" => 1_000_000,
        "GB" => 1_000_000_000,
        "TB" => 1_000_000_000_000,
        _ => return None,
    };
    Some(n * per)
}

/// `openconfig-interfaces` `admin-status` enumeration → ifAdminStatus. An absent leaf is `None`:
/// the device made no claim, and recording one would be ours.
pub(super) fn admin_status(v: Option<&str>) -> Option<IfAdminStatus> {
    v.map(|v| {
        IfAdminStatus::from(match v {
            "DOWN" => 2,
            "TESTING" => 3,
            _ => 1,
        })
    })
}

/// `openconfig-interfaces` `oper-status` enumeration → ifOperStatus. Same enumerators as
/// IF-MIB, same order; an enumerator IF-MIB lacks is the device's `unknown(4)`, an absent leaf
/// is `None`.
pub(super) fn oper_status(v: Option<&str>) -> Option<IfOperStatus> {
    v.map(|v| {
        IfOperStatus::from(match v {
            "UP" => 1,
            "DOWN" => 2,
            "TESTING" => 3,
            "DORMANT" => 5,
            "NOT_PRESENT" => 6,
            "LOWER_LAYER_DOWN" => 7,
            _ => 4,
        })
    })
}

/// Map an `openconfig-lldp-types` identity (`openconfig-lldp-types:MAC_ADDRESS`) plus value
/// onto the 802.1AB chassis subtype. Unknown or absent types fall back to
/// [`LldpChassisId::from_identifier_str`].
pub(super) fn map_chassis(id: &str, id_type: Option<&str>) -> Option<LldpChassisId> {
    match id_type.map(unqualified) {
        Some("MAC_ADDRESS") => canonical_mac(id).map(LldpChassisId::MacAddress),
        Some("INTERFACE_NAME") => Some(LldpChassisId::InterfaceName(id.to_string())),
        Some("INTERFACE_ALIAS") => Some(LldpChassisId::InterfaceAlias(id.to_string())),
        Some("NETWORK_ADDRESS") => id.parse().ok().map(LldpChassisId::NetworkAddress),
        Some("LOCAL") => Some(LldpChassisId::LocallyAssigned(id.to_string())),
        Some("CHASSIS_COMPONENT") => Some(LldpChassisId::ChassisComponent(id.to_string())),
        Some("PORT_COMPONENT") => Some(LldpChassisId::PortComponent(id.to_string())),
        _ => Some(LldpChassisId::from_identifier_str(id)),
    }
}

pub(super) fn map_port(id: &str, id_type: Option<&str>) -> Option<LldpPortId> {
    match id_type.map(unqualified) {
        Some("MAC_ADDRESS") => canonical_mac(id).map(LldpPortId::MacAddress),
        Some("INTERFACE_NAME") => Some(LldpPortId::InterfaceName(id.to_string())),
        Some("INTERFACE_ALIAS") => Some(LldpPortId::InterfaceAlias(id.to_string())),
        Some("NETWORK_ADDRESS") => id.parse().ok().map(LldpPortId::NetworkAddress),
        Some("LOCAL") => Some(LldpPortId::LocallyAssigned(id.to_string())),
        Some("PORT_COMPONENT") => Some(LldpPortId::PortComponent(id.to_string())),
        Some("AGENT_CIRCUIT_ID") => Some(LldpPortId::AgentCircuitId(id.to_string())),
        _ => Some(LldpPortId::from_identifier_str(id)),
    }
}
