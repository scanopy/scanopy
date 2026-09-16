//! `InterfaceNeighborService::replace_candidates_from_discovery` against a real database.
//!
//! GH #701 moved neighbour evidence off `interfaces` and into `interface_neighbor_candidates`,
//! and carried forward the old per-host `collected: InterfaceDataComplete` veto unchanged. That
//! veto was written against the old schema, where "preserve the stored value" meant "leave an
//! always-present scalar column alone" — moving to a disposable candidate table that starts empty
//! turned the same veto into "a device with any malformed row anywhere can never establish its
//! first row," because every row daemon-side validation already lets through was still being
//! discarded a second time by a device-wide flag. These tests pin the fix: acceptance is per row
//! (already proven by the daemon), and `collected` only governs what happens to a row this scan
//! said nothing about at all.

use uuid::Uuid;

use crate::server::{
    hosts::r#impl::{base::Host, name::HostNameSources},
    interface_neighbors::{
        r#impl::base::InterfaceNeighborCandidate, service::InterfaceNeighborService,
    },
    interfaces::r#impl::base::{
        IfAdminStatus, IfOperStatus, Interface, InterfaceBase, InterfaceDataComplete, if_type,
    },
    lldp::LldpChassisId,
    shared::storage::traits::Storage,
};

use super::{host, network, organization, subnet, test_services};
use crate::server::interface_neighbors::r#impl::base::InterfaceNeighborEvidence;

/// Everything a candidate-replacement test needs: a network with a host and interfaces in it, and
/// the service under test.
struct Lab {
    interface_neighbor_service: std::sync::Arc<InterfaceNeighborService>,
    storage: crate::server::shared::storage::factory::StorageFactory,
    network_id: Uuid,
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

        Self {
            interface_neighbor_service: services.interface_neighbor_service.clone(),
            network_id: network.id,
            storage,
            _container,
        }
    }

    async fn host(&self, name: &str) -> Host {
        let mut h = host(&self.network_id);
        h.base.name = crate::server::hosts::r#impl::name::HostName::manual(name.to_string());
        self.storage.hosts.create(&h).await.unwrap();
        h
    }

    async fn port(&self, host_id: Uuid, if_index: i32, descr: &str) -> Interface {
        let entry = Interface::new(InterfaceBase {
            host_id,
            network_id: self.network_id,
            if_index: Some(if_index),
            if_descr: Some(descr.to_string()),
            if_type: Some(if_type::ETHERNET_CSMA_CD),
            admin_status: Some(IfAdminStatus::Up),
            oper_status: Some(IfOperStatus::Up),
            ..Default::default()
        });
        self.storage.interfaces.create(&entry).await.unwrap();
        entry
    }

    async fn candidates(&self, interface_id: Uuid) -> Vec<InterfaceNeighborCandidate> {
        self.interface_neighbor_service
            .candidates_for_interface(&interface_id)
            .await
            .unwrap()
    }
}

fn lldp_evidence(chassis_mac: &str) -> InterfaceNeighborEvidence {
    InterfaceNeighborEvidence {
        lldp_chassis_id: Some(LldpChassisId::MacAddress(chassis_mac.into())),
        ..Default::default()
    }
}

fn cdp_evidence(device_id: &str) -> InterfaceNeighborEvidence {
    InterfaceNeighborEvidence {
        cdp_device_id: Some(device_id.to_string()),
        ..Default::default()
    }
}

fn complete() -> InterfaceDataComplete {
    InterfaceDataComplete {
        lldp: true,
        cdp: true,
        fdb: true,
        vlan_membership: true,
    }
}

/// The reported bug: a well-formed candidate on one interface must persist even though the same
/// scan marked the whole device's LLDP walk partial because of an unrelated malformed row on a
/// different port. Every row `query_lldp_neighbors_for` lets through already has a valid chassis
/// ID — a device-wide `collected.lldp = false` must not veto it a second time.
#[tokio::test]
async fn a_well_formed_candidate_survives_a_malformed_sibling_on_the_same_walk() {
    let lab = Lab::new().await;
    let switch = lab.host("switch-partial-lldp").await;
    let port = lab.port(switch.id, 1, "Gi0/1").await;

    lab.interface_neighbor_service
        .replace_candidates_from_discovery(
            lab.network_id,
            port.id,
            vec![lldp_evidence("00:1a:2b:00:10:00")],
            InterfaceDataComplete {
                lldp: false,
                ..complete()
            },
        )
        .await
        .unwrap();

    let candidates = lab.candidates(port.id).await;
    assert_eq!(
        candidates.len(),
        1,
        "a well-formed row from this scan must persist despite the host-wide partial flag"
    );
    assert_eq!(
        candidates[0].base.evidence.lldp_chassis_id,
        Some(LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()))
    );
}

/// The property `b4afa6cbf`/GH #701 exist to protect: a walk that read nothing for a group must
/// not refresh — or discard — existing evidence's freshness. Must hold before and after the fix.
#[tokio::test]
async fn a_walk_that_read_nothing_does_not_refresh_or_delete_existing_evidence() {
    let lab = Lab::new().await;
    let switch = lab.host("switch-total-failure").await;
    let port = lab.port(switch.id, 1, "Gi0/1").await;

    lab.interface_neighbor_service
        .replace_candidates_from_discovery(
            lab.network_id,
            port.id,
            vec![lldp_evidence("00:1a:2b:00:10:00")],
            complete(),
        )
        .await
        .unwrap();
    let original = lab.candidates(port.id).await;
    assert_eq!(original.len(), 1);
    let original_created_at = original[0].created_at;

    // A later scan reads nothing at all for LLDP (timeout, revoked community string).
    lab.interface_neighbor_service
        .replace_candidates_from_discovery(
            lab.network_id,
            port.id,
            vec![],
            InterfaceDataComplete {
                lldp: false,
                ..complete()
            },
        )
        .await
        .unwrap();

    let after = lab.candidates(port.id).await;
    assert_eq!(
        after.len(),
        1,
        "existing evidence must not be deleted by a walk that read nothing"
    );
    assert_eq!(
        after[0].created_at, original_created_at,
        "existing evidence's freshness must not be refreshed by a walk that read nothing"
    );
}

/// The duplicate-revival risk the per-interface scoping guards against: an interface that *did*
/// get a fresh candidate this scan must not also have a stale, different candidate from an
/// earlier scan revived beside it just because the host-wide walk was marked partial.
#[tokio::test]
async fn fresh_evidence_does_not_resurrect_stale_evidence_for_the_same_interface() {
    let lab = Lab::new().await;
    let switch = lab.host("switch-recovered-port").await;
    let port = lab.port(switch.id, 1, "Gi0/1").await;

    lab.interface_neighbor_service
        .replace_candidates_from_discovery(
            lab.network_id,
            port.id,
            vec![lldp_evidence("00:1a:2b:00:10:00")],
            complete(),
        )
        .await
        .unwrap();

    // This scan: the host-wide LLDP walk is partial (some other port's row was malformed), but
    // *this* port reports a fresh, different neighbour.
    lab.interface_neighbor_service
        .replace_candidates_from_discovery(
            lab.network_id,
            port.id,
            vec![lldp_evidence("00:1a:2b:00:20:00")],
            InterfaceDataComplete {
                lldp: false,
                ..complete()
            },
        )
        .await
        .unwrap();

    let candidates = lab.candidates(port.id).await;
    assert_eq!(
        candidates.len(),
        1,
        "a port that reported fresh evidence must not also carry forward its old neighbour"
    );
    assert_eq!(
        candidates[0].base.evidence.lldp_chassis_id,
        Some(LldpChassisId::MacAddress("00:1a:2b:00:20:00".into())),
        "the fresh neighbour must be the one that survives"
    );
}

/// Same shape as the LLDP fix, for the parallel CDP branch — a copy/paste slip in the rewrite
/// could fix one protocol and leave the other vetoed.
#[tokio::test]
async fn the_same_guarantees_hold_for_cdp() {
    let lab = Lab::new().await;
    let switch = lab.host("switch-partial-cdp").await;
    let port = lab.port(switch.id, 1, "Gi0/1").await;

    lab.interface_neighbor_service
        .replace_candidates_from_discovery(
            lab.network_id,
            port.id,
            vec![cdp_evidence("core-switch-01")],
            InterfaceDataComplete {
                cdp: false,
                ..complete()
            },
        )
        .await
        .unwrap();

    let candidates = lab.candidates(port.id).await;
    assert_eq!(
        candidates.len(),
        1,
        "a well-formed CDP row from this scan must persist despite the host-wide partial flag"
    );
    assert_eq!(
        candidates[0].base.evidence.cdp_device_id,
        Some("core-switch-01".to_string())
    );
}
