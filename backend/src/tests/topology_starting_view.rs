//! The view a site's topology tab opens on, and what it costs to pick it.

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, registry};

use crate::server::metrics::db::{DbTimingLayer, measure};
use crate::server::organizations::r#impl::intent::SignupIntent;
use crate::server::shared::storage::traits::Storage;
use crate::server::topology::types::views::TopologyView;

use super::{organization, site, test_services};

/// The topology list picks a starting view for every site. A view that needs no data to be
/// offered is picked without reading the site's interfaces and addresses, which on a large site
/// are thousands of rows per list row.
#[tokio::test]
async fn a_view_that_needs_no_data_is_picked_without_reading_the_site() {
    let subscriber = registry().with(DbTimingLayer.with_filter(DbTimingLayer::filter()));
    let _tracing = tracing::subscriber::set_default(subscriber);

    let (storage, services, _db) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let site = site(&org.id);
    storage.sites.create(&site).await.unwrap();
    let topology = &services.topology_service;

    tracing::callsite::rebuild_interest_cache();
    let (view, totals) = measure(topology.starting_view(&[SignupIntent::Workloads], site.id)).await;
    assert_eq!(view.unwrap(), TopologyView::Workloads);
    assert_eq!(totals.queries, 0, "Workloads needs no support lookup");

    // L2 Physical is offered only with link data, which this empty site lacks, so the lookup
    // runs and the tab opens on the default view.
    let (view, totals) =
        measure(topology.starting_view(&[SignupIntent::Troubleshooting], site.id)).await;
    assert_eq!(view.unwrap(), TopologyView::default());
    assert!(
        totals.queries > 0,
        "L2 Physical is offered only after the lookup"
    );
}
