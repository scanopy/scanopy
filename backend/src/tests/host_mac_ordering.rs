//! Sorting the Hosts tab by MAC address (GH #569), through the real paginated query.
//!
//! A host has no MAC of its own: it sorts by the lowest MAC across its live IP addresses and
//! interfaces, and a host with none sorts last in both directions. The list is paginated, so the
//! order is only right if it holds across page boundaries, which is why every case walks pages.

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::server::hosts::handlers::HostOrderField;
use crate::server::hosts::r#impl::base::Host;
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase};
use crate::server::ip_addresses::r#impl::base::{IPAddress, MacEvidence, MacEvidenceValue};
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::handlers::ordering::apply_ordering;
use crate::server::shared::handlers::query::OrderDirection;
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::storage::factory::StorageFactory;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Storage;

use super::{host, ip_address, network, organization, subnet, test_services};

const PAGE_SIZE: u32 = 2;

fn mac(m: &str) -> Option<MacEvidence> {
    Some(MacEvidence::new(
        MacEvidenceValue(m.parse().unwrap()),
        AttributeSource::ArpReply,
    ))
}

struct Fixture {
    storage: StorageFactory,
    network_id: Uuid,
    subnet_id: Uuid,
}

impl Fixture {
    async fn host(&self) -> Uuid {
        let h = host(&self.network_id);
        self.storage.hosts.create(&h).await.unwrap();
        h.id
    }

    fn ip_address(&self, host_id: Uuid, m: Option<&str>) -> IPAddress {
        let mut ip = ip_address(&self.network_id, &self.subnet_id);
        ip.base.host_id = host_id;
        ip.base.mac_address = m.and_then(mac);
        ip
    }

    async fn live_ip(&self, host_id: Uuid, m: Option<&str>) {
        let ip = self.ip_address(host_id, m);
        self.storage.ip_addresses.create(&ip).await.unwrap();
    }

    /// A superseded SCD2 row: the host held this MAC once and no longer does.
    async fn closed_ip(&self, host_id: Uuid, m: &str) {
        let mut ip = self.ip_address(host_id, Some(m));
        ip.valid_to = Some(Utc::now() - Duration::hours(1));
        self.storage.ip_addresses.create(&ip).await.unwrap();
    }

    async fn interface(&self, host_id: Uuid, m: &str) {
        let entry = Interface::new(InterfaceBase {
            host_id,
            network_id: self.network_id,
            mac_address: mac(m),
            ..Default::default()
        });
        self.storage.interfaces.create(&entry).await.unwrap();
    }

    /// Every host id in list order, one page at a time, plus the total count each page reported.
    async fn walk(&self, services: &ServiceFactory, dir: OrderDirection) -> (Vec<Uuid>, Vec<u64>) {
        let mut ids = Vec::new();
        let mut totals = Vec::new();
        let mut offset = 0;
        loop {
            let filter = StorableFilter::<Host>::new_from_network_ids(&[self.network_id])
                .limit(PAGE_SIZE)
                .offset(offset);
            let (filter, order_by) = apply_ordering(
                None,
                Some(HostOrderField::MacAddress),
                Some(dir),
                filter,
                "hosts.created_at ASC",
            );
            let page = services
                .host_service
                .get_all_host_responses_paginated(filter, &order_by, None, false)
                .await
                .unwrap();
            if page.items.is_empty() {
                break;
            }
            totals.push(page.total_count);
            ids.extend(page.items.iter().map(|h| h.id));
            offset += PAGE_SIZE;
        }
        (ids, totals)
    }
}

#[tokio::test]
async fn hosts_sort_by_lowest_live_mac_with_mac_less_hosts_last() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let net = network(&org.id);
    storage.networks.create(&net).await.unwrap();
    let sub = subnet(&net.id);
    storage.subnets.create(&sub).await.unwrap();

    let fx = Fixture {
        storage,
        network_id: net.id,
        subnet_id: sub.id,
    };

    // Two MACs; the interface's is lower, so it sorts on :02 and lands ahead of `single`.
    let multi = fx.host().await;
    fx.live_ip(multi, Some("00:00:00:00:00:05")).await;
    fx.interface(multi, "00:00:00:00:00:02").await;

    let single = fx.host().await;
    fx.live_ip(single, Some("00:00:00:00:00:03")).await;

    // Its only MAC is on a closed row, lower than every live one. Were it counted, this host
    // would lead the ascending list.
    let stale = fx.host().await;
    fx.live_ip(stale, None).await;
    fx.closed_ip(stale, "00:00:00:00:00:01").await;

    let bare = fx.host().await;
    let unaddressed = fx.host().await;
    fx.live_ip(unaddressed, None).await;

    let mut mac_less = vec![stale, bare, unaddressed];
    mac_less.sort();

    for (dir, with_mac) in [
        (OrderDirection::Asc, [multi, single]),
        (OrderDirection::Desc, [single, multi]),
    ] {
        let (ids, totals) = fx.walk(&services, dir).await;

        assert!(
            totals.iter().all(|&t| t == 5),
            "{dir:?}: every page counts each host once, got {totals:?}"
        );
        assert_eq!(ids.len(), 5, "{dir:?}: every host appears exactly once");
        assert_eq!(
            ids[..2],
            with_mac,
            "{dir:?}: hosts with a MAC, by lowest MAC"
        );
        let mut tail = ids[2..].to_vec();
        tail.sort();
        assert_eq!(tail, mac_less, "{dir:?}: MAC-less hosts come last");
    }
}
