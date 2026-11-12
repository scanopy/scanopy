use std::time::Duration;

use axum::response::IntoResponse;
use reqwest::StatusCode;

use super::super::key::LicenseKey;
use super::super::key::test_support::{ORG_ID, online_key};
use super::super::problem::LicenseProblem;
use super::super::types::{LicensePlan, LicenseStatus};
use super::LicenseService;
use super::tests::{cloud, entitlement_cloud, unreachable_cloud, unreachable_org_service};
use crate::server::shared::trusted_ca::test_support::serve_test_tls;
use crate::server::shared::types::api::ApiError;

async fn online_license() -> LicenseService {
    LicenseService::new(
        online_key(ORG_ID),
        unreachable_org_service(),
        None,
        None,
        None,
        false,
    )
    .await
}

/// A pending server checks in against `base_url`, and reports what it found.
async fn problem_after_check_in(base_url: &str) -> (Option<LicenseProblem>, LicenseStatus) {
    let license = online_license().await;
    license.check_in(base_url).await;
    (license.problem().await, license.current_status().await)
}

#[tokio::test]
async fn a_pasted_key_with_whitespace_or_quotes_still_checks_in() {
    let key = online_key(ORG_ID);
    let raw = key.as_str();
    let (head, tail) = raw.split_at(raw.len() / 2);

    for pasted in [
        format!("  {raw}\n"),
        format!("{head}\n{tail}"),
        format!("{head}\r\n    {tail}"),
        format!("'{raw}'"),
        format!("\"{raw}\""),
        format!("\u{feff}{raw}"),
    ] {
        let normalized = LicenseKey::normalized(&pasted).expect("a key remains");
        let license = LicenseService::new(
            normalized,
            unreachable_org_service(),
            None,
            None,
            None,
            false,
        )
        .await;
        assert!(license.is_online(), "not read as an online key: {pasted:?}");

        license
            .check_in(&entitlement_cloud(LicensePlan::Plus, 30).await)
            .await;
        assert!(
            matches!(license.current_status().await, LicenseStatus::Valid(_)),
            "did not check in: {pasted:?}"
        );
    }
}

#[test]
fn a_blank_key_is_no_key() {
    for blank in ["", "   ", "\n", "''", "\"\""] {
        assert!(LicenseKey::normalized(blank).is_none(), "{blank:?}");
    }
}

#[tokio::test]
async fn a_truncated_key_names_the_problem_and_never_reads_as_online() {
    let key = online_key(ORG_ID);
    let truncated = &key.as_str()[..key.as_str().len() - 12];
    let license = LicenseService::new(
        LicenseKey::new(truncated.to_string()),
        unreachable_org_service(),
        None,
        None,
        None,
        false,
    )
    .await;

    assert!(!license.is_online());
    assert_eq!(license.problem().await, Some(LicenseProblem::KeyMalformed));
    let LicenseStatus::Invalid(reason) = license.current_status().await else {
        panic!("a truncated key should be invalid");
    };
    assert!(reason.contains(LicenseProblem::KeyMalformed.log_message()));
}

#[tokio::test]
async fn an_unresolvable_host_is_a_dns_problem() {
    let (problem, status) = problem_after_check_in("http://scanopy-license.invalid").await;
    assert_eq!(problem, Some(LicenseProblem::Dns));
    assert!(matches!(status, LicenseStatus::Pending));
}

#[tokio::test]
async fn a_refused_port_is_a_connect_problem() {
    let (problem, status) = problem_after_check_in(&unreachable_cloud().await).await;
    assert_eq!(problem, Some(LicenseProblem::Connect));
    assert!(matches!(status, LicenseStatus::Pending));
}

#[tokio::test]
async fn no_answer_in_time_is_a_timeout() {
    // Accepts the connection and never answers.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _held = listener.accept().await;
        std::future::pending::<()>().await;
    });

    let mut license = online_license().await;
    license.http = reqwest::Client::builder()
        .timeout(Duration::from_millis(200))
        .build()
        .unwrap();
    license.check_in(&format!("http://{addr}")).await;

    assert_eq!(license.problem().await, Some(LicenseProblem::Timeout));
}

#[tokio::test]
async fn an_intercepted_tls_connection_is_an_untrusted_certificate() {
    let port = serve_test_tls().await;
    let (problem, status) = problem_after_check_in(&format!("https://localhost:{port}")).await;
    assert_eq!(problem, Some(LicenseProblem::UntrustedCertificate));
    assert!(matches!(status, LicenseStatus::Pending));
}

#[tokio::test]
async fn a_proxy_refusing_the_tunnel_is_a_proxy_problem() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0u8; 1024];
        let _ = socket.read(&mut buffer).await;
        let _ = socket
            .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
            .await;
    });

    let mut license = online_license().await;
    license.http = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(format!("http://{addr}")).unwrap())
        .build()
        .unwrap();
    license.check_in("https://license.test").await;

    assert_eq!(license.problem().await, Some(LicenseProblem::Proxy));
}

#[tokio::test]
async fn statuses_that_say_nothing_about_the_key_leave_it_pending() {
    for (status, expected) in [
        (
            StatusCode::PROXY_AUTHENTICATION_REQUIRED,
            LicenseProblem::Proxy,
        ),
        (StatusCode::TOO_MANY_REQUESTS, LicenseProblem::RateLimited),
        (StatusCode::SERVICE_UNAVAILABLE, LicenseProblem::ServerError),
    ] {
        let base_url = cloud(move || (status, "nope").into_response()).await;
        let (problem, license_status) = problem_after_check_in(&base_url).await;
        assert_eq!(problem, Some(expected), "{status}");
        assert!(matches!(license_status, LicenseStatus::Pending), "{status}");
    }
}

/// A firewall's block page must not read as the cloud refusing the key: that
/// used to drop the entitlement and lock a licensed server.
#[tokio::test]
async fn a_proxy_403_keeps_a_licensed_server_licensed() {
    let license = online_license().await;
    license
        .check_in(&entitlement_cloud(LicensePlan::Plus, 30).await)
        .await;
    assert!(matches!(
        license.current_status().await,
        LicenseStatus::Valid(_)
    ));

    let base_url = cloud(|| {
        (
            StatusCode::FORBIDDEN,
            axum::response::Html("<h1>Blocked by policy</h1>"),
        )
            .into_response()
    })
    .await;
    license.check_in(&base_url).await;

    assert!(matches!(
        license.current_status().await,
        LicenseStatus::Valid(_)
    ));
    // Still licensed, so the UI must not tell this server it is read-only.
    assert_eq!(license.problem().await, None);
}

#[tokio::test]
async fn a_200_block_page_is_a_proxy_problem() {
    let base_url = cloud(|| axum::response::Html("<h1>Sign in</h1>").into_response()).await;
    let (problem, status) = problem_after_check_in(&base_url).await;
    assert_eq!(problem, Some(LicenseProblem::Proxy));
    assert!(matches!(status, LicenseStatus::Pending));
}

#[tokio::test]
async fn cloud_refusals_name_their_cause() {
    for (refusal, expected) in [
        (
            ApiError::license_key_regenerated as fn() -> ApiError,
            LicenseProblem::KeyRegenerated,
        ),
        (
            ApiError::license_organization_missing,
            LicenseProblem::OrganizationMissing,
        ),
        (
            ApiError::license_plan_has_no_key,
            LicenseProblem::PlanHasNoKey,
        ),
        (
            || ApiError::forbidden("Some other refusal"),
            LicenseProblem::KeyRefused,
        ),
    ] {
        let base_url = cloud(move || refusal().into_response()).await;
        let (problem, status) = problem_after_check_in(&base_url).await;
        assert_eq!(problem, Some(expected));
        assert!(matches!(status, LicenseStatus::Invalid(_)));
    }
}

#[tokio::test]
async fn a_successful_check_in_clears_the_problem() {
    let license = online_license().await;
    license.check_in(&unreachable_cloud().await).await;
    assert_eq!(license.problem().await, Some(LicenseProblem::Connect));

    license
        .check_in(&entitlement_cloud(LicensePlan::Plus, 30).await)
        .await;

    assert_eq!(license.problem().await, None);
}

#[tokio::test]
async fn only_a_pending_server_retries_within_minutes() {
    let license = online_license().await;
    assert!(license.next_check_in_wait().await < Duration::from_secs(10 * 60));

    license
        .check_in(&entitlement_cloud(LicensePlan::Plus, 30).await)
        .await;
    assert!(license.next_check_in_wait().await >= Duration::from_secs(6 * 60 * 60));

    let refused = online_license().await;
    let base_url = cloud(|| ApiError::license_key_regenerated().into_response()).await;
    refused.check_in(&base_url).await;
    assert!(refused.next_check_in_wait().await >= Duration::from_secs(6 * 60 * 60));
}
