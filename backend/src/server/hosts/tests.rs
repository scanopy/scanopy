//! Host API-shape tests.

use crate::server::hosts::r#impl::api::HostResponse;
use crate::server::hosts::r#impl::attributes::{
    HostChassisIdValue, HostFirmwareRevisionValue, HostManagementUrlValue, HostManufacturerValue,
    HostModelValue, HostSerialNumberValue, HostSoftwareRevisionValue, HostSysContactValue,
    HostSysDescrValue, HostSysLocationValue, HostSysNameValue, HostSysObjectIdValue,
};
use crate::server::hosts::r#impl::base::Host;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeSource, Attributed};
use crate::server::shared::types::examples;

/// `HostResponse` is the only shape a host is ever read back through, and `to_host()` claims to be
/// its inverse. Every field dropped between the two is a column a customer can write and never
/// read — which is exactly how `sys_name`, `manufacturer`, `model` and `serial_number` stayed
/// invisible while being collected and stored. Assert the whole `HostBase` survives the round trip
/// rather than naming fields, so a field added later is covered without editing this test.
///
/// Now that every attribute carries its source, the round trip has to preserve both halves — the
/// response splits each pair into two flat fields and `to_host` reassembles it, and a source
/// dropped on the way through would silently demote every attribute to `Unspecified` the next
/// time a daemon read a host back and re-submitted it. The two sources below are deliberately
/// different so a conversion that hard-codes one would fail.
#[test]
fn host_response_round_trip_preserves_every_base_field() {
    let probe = AttributeSource::Probe(ClientProbe::Snmp);
    let authored = AttributeSource::Authored(ClientProbe::Snmp);

    let mut host = examples::host();
    host.base.sys_descr = Some(Attributed::new(
        HostSysDescrValue("Cisco IOS Software, C2960X".to_string()),
        probe,
    ));
    host.base.sys_object_id = Some(Attributed::new(
        HostSysObjectIdValue("1.3.6.1.4.1.9.1.2494".to_string()),
        probe,
    ));
    host.base.sys_location = Some(Attributed::new(
        HostSysLocationValue("Rack 4, DC1".to_string()),
        authored,
    ));
    host.base.sys_contact = Some(Attributed::new(
        HostSysContactValue("noc@example.com".to_string()),
        authored,
    ));
    host.base.management_url = Some(Attributed::new(
        HostManagementUrlValue("https://10.0.0.2".to_string()),
        probe,
    ));
    host.base.chassis_id = Some(Attributed::new(
        HostChassisIdValue("00:1a:2b:3c:4d:5e".to_string()),
        probe,
    ));
    host.base.sys_name = Some(Attributed::new(
        HostSysNameValue("core-sw-01".to_string()),
        probe,
    ));
    host.base.manufacturer = Some(Attributed::new(
        HostManufacturerValue("Cisco".to_string()),
        probe,
    ));
    host.base.model = Some(Attributed::new(
        HostModelValue("WS-C2960X-48FPD-L".to_string()),
        probe,
    ));
    host.base.serial_number = Some(Attributed::new(
        HostSerialNumberValue("FOC1234X5YZ".to_string()),
        probe,
    ));
    host.base.firmware_revision = Some(Attributed::new(
        HostFirmwareRevisionValue("15.0(4)".to_string()),
        probe,
    ));
    host.base.software_revision = Some(Attributed::new(
        HostSoftwareRevisionValue("15.0(2)SE11".to_string()),
        probe,
    ));

    let response =
        HostResponse::from_host_with_children(host.clone(), vec![], vec![], vec![], vec![]);

    assert_eq!(response.to_host().base, host.base);
}

/// The Hosts tab's site filter widened from one id to a list. The old
/// spelling is what every existing caller — and the published API — sends, so
/// it has to keep deserialising, into a one-element list rather than an error.
/// This is the compatibility claim the widening rests on; without it the change
/// silently breaks callers that pass `?site_id=<uuid>`.
#[tokio::test]
async fn site_filter_accepts_both_the_old_and_new_spellings() {
    use crate::server::hosts::handlers::HostFilterQuery;
    use crate::server::shared::extractors::Query;
    use axum::extract::FromRequestParts;
    use uuid::Uuid;

    async fn parse(query: &str) -> HostFilterQuery {
        let request = axum::http::Request::builder()
            .uri(format!("/api/v1/hosts?{query}"))
            .body(())
            .expect("request builds");
        let (mut parts, ()) = request.into_parts();
        let Query(parsed) = Query::<HostFilterQuery>::from_request_parts(&mut parts, &())
            .await
            .expect("query deserialises");
        parsed
    }

    let one = Uuid::new_v4();
    let two = Uuid::new_v4();

    assert_eq!(
        parse(&format!("site_id={one}")).await.site_ids,
        Some(vec![one]),
        "the singular spelling must still be accepted"
    );
    assert_eq!(
        parse(&format!("site_ids={one}&site_ids={two}"))
            .await
            .site_ids,
        Some(vec![one, two]),
        "repeating the parameter must collect into a list"
    );
    assert_eq!(
        parse("limit=10").await.site_ids,
        None,
        "an absent filter must stay absent rather than becoming an empty set"
    );
}

/// A site filter must only ever narrow what the caller can already see.
/// Requesting a site they have no access to has to yield nothing, and a
/// mixed request has to keep only the accessible half — otherwise the filter
/// becomes a way to read another tenant's hosts.
#[test]
fn site_filter_cannot_widen_access_beyond_the_callers_sites() {
    use crate::server::hosts::handlers::HostFilterQuery;
    use crate::server::shared::handlers::query::FilterQueryExtractor;
    use crate::server::shared::storage::filter::StorableFilter;
    use uuid::Uuid;

    let mine = Uuid::new_v4();
    let theirs = Uuid::new_v4();
    let org = Uuid::new_v4();

    let applied = |requested: Vec<Uuid>| {
        let query = HostFilterQuery {
            site_ids: Some(requested),
            ..Default::default()
        };
        query.apply_to_filter(StorableFilter::<Host>::new_unfiltered(), &[mine], org)
    };

    let foreign = applied(vec![theirs]);
    assert_eq!(
        foreign.to_where_clause().trim(),
        "WHERE FALSE",
        "a site the caller cannot see must match nothing"
    );
    assert!(
        foreign.values().is_empty(),
        "the inaccessible id must not even be bound"
    );

    let mixed = applied(vec![mine, theirs]);
    assert_eq!(
        mixed.values().len(),
        1,
        "only the accessible site may survive the intersection"
    );
}
