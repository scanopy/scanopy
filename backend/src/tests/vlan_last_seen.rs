//! A VLAN a scan reports again reads as seen by that scan, against a real database.
//!
//! An unchanged VLAN is not rewritten by the upsert itself, so without the refresh its Last seen
//! stays at the scan that created it and the digest reports it stale while every scan still sees
//! it.

use chrono::Duration;
use uuid::Uuid;

use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::services::scan_context::ScanContext;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Storage;
use crate::server::vlans::handlers::VlanDiscoveryItem;
use crate::server::vlans::r#impl::base::Vlan;

use super::{organization, site, test_services};

fn servers() -> Vec<VlanDiscoveryItem> {
    vec![VlanDiscoveryItem {
        vlan_number: 20,
        name: "Servers".to_string(),
        name_source: AttributeSource::Authored(ClientProbe::UnifiController),
    }]
}

#[tokio::test]
async fn a_vlan_reported_again_unchanged_reads_as_seen_by_that_scan() {
    let (storage, services, _container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let site = site(&org.id);
    storage.sites.create(&site).await.unwrap();
    let vlans = &services.vlan_service;

    let first = vlans
        .upsert_discovery_batch(
            site.id,
            org.id,
            servers(),
            Some(&ScanContext::new(Uuid::new_v4())),
        )
        .await
        .unwrap();
    let id = first.vlans[0].id;

    // As if the first scan had run a day ago.
    let mut stored = storage.vlans.get_by_id(&id).await.unwrap().unwrap();
    stored.last_seen_at = crate::server::shared::clock::now() - Duration::days(1);
    storage.vlans.update(&mut stored).await.unwrap();

    let rescan = ScanContext::new(Uuid::new_v4());
    let second = vlans
        .upsert_discovery_batch(site.id, org.id, servers(), Some(&rescan))
        .await
        .unwrap();

    assert_eq!(second.vlans[0].id, id, "the rescan matches the same VLAN");
    let live = storage
        .vlans
        .get_all(StorableFilter::<Vlan>::new_from_uuid_column("site_id", &site.id).live())
        .await
        .unwrap();
    assert_eq!(live.len(), 1, "one live row for the number");
    assert_eq!(live[0].last_seen_at, rescan.scan_time);
}
