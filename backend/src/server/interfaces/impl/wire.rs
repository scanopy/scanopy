//! The interface shape a daemon submits, as distinct from the interface the server models.
//!
//! GH #701 replaced twelve scalar LLDP/CDP columns on `interfaces` with the
//! `interface_neighbor_candidates` table, so a current daemon submits
//! `neighbor_candidates: [...]` where a pre-#701 daemon submits `lldp_chassis_id`,
//! `cdp_device_id` and eight siblings flat on the interface object. Both have to keep working
//! for a whole deprecation window.
//!
//! Nothing in this codebase sets `deny_unknown_fields`, so removing the old fields outright
//! would let an old daemon's submission deserialize *successfully* with its neighbour data
//! silently dropped — worse than an error. Translating the old shape is therefore mandatory;
//! the only question is where it lives. It lives here, at the boundary, and not on `Interface`:
//! `Interface` describes an interface, and reading a superseded wire format is a different job
//! with a different lifetime.
//!
//! This module is written to be deleted. When the enforced daemon floor rises above
//! [`last_legacy_neighbor_wire`](crate::server::daemons::r#impl::version::last_legacy_neighbor_wire),
//! `legacy_neighbor_wire_shim_still_needed` fails and says so.

use serde::Deserialize;

use crate::server::interface_neighbors::r#impl::base::InterfaceNeighborEvidence;
use crate::server::interfaces::r#impl::base::Interface;

/// One interface as a daemon submits it: the domain entity, plus the superseded scalar
/// LLDP/CDP shape a pre-#701 daemon still sends alongside it.
///
/// Deserialize-only, and deliberately not `Serialize`: the server never emits this shape, and a
/// current daemon never does either. Outgoing payloads serialize [`Interface`] directly.
///
/// Both flattens read from the same JSON object. They cannot collide, because #701 removed the
/// scalar fields from `InterfaceBase` in the same change that added `neighbor_candidates` — so a
/// key is claimed by exactly one of the two.
#[derive(Debug, Clone, Deserialize)]
pub struct DiscoveryInterface {
    #[serde(flatten)]
    interface: Interface,
    /// The pre-#701 scalar shape. Reuses [`InterfaceNeighborEvidence`] rather than redeclaring
    /// ten fields, because that type *is* the old scalar shape — #701 kept the field names and
    /// only changed their cardinality.
    ///
    /// Empty for every current daemon, and also for an old daemon reporting a port that heard
    /// no neighbour, which is why its emptiness is not a reliable "this daemon is current"
    /// signal. See [`Self::submitted_legacy_neighbor_evidence`].
    #[serde(flatten)]
    legacy_neighbor_evidence: InterfaceNeighborEvidence,
}

impl DiscoveryInterface {
    /// Whether this submission actually carried neighbour data in the superseded shape.
    ///
    /// True only when an old daemon heard something on the port. An old daemon reporting a port
    /// with no LLDP/CDP is indistinguishable here from a current one — the unambiguous signal
    /// would be the absence of the `neighbor_candidates` key, which this type cannot observe
    /// because the flattened `Interface` consumes it. Kept deliberately: this drives an
    /// operator-facing warning, and firing it says "we translated something", which is true.
    pub fn submitted_legacy_neighbor_evidence(&self) -> bool {
        self.legacy_neighbor_evidence != InterfaceNeighborEvidence::default()
    }
}

impl From<DiscoveryInterface> for Interface {
    /// Fold the superseded scalar shape into `neighbor_candidates` as one more candidate, so a
    /// pre-#701 submission produces exactly the row a current daemon's candidate entry would.
    ///
    /// The old shape's `neighbor` and `neighbor_seen_at` keys are left unmatched and ignored,
    /// as they were before this type existed: resolution is recomputed server-side from
    /// candidates, and `interface_neighbor_candidates` has no `neighbor_seen_at` column, so
    /// there is nowhere to put either one.
    fn from(wire: DiscoveryInterface) -> Self {
        let DiscoveryInterface {
            mut interface,
            legacy_neighbor_evidence,
        } = wire;

        if legacy_neighbor_evidence != InterfaceNeighborEvidence::default() {
            interface
                .base
                .neighbor_candidates
                .push(legacy_neighbor_evidence);
        }

        interface
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::lldp::{LldpChassisId, LldpPortId};

    /// The old wire shape, as captured from a real v0.17.14 daemon: the ten scalars flat on the
    /// interface object, `neighbor`/`neighbor_seen_at` alongside them, and no
    /// `neighbor_candidates` key at all.
    fn pre_701_interface() -> serde_json::Value {
        serde_json::json!({
            "host_id": "3fd2970b-7217-4821-8bd7-6985174b6a22",
            "network_id": "d558305a-dc8b-423b-88d6-3cb138d8c51d",
            "if_index": 3,
            "if_name": "GigabitEthernet0/3",
            "if_descr": "GigabitEthernet0/3",
            "lldp_chassis_id": { "subtype": "MacAddress", "value": "00:1a:2b:00:11:00" },
            "lldp_port_id": { "subtype": "InterfaceName", "value": "Gi0/1" },
            "lldp_sys_name": "core-sw-01",
            "lldp_port_desc": "uplink to access",
            "lldp_sys_desc": "Cisco IOS",
            "lldp_mgmt_addr": "192.168.1.1",
            "cdp_device_id": null,
            "cdp_port_id": null,
            "cdp_platform": null,
            "cdp_address": null,
            "neighbor": null,
            "neighbor_seen_at": null,
        })
    }

    #[test]
    fn pre_701_scalars_become_one_candidate() {
        // The guarantee the whole shim exists for: an old daemon's neighbour data must reach
        // `neighbor_candidates`, not vanish into a successful-but-empty deserialization.
        let wire: DiscoveryInterface =
            serde_json::from_value(pre_701_interface()).expect("old shape deserializes");
        assert!(wire.submitted_legacy_neighbor_evidence());

        let interface: Interface = wire.into();
        let candidates = &interface.base.neighbor_candidates;
        assert_eq!(candidates.len(), 1, "expected exactly one candidate");

        let candidate = &candidates[0];
        assert_eq!(
            candidate.lldp_chassis_id,
            Some(LldpChassisId::MacAddress("00:1a:2b:00:11:00".into()))
        );
        assert_eq!(
            candidate.lldp_port_id,
            Some(LldpPortId::InterfaceName("Gi0/1".into()))
        );
        assert_eq!(candidate.lldp_sys_name.as_deref(), Some("core-sw-01"));
        assert_eq!(
            candidate.lldp_port_desc.as_deref(),
            Some("uplink to access")
        );
        assert_eq!(candidate.lldp_sys_desc.as_deref(), Some("Cisco IOS"));
        assert_eq!(
            candidate.lldp_mgmt_addr,
            Some("192.168.1.1".parse().expect("valid ip"))
        );

        // The ifTable fields still land where they always did.
        assert_eq!(interface.base.if_index, Some(3));
        assert_eq!(
            interface.base.if_name.as_deref(),
            Some("GigabitEthernet0/3")
        );
    }

    #[test]
    fn an_old_daemon_reporting_no_neighbour_yields_no_candidate() {
        // What the entire v0.17.14 installed base actually sends on most ports: the old keys
        // present, every value null. Translating that into a candidate row would invent an
        // adjacency the device never advertised.
        let mut json = pre_701_interface();
        for key in [
            "lldp_chassis_id",
            "lldp_port_id",
            "lldp_sys_name",
            "lldp_port_desc",
            "lldp_sys_desc",
            "lldp_mgmt_addr",
        ] {
            json[key] = serde_json::Value::Null;
        }

        let wire: DiscoveryInterface =
            serde_json::from_value(json).expect("all-null old shape deserializes");
        assert!(!wire.submitted_legacy_neighbor_evidence());

        let interface: Interface = wire.into();
        assert!(interface.base.neighbor_candidates.is_empty());
    }

    #[test]
    fn a_current_daemons_candidates_pass_through_untouched() {
        let submitted = serde_json::json!({
            "host_id": "3fd2970b-7217-4821-8bd7-6985174b6a22",
            "network_id": "d558305a-dc8b-423b-88d6-3cb138d8c51d",
            "if_index": 3,
            "if_name": "GigabitEthernet0/3",
            "neighbor_candidates": [
                { "lldp_chassis_id": { "subtype": "MacAddress", "value": "00:1a:2b:00:11:00" } },
                { "cdp_device_id": "core-sw-01" },
            ],
        });

        let wire: DiscoveryInterface =
            serde_json::from_value(submitted).expect("current shape deserializes");
        assert!(!wire.submitted_legacy_neighbor_evidence());

        let interface: Interface = wire.into();
        // Two in, two out — the shim must not append a phantom eleventh-field candidate.
        assert_eq!(interface.base.neighbor_candidates.len(), 2);
        assert_eq!(
            interface.base.neighbor_candidates[1]
                .cdp_device_id
                .as_deref(),
            Some("core-sw-01")
        );
    }
}
