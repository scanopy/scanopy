//! Gathers the facts the Setup list reads, through each entity's service, and lists the items.

use std::{collections::BTreeSet, sync::Arc};

use anyhow::Result;
use strum::{IntoDiscriminant, IntoEnumIterator};
use uuid::Uuid;

use super::items::{
    SetupContext, SetupCounts, SetupDaemon, SetupSite, TargetlessCredential, UncoveredIntegration,
};
use super::types::{SetupItemDiscriminants, SetupListEntry, SetupOperation};
use crate::server::{
    auth::{
        access::{Action, Resource},
        middleware::auth::AuthenticatedEntity,
    },
    config::DeploymentType,
    credentials::service::{CredentialService, FoundIntegration},
    daemons::{
        r#impl::{api::DaemonHealth, base::Daemon, version::DaemonVersionPolicy},
        service::DaemonService,
    },
    dependencies::service::DependencyService,
    discovery::service::DiscoveryService,
    hosts::service::HostService,
    invites::{r#impl::base::Invite, service::InviteService},
    organizations::{r#impl::base::Organization, service::OrganizationService},
    shared::{
        entities::EntityDiscriminants, services::traits::CrudService,
        storage::filter::StorableFilter, types::entities::EntitySource,
    },
    shares::service::ShareService,
    sites::{r#impl::Site, service::SiteService},
    subnets::service::SubnetService,
    tags::{r#impl::base::Tag, service::TagService},
    user_api_keys::service::UserApiKeyService,
    users::service::UserService,
};

pub struct SetupService {
    pub site_service: Arc<SiteService>,
    pub daemon_service: Arc<DaemonService>,
    pub discovery_service: Arc<DiscoveryService>,
    pub credential_service: Arc<CredentialService>,
    pub host_service: Arc<HostService>,
    pub subnet_service: Arc<SubnetService>,
    pub dependency_service: Arc<DependencyService>,
    pub share_service: Arc<ShareService>,
    pub user_api_key_service: Arc<UserApiKeyService>,
    pub user_service: Arc<UserService>,
    pub invite_service: Arc<InviteService>,
    pub tag_service: Arc<TagService>,
    pub organization_service: Arc<OrganizationService>,
    pub deployment_type: DeploymentType,
}

impl SetupService {
    /// The org's facts. `entity` is the user the list is for, whose writable resources decide
    /// which items apply and whose readable sites count the unconfirmed rows. `None` for the
    /// sweep.
    pub async fn gather(
        &self,
        org: &Organization,
        entity: Option<&AuthenticatedEntity>,
    ) -> Result<SetupContext> {
        let org_id = org.id;
        let sites: Vec<SetupSite> = self
            .site_service
            .get_all(StorableFilter::<Site>::new_from_org_id(&org_id))
            .await?
            .into_iter()
            .map(|s| SetupSite {
                id: s.id,
                name: s.base.name,
            })
            .collect();
        let site_ids: Vec<Uuid> = sites.iter().map(|s| s.id).collect();

        let policy = DaemonVersionPolicy::default();
        let mut daemon_rows = self
            .daemon_service
            .get_all(StorableFilter::<Daemon>::new_from_site_ids(&site_ids))
            .await?;
        daemon_rows.sort_by_key(|d| d.created_at);
        let first_scan_daemon_id = daemon_rows
            .iter()
            .find(|d| d.base.last_seen.is_some())
            .map(|d| d.id);
        let daemons: Vec<SetupDaemon> = daemon_rows
            .into_iter()
            .map(|d| SetupDaemon {
                id: d.id,
                health: DaemonHealth::of(&d.base, &policy.evaluate(d.base.version.as_ref())),
                name: d.base.name,
                site_id: d.base.site_id,
                version: d.base.version,
            })
            .collect();
        let daemon_ids: Vec<Uuid> = daemons.iter().map(|d| d.id).collect();

        // Re-checked against today's assignments, so a target saved since clears the item without
        // waiting for a scan. Only a credential a person added counts.
        let targetless_credentials = self
            .discovery_service
            .targetless_credentials(&org_id, None)
            .await?
            .into_iter()
            .filter(|c| c.base.source == EntitySource::Manual)
            .map(|c| TargetlessCredential {
                id: c.id,
                integration: c.base.credential_type.discriminant().integration(),
                name: c.base.name,
            })
            .collect();

        // What the latest full scans found with no credential, re-checked against today's
        // assignments so a credential saved since clears it.
        let (found, tried_at) = self
            .discovery_service
            .latest_integrations_without_credentials(&site_ids)
            .await?;
        let uncovered = self
            .credential_service
            .uncovered_integrations(found, &[], None, &tried_at)
            .await?;

        let run_warnings = self
            .discovery_service
            .latest_runs_per_daemon(&daemon_ids)
            .await?;

        let (host_sites, subnet_sites) = match entity {
            Some(e) => (
                e.site_ids_for(Resource::Host, Action::Read),
                e.site_ids_for(Resource::Subnet, Action::Read),
            ),
            None => (site_ids.clone(), site_ids.clone()),
        };
        let unconfirmed_hosts = self
            .host_service
            .unconfirmed_count_for_sites(&host_sites)
            .await?;
        let unconfirmed_subnets = self
            .subnet_service
            .unconfirmed_count_for_sites(&subnet_sites)
            .await?;

        let now = crate::server::shared::clock::now();
        let counts = SetupCounts {
            dependencies: self.dependency_service.count_for_sites(&site_ids).await?,
            user_api_keys: self.user_api_key_service.count_for_org(&org_id).await?,
            shares: self.share_service.count_for_sites(&site_ids).await?,
            members: self.user_service.count_for_org(&org_id).await?,
            pending_invites: self
                .invite_service
                .get_all(StorableFilter::<Invite>::new_from_org_id(&org_id))
                .await?
                .iter()
                .filter(|i| i.base.expires_at > now)
                .count() as u64,
        };

        // Read only while the step is still to do: once recorded, it stays done.
        let has_tag_assignment = !org.has_setup(SetupItemDiscriminants::Tags) && {
            let tag_ids: Vec<Uuid> = self
                .tag_service
                .get_all(StorableFilter::<Tag>::new_from_org_id(&org_id).live())
                .await?
                .iter()
                .map(|t| t.id)
                .collect();
            self.tag_service.has_live_assignment(&tag_ids, None).await?
        };
        let has_application_tag_assignment = !org
            .has_setup(SetupItemDiscriminants::ApplicationTags)
            && match self.tag_service.application_group_id(&org_id).await? {
                Some(group_id) => {
                    let tag_ids: Vec<Uuid> = self
                        .tag_service
                        .live_tags_in_group(group_id)
                        .await?
                        .iter()
                        .map(|t| t.id)
                        .collect();
                    self.tag_service
                        .has_live_assignment(&tag_ids, Some(EntityDiscriminants::Service))
                        .await?
                }
                None => false,
            };

        Ok(SetupContext {
            recorded: org.base.setup.iter().copied().collect(),
            daemons,
            sites,
            targetless_credentials,
            uncovered_integrations: group_by_integration(uncovered),
            run_warnings,
            first_scan_daemon_id,
            unconfirmed_hosts,
            unconfirmed_subnets,
            counts,
            has_tag_assignment,
            has_application_tag_assignment,
            has_manual_interface_credential: self
                .credential_service
                .has_manual_interface_credential(org_id)
                .await?,
            plan: org.base.plan.clone(),
            cloud: self.deployment_type == DeploymentType::Cloud,
            can_write: entity.map(|e| {
                Resource::iter()
                    .filter(|r| e.can(*r, Action::Write))
                    .collect::<BTreeSet<_>>()
            }),
        })
    }

    /// Every type's entries in declaration order, which groups them by tier.
    pub fn list(ctx: &SetupContext) -> Vec<SetupListEntry> {
        SetupItemDiscriminants::iter()
            .flat_map(|t| t.items(ctx))
            .collect()
    }

    /// Record each Setup type the org's rows show done and `organizations.setup` doesn't hold
    /// yet, so it stays done after the rows change, and add it to `ctx`.
    pub async fn record_newly_done(
        &self,
        ctx: &mut SetupContext,
        organization_id: Uuid,
        entity: &AuthenticatedEntity,
    ) -> Result<()> {
        let newly_done: Vec<SetupItemDiscriminants> = SetupItemDiscriminants::iter()
            .filter(|t| t.recorded() && !ctx.recorded.contains(t) && t.live_done(ctx))
            .collect();
        for item_type in newly_done {
            self.organization_service
                .record_setup(
                    organization_id,
                    SetupOperation::Completed {
                        item_type,
                        subject_id: None,
                    },
                    entity.clone(),
                )
                .await?;
            ctx.recorded.insert(item_type);
        }
        Ok(())
    }

    /// The Setup list `entity` sees: Issues on what it may read, and every Setup item that
    /// applies to it, each with its state.
    pub async fn for_user(
        &self,
        org: &Organization,
        entity: &AuthenticatedEntity,
    ) -> Result<Vec<SetupListEntry>> {
        let mut ctx = self.gather(org, Some(entity)).await?;
        self.record_newly_done(&mut ctx, org.id, entity).await?;
        Ok(Self::for_reader(Self::list(&ctx), entity))
    }

    /// `entries` cut to what `entity` may read.
    fn for_reader(
        entries: Vec<SetupListEntry>,
        entity: &AuthenticatedEntity,
    ) -> Vec<SetupListEntry> {
        entries
            .into_iter()
            .filter(|entry| entry.item.readable_by(entity))
            .collect()
    }
}

/// One entry per integration, naming every host it was found on, most hosts first.
fn group_by_integration(found: Vec<FoundIntegration>) -> Vec<UncoveredIntegration> {
    let mut grouped: Vec<UncoveredIntegration> = Vec::new();
    for f in found {
        match grouped.iter_mut().find(|g| g.integration == f.integration) {
            Some(g) => {
                g.host_ids.push(f.host_id);
                if !g.site_ids.contains(&f.site_id) {
                    g.site_ids.push(f.site_id);
                }
            }
            None => grouped.push(UncoveredIntegration {
                integration: f.integration,
                host_ids: vec![f.host_id],
                site_ids: vec![f.site_id],
            }),
        }
    }
    grouped.sort_by(|a, b| b.host_ids.len().cmp(&a.host_ids.len()));
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{
        daemons::r#impl::api::DaemonHealth,
        setup::types::{SetupItem, SetupItemState},
        users::r#impl::permissions::UserOrgPermissions,
    };
    use email_address::EmailAddress;

    fn member_on(site_ids: &[Uuid]) -> AuthenticatedEntity {
        AuthenticatedEntity::User {
            user_id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            permissions: UserOrgPermissions::Member,
            access: crate::tests::access_on(UserOrgPermissions::Member, site_ids),
            email: EmailAddress::new_unchecked("member@example.test"),
            email_verified: true,
            terms_accepted_at: None,
        }
    }

    fn entry(item: SetupItem) -> SetupListEntry {
        SetupListEntry::new(item, SetupItemState::Open)
    }

    fn daemon_issue(site_id: Uuid) -> SetupItem {
        SetupItem::DaemonHealth {
            daemon_id: Uuid::new_v4(),
            name: "edge-01".to_string(),
            site_id,
            health: DaemonHealth::Outdated,
        }
    }

    /// A member sees Issues on its own sites only, and every Setup item.
    #[test]
    fn a_reader_sees_its_sites_issues_and_every_setup_item() {
        let (mine, other) = (Uuid::new_v4(), Uuid::new_v4());
        let entries = [
            daemon_issue(mine),
            daemon_issue(other),
            SetupItem::OpenTopology,
            SetupItem::Tags,
            SetupItem::InviteTeam,
            SetupItem::Share,
        ]
        .map(entry)
        .to_vec();
        let shown: Vec<SetupItemDiscriminants> =
            SetupService::for_reader(entries, &member_on(&[mine]))
                .iter()
                .map(|e| SetupItemDiscriminants::from(&e.item))
                .collect();
        assert_eq!(
            shown,
            [
                SetupItemDiscriminants::DaemonHealth,
                SetupItemDiscriminants::OpenTopology,
                SetupItemDiscriminants::Tags,
                SetupItemDiscriminants::InviteTeam,
                SetupItemDiscriminants::Share,
            ]
        );
    }
}
