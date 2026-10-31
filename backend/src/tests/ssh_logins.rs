//! SSH logins through the services: a credential that runs over SSH names a login of its own
//! organization, a login in use cannot be deleted or assigned, and a scan sends the two combined.

use secrecy::SecretString;
use uuid::Uuid;

use super::{organization, site, test_services};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    credentials::{
        r#impl::{
            base::{Credential, CredentialBase},
            mapping::{CredentialQueryPayload, SshAuth},
            types::{CredentialAssignment, CredentialType, OsFamily, ScriptSource, SecretValue},
        },
        service::DaemonPlacement,
    },
    shared::{
        events::types::DeletionReason,
        services::{factory::ServiceFactory, traits::CrudService},
        types::{api::ApiError, error_codes::ErrorCode},
    },
};

fn login_type() -> CredentialType {
    CredentialType::SshPassword {
        port: 22,
        username: "scanopy".to_string(),
        password: SecretValue::Inline {
            value: SecretString::from("password".to_string()),
        },
        host_key_fingerprint: None,
    }
}

fn script_type() -> CredentialType {
    CredentialType::SshScript {
        target_os: OsFamily::Unix,
        script: ScriptSource::Inline {
            value: "inventory".to_string(),
        },
        timeout_seconds: 60,
    }
}

fn credential(
    organization_id: Uuid,
    name: &str,
    credential_type: CredentialType,
    login: Option<Uuid>,
) -> Credential {
    Credential::new(CredentialBase {
        organization_id,
        name: name.to_string(),
        credential_type,
        login_credential_id: login,
        ..Default::default()
    })
}

async fn create(services: &ServiceFactory, credential: Credential) -> anyhow::Result<Credential> {
    services
        .credential_service
        .create(credential, AuthenticatedEntity::System)
        .await
}

async fn new_org(services: &ServiceFactory) -> Uuid {
    services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap()
        .id
}

/// The error code a refused service call carried.
fn error_code<T: std::fmt::Debug>(result: anyhow::Result<T>) -> ErrorCode {
    let err = result.expect_err("the service accepted it");
    err.downcast_ref::<ApiError>()
        .and_then(|e| e.error_code.clone())
        .unwrap_or_else(|| panic!("not a coded error: {err}"))
}

/// A script must name a login, and only a login of its own organization; a login names none.
#[tokio::test]
async fn a_script_names_a_login_of_its_own_organization() {
    let (_storage, services, _container) = test_services().await;
    let org = new_org(&services).await;
    let other_org = new_org(&services).await;
    let login = create(&services, credential(org, "login", login_type(), None))
        .await
        .unwrap();
    let foreign = create(
        &services,
        credential(other_org, "foreign", login_type(), None),
    )
    .await
    .unwrap();
    let not_a_login = create(
        &services,
        credential(org, "script", script_type(), Some(login.id)),
    )
    .await
    .unwrap();

    assert!(matches!(
        error_code(create(&services, credential(org, "a", script_type(), None)).await),
        ErrorCode::CredentialLoginRequired
    ));
    for bad in [foreign.id, not_a_login.id, Uuid::new_v4()] {
        assert!(matches!(
            error_code(create(&services, credential(org, "b", script_type(), Some(bad))).await),
            ErrorCode::CredentialLoginInvalid
        ));
    }
    assert!(matches!(
        error_code(
            create(
                &services,
                credential(org, "c", login_type(), Some(login.id))
            )
            .await
        ),
        ErrorCode::CredentialLoginNotUsed
    ));
}

/// A login in use is refused deletion; deleted together with every credential using it, it goes.
#[tokio::test]
async fn a_login_in_use_is_deleted_only_with_its_users() {
    let (_storage, services, _container) = test_services().await;
    let org = new_org(&services).await;
    let login = create(&services, credential(org, "login", login_type(), None))
        .await
        .unwrap();
    let script = create(
        &services,
        credential(org, "script", script_type(), Some(login.id)),
    )
    .await
    .unwrap();

    let refused = services
        .credential_service
        .delete(
            &login.id,
            AuthenticatedEntity::System,
            DeletionReason::EntityRequested,
        )
        .await;
    assert!(matches!(
        error_code(refused),
        ErrorCode::CredentialLoginInUse { .. }
    ));

    services
        .credential_service
        .delete_many(
            &[login.id, script.id],
            AuthenticatedEntity::System,
            DeletionReason::EntityRequested,
        )
        .await
        .expect("a login goes together with its users");
    assert!(
        services
            .credential_service
            .get_by_id(&login.id)
            .await
            .unwrap()
            .is_none()
    );
}

/// A login is assigned nowhere, and a scan of a site sends its script with the login folded in,
/// on the SSH wire daemons already read.
#[tokio::test]
async fn a_scan_sends_the_script_with_its_login() {
    let (_storage, services, _container) = test_services().await;
    let org = new_org(&services).await;
    let site_id = services
        .site_service
        .create(site(&org), AuthenticatedEntity::System)
        .await
        .unwrap()
        .id;
    let login = create(&services, credential(org, "login", login_type(), None))
        .await
        .unwrap();
    let script = create(
        &services,
        credential(org, "script", script_type(), Some(login.id)),
    )
    .await
    .unwrap();

    assert!(matches!(
        error_code(
            services
                .credential_service
                .set_site_credentials(&site_id, &[login.id])
                .await
        ),
        ErrorCode::CredentialLoginNotAssignable { .. }
    ));
    assert!(matches!(
        error_code(
            services
                .credential_service
                .set_host_credentials(
                    &Uuid::new_v4(),
                    &[CredentialAssignment {
                        credential_id: login.id,
                        ip_address_ids: None,
                    }],
                )
                .await
        ),
        ErrorCode::CredentialLoginNotAssignable { .. }
    ));

    services
        .credential_service
        .set_site_credentials(&site_id, &[script.id])
        .await
        .unwrap();
    let (mappings, warnings) = services
        .credential_service
        .build_all_credential_mappings(
            site_id,
            &[],
            Some(&semver::Version::new(0, 19, 5)),
            &DaemonPlacement::default(),
        )
        .await
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let [mapping] = mappings.as_slice() else {
        panic!("one mapping, the script's: {mappings:?}");
    };
    assert_eq!(mapping.default_credential_id, Some(script.id));
    let Some(CredentialQueryPayload::Ssh(ssh)) = &mapping.default_credential else {
        panic!("an SSH payload");
    };
    assert_eq!(ssh.username, "scanopy");
    assert!(matches!(ssh.auth, SshAuth::Password { .. }));
    assert!(matches!(ssh.script, ScriptSource::Inline { .. }));
}
