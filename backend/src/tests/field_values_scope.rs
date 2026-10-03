//! The field-values endpoint counts only rows the caller could list: a network outside their
//! `network_ids` contributes no values, whether or not the request names it.

use crate::server::hosts::handlers::{HostFilterQuery, HostOrderField};
use crate::server::hosts::r#impl::attributes::HostManufacturerValue;
use crate::server::hosts::r#impl::base::Host;
use crate::server::shared::attribution::{AttributeSource, Attributed};
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::traits::field_values_filter;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Storage;

use super::{host, network, organization, test_services};

#[tokio::test]
async fn field_values_exclude_networks_outside_the_callers_access() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let visible = network(&org.id);
    storage.networks.create(&visible).await.unwrap();
    let hidden = network(&org.id);
    storage.networks.create(&hidden).await.unwrap();

    for (network_id, manufacturer) in [(visible.id, "Cisco"), (hidden.id, "Juniper")] {
        let mut h = host(&network_id);
        h.base.manufacturer = Some(Attributed::new(
            HostManufacturerValue(manufacturer.to_string()),
            AttributeSource::ReverseDns,
        ));
        storage.hosts.create(&h).await.unwrap();
    }

    let field = HostOrderField::Manufacturer;
    let values = |query: HostFilterQuery| {
        let filter = field_values_filter::<Host>(&[visible.id], org.id, &query, field);
        let services = &services;
        async move {
            services
                .host_service
                .count_by_group(filter, field.to_sql())
                .await
                .unwrap()
                .into_iter()
                .map(|g| g.value)
                .collect::<Vec<_>>()
        }
    };

    assert_eq!(
        values(HostFilterQuery::default()).await,
        vec![Some("Cisco".to_string())],
        "an unscoped request must only count the caller's networks"
    );

    let asks_for_hidden = HostFilterQuery {
        network_ids: Some(vec![hidden.id]),
        ..Default::default()
    };
    assert!(
        values(asks_for_hidden).await.is_empty(),
        "naming an inaccessible network must narrow to nothing, never widen"
    );
}
