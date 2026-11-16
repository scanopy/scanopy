//! The Issues CSV export, through the real router and a real key.

use super::api_key_site_access::{serve, user_key};
use super::self_hosted_licensing::{cloud_test_state, create_org};
use super::{daemon, host, site, user};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::get_enterprise_plan;
use crate::server::shared::services::traits::CrudService;
use crate::server::users::r#impl::permissions::UserOrgPermissions;

/// A key on one site exports that site's Issues, named, and none from a site it can't read.
#[tokio::test]
async fn the_issues_export_holds_only_issues_on_sites_the_caller_reads() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let services = &state.services;
    let mut installer = user(&org.id);
    installer.base.email = "installer@example.com".parse().unwrap();
    let installer = services
        .user_service
        .create(installer, AuthenticatedEntity::System)
        .await
        .unwrap();
    let mut site_ids = Vec::new();
    for name in ["Readable daemon", "Hidden daemon"] {
        let site = services
            .site_service
            .create(site(&org.id), AuthenticatedEntity::System)
            .await
            .unwrap();
        let host = services
            .host_service
            .create(host(&site.id), AuthenticatedEntity::System)
            .await
            .unwrap();
        // No version reported, so each daemon is an Issue on its own site.
        let mut daemon = daemon(&site.id, &host.id);
        daemon.base.name = name.to_string();
        daemon.base.user_id = installer.id;
        services
            .daemon_service
            .create(daemon, AuthenticatedEntity::System)
            .await
            .unwrap();
        site_ids.push(site.id);
    }
    let (_, key) = user_key(
        &state,
        org.id,
        UserOrgPermissions::Viewer,
        vec![site_ids[0]],
    )
    .await;
    let addr = serve(&state).await;

    let response = reqwest::Client::new()
        .get(format!("http://{addr}/api/v1/dashboard/issues/export/csv"))
        .bearer_auth(&key)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.headers()["content-type"], "text/csv");
    let body = response.text().await.unwrap();

    let mut reader = csv::Reader::from_reader(body.as_bytes());
    let headers = reader.headers().unwrap().clone();
    let daemons_column = headers.iter().position(|h| h == "daemons").unwrap();
    let named: Vec<String> = reader
        .records()
        .map(|record| record.unwrap()[daemons_column].to_string())
        .collect();
    assert!(named.contains(&"Readable daemon".to_string()), "{body}");
    assert!(!body.contains("Hidden daemon"), "{body}");
}
