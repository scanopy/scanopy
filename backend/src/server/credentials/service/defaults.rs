//! The organization's default credentials: seeding them, assigning them to new sites, and the
//! rules that keep exactly one per seeded type.
//!
//! A default is recognized by content alone (`CredentialType::is_default`). It can't be deleted,
//! and its settings can't change, so it stays the default. Its site and host assignments can be
//! removed like any other credential's.

use super::*;
use crate::server::credentials::r#impl::base::CredentialBase;
use crate::server::shared::storage::traits::Storage;
use crate::server::shared::types::entities::EntitySource;
use crate::server::shared::types::{api::ApiError, error_codes::ErrorCode};
use strum::IntoEnumIterator;

impl CredentialService {
    /// The organization's default credential of this type, if it has one. Should there be more
    /// than one, the oldest is the default.
    pub async fn find_default(
        &self,
        organization_id: Uuid,
        discriminant: CredentialTypeDiscriminants,
    ) -> Result<Option<Credential>, Error> {
        let filter = StorableFilter::<Credential>::new_from_org_id(&organization_id).json_field_eq(
            "credential_type",
            "type",
            discriminant.into(),
        );
        Ok(self
            .get_all(filter)
            .await?
            .into_iter()
            .filter(|c| c.is_default)
            .min_by_key(|c| c.created_at))
    }

    /// Create each seeded default the organization lacks, and return the ones created. Running it
    /// again creates nothing.
    pub async fn seed_defaults(&self, organization_id: Uuid) -> Result<Vec<Credential>, Error> {
        let mut created = Vec::new();
        for discriminant in CredentialTypeDiscriminants::iter() {
            let Some(seeded) = discriminant.seeded_default() else {
                continue;
            };
            if self
                .find_default(organization_id, discriminant)
                .await?
                .is_some()
            {
                continue;
            }
            let credential = Credential::new(CredentialBase {
                organization_id,
                name: seeded.name.to_string(),
                description: Some(seeded.description.to_string()),
                credential_type: seeded.credential_type,
                source: EntitySource::System,
                ..Default::default()
            });
            // Straight to storage: seeding is not a person creating a credential, so it must not
            // complete the first-credential milestones or count as a credential created.
            created.push(self.storage.create(&credential).await?);
        }
        Ok(created)
    }

    /// The organization's defaults whose type applies site-wide (SNMP `public`): what a new site
    /// is assigned when its create request names no credentials.
    pub async fn site_default_credential_ids(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<Uuid>, Error> {
        let mut ids = Vec::new();
        for discriminant in CredentialTypeDiscriminants::iter() {
            let Some(seeded) = discriminant.seeded_default() else {
                continue;
            };
            if !seeded.credential_type.targets().contains(&Target::Site) {
                continue;
            }
            match self.find_default(organization_id, discriminant).await? {
                Some(default) => ids.push(default.id),
                None => tracing::warn!(
                    %organization_id,
                    credential_type = %discriminant,
                    "The organization has no default credential of this type; a new site gets none"
                ),
            }
        }
        Ok(ids)
    }

    /// Refuse a new or edited credential that would be a second copy of an existing default.
    pub(super) async fn refuse_second_default(&self, candidate: &Credential) -> Result<(), Error> {
        if !candidate.base.credential_type.is_default() {
            return Ok(());
        }
        let existing = self
            .find_default(
                candidate.base.organization_id,
                candidate.base.credential_type.discriminant(),
            )
            .await?;
        match existing {
            Some(existing) if existing.id != candidate.id => {
                Err(ApiError::validation(ErrorCode::CredentialDefaultExists {
                    name: existing.base.name,
                })
                .into())
            }
            _ => Ok(()),
        }
    }
}

/// Refuse an edit that changes a default's settings or daemon OS. Its name, description and tags
/// stay editable.
pub(super) fn refuse_default_settings_change(
    current: &Credential,
    updated: &Credential,
) -> Result<(), ApiError> {
    if !current.is_default {
        return Ok(());
    }
    let unchanged = updated.base.credential_type.discriminant()
        == current.base.credential_type.discriminant()
        && updated.base.credential_type.is_default()
        && updated.base.daemon_os == current.base.daemon_os;
    if unchanged {
        Ok(())
    } else {
        Err(ApiError::validation(
            ErrorCode::CredentialDefaultSettingsLocked {
                name: current.base.name.clone(),
            },
        ))
    }
}

/// Refuse deleting any of `credentials` that is a default.
pub(super) fn refuse_default_delete(credentials: &[Credential]) -> Result<(), ApiError> {
    match credentials.iter().find(|c| c.is_default) {
        Some(default) => Err(ApiError::validation(
            ErrorCode::CredentialDefaultDeleteForbidden {
                name: default.base.name.clone(),
            },
        )),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::types::SecretValue;
    use secrecy::SecretString;

    fn credential(credential_type: CredentialType) -> Credential {
        Credential::new(CredentialBase {
            credential_type,
            ..Default::default()
        })
    }

    fn default_of(discriminant: CredentialTypeDiscriminants) -> Credential {
        credential(discriminant.seeded_default().unwrap().credential_type)
    }

    fn community(value: &str) -> CredentialType {
        CredentialType::SnmpCommunity {
            community: SecretValue::Inline {
                value: SecretString::from(value.to_string()),
            },
        }
    }

    #[test]
    fn a_default_keeps_its_settings_but_can_be_renamed() {
        let current = default_of(CredentialTypeDiscriminants::SnmpCommunity);

        let mut renamed = current.clone();
        renamed.base.name = "Factory community".to_string();
        renamed.base.description = None;
        assert!(refuse_default_settings_change(&current, &renamed).is_ok());

        let mut recommunitied = current.clone();
        recommunitied.base.credential_type = community("private");
        assert!(refuse_default_settings_change(&current, &recommunitied).is_err());
    }

    #[test]
    fn a_default_cannot_change_type_even_into_another_default() {
        let current = default_of(CredentialTypeDiscriminants::DockerSocket);
        let mut retyped = current.clone();
        retyped.base.credential_type = CredentialTypeDiscriminants::PodmanSocket
            .seeded_default()
            .unwrap()
            .credential_type;
        assert!(refuse_default_settings_change(&current, &retyped).is_err());

        let mut daemon_os = current.clone();
        daemon_os.base.daemon_os = Some(crate::server::credentials::r#impl::types::OsFamily::Unix);
        assert!(refuse_default_settings_change(&current, &daemon_os).is_err());
    }

    #[test]
    fn any_edit_of_an_ordinary_credential_passes_this_rule() {
        let current = credential(community("private"));
        let mut edited = current.clone();
        edited.base.credential_type = community("other");
        assert!(refuse_default_settings_change(&current, &edited).is_ok());
    }

    #[test]
    fn a_delete_naming_any_default_is_refused() {
        let ordinary = credential(community("private"));
        let default = default_of(CredentialTypeDiscriminants::PodmanSocket);
        assert!(refuse_default_delete(std::slice::from_ref(&ordinary)).is_ok());
        assert!(refuse_default_delete(&[ordinary, default]).is_err());
    }
}
