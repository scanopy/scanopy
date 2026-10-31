//! SSH logins and the credentials that run over them.
//!
//! A login (`CredentialRole::Login`) is who the daemon is on a host. It runs nothing and is
//! assigned nowhere; a credential whose type requires one names it as its `login_credential_id`,
//! and the daemon receives the two combined. These are the rules that keep that link sound.

use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::Error;
use strum::IntoDiscriminant;
use uuid::Uuid;

use super::CredentialService;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::credentials::r#impl::base::Credential;
use crate::server::credentials::r#impl::mapping::CredentialQueryPayload;
use crate::server::credentials::r#impl::types::{CredentialRole, OsFamily};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::types::api::ApiError;
use crate::server::shared::types::error_codes::ErrorCode;

impl CredentialService {
    /// `credential` as a daemon receives it, its login fetched once per dispatch into `logins`
    /// and kept only when `owned` (the scanned site's organization) holds it. `None` for a
    /// login, which is sent only inside the credentials that use it, and for a credential whose
    /// login is unavailable, which also gets a `CredentialLoginUnavailable` warning in `warnings`.
    pub(super) async fn dispatchable_or_warn(
        &self,
        credential: &Credential,
        owned: &impl Fn(&Credential) -> bool,
        logins: &mut HashMap<Uuid, Option<Credential>>,
        warnings: &mut BTreeMap<Uuid, DiscoveryWarning>,
    ) -> Result<Option<Dispatchable>, Error> {
        let login = match credential.base.login_credential_id {
            Some(id) => {
                if !logins.contains_key(&id) {
                    let login = self.get_by_id(&id).await?.filter(|l| owned(l));
                    logins.insert(id, login);
                }
                logins.get(&id).and_then(Option::as_ref)
            }
            None => None,
        };
        match dispatchable(credential, login) {
            Ok(dispatch) => Ok(Some(dispatch)),
            Err(NotDispatchable::Login) => Ok(None),
            Err(NotDispatchable::LoginUnavailable) => {
                warnings.entry(credential.id).or_insert(
                    DiscoveryWarning::CredentialLoginUnavailable {
                        integration: credential
                            .base
                            .credential_type
                            .discriminant()
                            .wire_discriminant(),
                        credential_id: Some(credential.id),
                    },
                );
                Ok(None)
            }
        }
    }

    /// Refuse a credential whose login is missing where its type needs one, named where it does
    /// not, or is not a login of the credential's own organization, or whose daemon OS disagrees
    /// with its login's.
    pub(super) async fn refuse_invalid_login(&self, credential: &Credential) -> Result<(), Error> {
        let requires_login = credential
            .base
            .credential_type
            .discriminant()
            .requires_login();
        let login_id = match (requires_login, credential.base.login_credential_id) {
            (true, None) => {
                return Err(ApiError::validation(ErrorCode::CredentialLoginRequired).into());
            }
            (false, Some(_)) => {
                return Err(ApiError::validation(ErrorCode::CredentialLoginNotUsed).into());
            }
            (false, None) => return Ok(()),
            (true, Some(id)) => id,
        };
        let login = self
            .get_by_id(&login_id)
            .await?
            .filter(|login| {
                login.base.organization_id == credential.base.organization_id
                    && login.base.credential_type.discriminant().role() == CredentialRole::Login
            })
            .ok_or_else(|| ApiError::validation(ErrorCode::CredentialLoginInvalid))?;
        if let (Some(own), Some(theirs)) = (credential.base.daemon_os, login.base.daemon_os)
            && own != theirs
        {
            return Err(
                ApiError::validation(ErrorCode::CredentialLoginDaemonOsMismatch {
                    login: login.base.name,
                })
                .into(),
            );
        }
        Ok(())
    }

    /// Refuse deleting any login in `ids` that a credential outside `ids` still runs over.
    /// Deleting a login together with every credential that uses it is fine.
    pub(super) async fn refuse_login_in_use(&self, deleting: &[Credential]) -> Result<(), Error> {
        let logins: Vec<&Credential> = deleting
            .iter()
            .filter(|c| c.base.credential_type.discriminant().role() == CredentialRole::Login)
            .collect();
        if logins.is_empty() {
            return Ok(());
        }
        let login_ids: Vec<Uuid> = logins.iter().map(|l| l.id).collect();
        let deleting_ids: HashSet<Uuid> = deleting.iter().map(|c| c.id).collect();
        let users = self
            .get_all(
                StorableFilter::<Credential>::new().uuids_column("login_credential_id", &login_ids),
            )
            .await?;
        for login in logins {
            let remaining: Vec<&str> = users
                .iter()
                .filter(|u| {
                    u.base.login_credential_id == Some(login.id) && !deleting_ids.contains(&u.id)
                })
                .map(|u| u.base.name.as_str())
                .collect();
            if !remaining.is_empty() {
                return Err(ApiError::validation(ErrorCode::CredentialLoginInUse {
                    name: login.base.name.clone(),
                    users: remaining.join(", "),
                })
                .into());
            }
        }
        Ok(())
    }

    /// For each login among `credentials`, the credentials that run over it.
    pub(crate) async fn users_of_logins(
        &self,
        credentials: &[Credential],
    ) -> Result<HashMap<Uuid, Vec<Uuid>>, Error> {
        let login_ids: Vec<Uuid> = credentials
            .iter()
            .filter(|c| c.base.credential_type.discriminant().role() == CredentialRole::Login)
            .map(|c| c.id)
            .collect();
        if login_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let users = self
            .get_all(
                StorableFilter::<Credential>::new().uuids_column("login_credential_id", &login_ids),
            )
            .await?;
        let mut by_login: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for user in users {
            if let Some(login) = user.base.login_credential_id {
                by_login.entry(login).or_default().push(user.id);
            }
        }
        Ok(by_login)
    }

    /// Refuse assigning any of `credential_ids` that is a login: a login runs nothing, so the
    /// credentials that use it are what gets assigned.
    pub(crate) async fn refuse_login_assignment(
        &self,
        credential_ids: &[Uuid],
    ) -> Result<(), Error> {
        if credential_ids.is_empty() {
            return Ok(());
        }
        let credentials = self
            .get_all(StorableFilter::<Credential>::new_from_entity_ids(
                credential_ids,
            ))
            .await?;
        match credentials
            .into_iter()
            .find(|c| c.base.credential_type.discriminant().role() == CredentialRole::Login)
        {
            Some(login) => Err(
                ApiError::validation(ErrorCode::CredentialLoginNotAssignable {
                    name: login.base.name,
                })
                .into(),
            ),
            None => Ok(()),
        }
    }
}

/// A credential ready to send to a daemon: its wire payload, with its login's fields where its
/// type runs over one, and the daemon OS its files need.
pub(crate) struct Dispatchable {
    pub payload: CredentialQueryPayload,
    pub daemon_os: Option<OsFamily>,
}

/// Why a credential could not be dispatched.
pub(crate) enum NotDispatchable {
    /// A login, which is never sent on its own.
    Login,
    /// Its login is missing, belongs to another organization, or is not a login.
    LoginUnavailable,
}

/// `credential` as a daemon receives it, given its login when its type runs over one. `login`
/// must be the credential its `login_credential_id` names, already checked to be of its
/// organization.
pub(crate) fn dispatchable(
    credential: &Credential,
    login: Option<&Credential>,
) -> Result<Dispatchable, NotDispatchable> {
    let discriminant = credential.base.credential_type.discriminant();
    if discriminant.role() == CredentialRole::Login {
        return Err(NotDispatchable::Login);
    }
    let login = login.filter(|l| {
        l.base.organization_id == credential.base.organization_id
            && l.base.credential_type.discriminant().role() == CredentialRole::Login
    });
    let payload = credential
        .base
        .credential_type
        .to_query_payload(login.map(|l| &l.base.credential_type))
        .ok_or(NotDispatchable::LoginUnavailable)?;
    Ok(Dispatchable {
        payload,
        daemon_os: credential
            .base
            .daemon_os
            .or(login.and_then(|l| l.base.daemon_os)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::base::CredentialBase;
    use crate::server::credentials::r#impl::mapping::SshAuth;
    use crate::server::credentials::r#impl::types::{
        CredentialType, CredentialTypeDiscriminants, ScriptSource, SecretValue,
    };

    fn credential(organization_id: Uuid, credential_type: CredentialType) -> Credential {
        Credential::new(CredentialBase {
            organization_id,
            credential_type,
            ..Default::default()
        })
    }

    fn login(organization_id: Uuid) -> Credential {
        credential(
            organization_id,
            CredentialType::SshPassword {
                port: 2222,
                username: "scanopy".to_string(),
                password: SecretValue::Inline {
                    value: secrecy::SecretString::from("pw".to_string()),
                },
                host_key_fingerprint: Some("SHA256:abc".to_string()),
            },
        )
    }

    fn script(organization_id: Uuid, login: &Credential) -> Credential {
        let mut script = credential(
            organization_id,
            CredentialType::SshScript {
                target_os: OsFamily::Unix,
                script: ScriptSource::Inline {
                    value: "inventory".to_string(),
                },
                timeout_seconds: 30,
            },
        );
        script.base.login_credential_id = Some(login.id);
        script
    }

    /// A script goes out on the SSH wire older daemons read, with its login's account and the
    /// script's own settings.
    #[test]
    fn a_script_is_sent_with_its_logins_account() {
        let org = Uuid::new_v4();
        let login = login(org);
        let dispatch = dispatchable(&script(org, &login), Some(&login))
            .ok()
            .expect("dispatchable");
        let CredentialQueryPayload::Ssh(ssh) = dispatch.payload else {
            panic!("an SSH payload");
        };
        assert_eq!((ssh.port, ssh.username.as_str()), (2222, "scanopy"));
        assert!(matches!(ssh.auth, SshAuth::Password { .. }));
        assert_eq!(ssh.host_key_fingerprint.as_deref(), Some("SHA256:abc"));
        assert_eq!(ssh.timeout_seconds, 30);
        assert_eq!(
            ssh.script,
            ScriptSource::Inline {
                value: "inventory".to_string()
            }
        );
    }

    /// A login is never sent on its own, and a script without a usable login is not sent.
    #[test]
    fn a_login_or_a_script_without_one_is_not_sent() {
        let org = Uuid::new_v4();
        let login = login(org);
        assert!(matches!(
            dispatchable(&login, None),
            Err(NotDispatchable::Login)
        ));
        assert!(matches!(
            dispatchable(&script(org, &login), None),
            Err(NotDispatchable::LoginUnavailable)
        ));
        // Another organization's login.
        let foreign = self::login(Uuid::new_v4());
        assert!(matches!(
            dispatchable(&script(org, &foreign), Some(&foreign)),
            Err(NotDispatchable::LoginUnavailable)
        ));
        // A credential that is not a login.
        let snmp = credential(
            org,
            CredentialTypeDiscriminants::SnmpCommunity.to_credential_type(),
        );
        assert!(matches!(
            dispatchable(&script(org, &snmp), Some(&snmp)),
            Err(NotDispatchable::LoginUnavailable)
        ));
    }

    /// A script whose files are all on the scanned host takes its daemon OS from its login's key.
    #[test]
    fn a_script_takes_its_logins_daemon_os() {
        let org = Uuid::new_v4();
        let mut login = login(org);
        login.base.daemon_os = Some(OsFamily::Windows);
        let dispatch = dispatchable(&script(org, &login), Some(&login))
            .ok()
            .expect("dispatchable");
        assert_eq!(dispatch.daemon_os, Some(OsFamily::Windows));
    }
}
