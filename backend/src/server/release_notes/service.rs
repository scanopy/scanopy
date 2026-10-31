use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use semver::Version;
use uuid::Uuid;

use crate::server::{
    email::{
        EmailService,
        messages::{LicensedServerUpgrade, ReleaseNotes},
        rate_limit,
    },
    organizations::{r#impl::base::Organization, service::OrganizationService},
    release_notes::{
        catalog::{Catalog, MAX_SHOWN, ReleaseNote},
        storage::{ReleaseAnnouncement, ReleaseAnnouncementBase},
    },
    shared::{
        services::traits::CrudService,
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
            traits::{Storable, Storage},
        },
    },
    users::{r#impl::base::User, service::UserService},
};

/// Release notes for the modal, and the once-per-version release email.
pub struct ReleaseNotesService {
    storage: Arc<GenericPostgresStorage<ReleaseAnnouncement>>,
    user_service: Arc<UserService>,
    email_service: Option<Arc<EmailService>>,
    /// Set on Scanopy Cloud only, the one server that sells self-hosted
    /// licences: it finds the licence owners a release email goes to.
    organization_service: Option<Arc<OrganizationService>>,
    /// Paces the release email below the transport's own limit, so other
    /// emails keep part of the quota while it goes out.
    bulk_limiter: DefaultDirectRateLimiter,
    catalog: &'static Catalog,
    running: Version,
    /// `SCANOPY_HOLD_RELEASE_EMAIL`: leave an emailed release for a later start.
    hold_email: bool,
}

/// Whether a start may send the release email now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmailReadiness {
    Ready,
    NotConfigured,
    /// Held by `SCANOPY_HOLD_RELEASE_EMAIL`.
    Held,
}

/// Releases to show, newest first, and whether older ones were left out.
#[derive(Debug, PartialEq, Eq)]
pub struct Shown<'a> {
    pub notes: Vec<&'a ReleaseNote>,
    pub truncated: bool,
}

impl<'a> Shown<'a> {
    fn capped(mut notes: Vec<&'a ReleaseNote>) -> Self {
        let truncated = notes.len() > MAX_SHOWN;
        notes.truncate(MAX_SHOWN);
        Self { notes, truncated }
    }
}

/// A user's unseen releases, and whether they open the modal on their own.
#[derive(Debug, PartialEq, Eq)]
pub struct Unseen<'a> {
    pub shown: Shown<'a>,
    pub auto_open: bool,
}

/// Who a release email goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Audience {
    /// Every verified user. On Scanopy Cloud, licence owners get the licensed
    /// variant instead.
    Everyone,
    /// Only the owners of a current self-hosted licence, on Scanopy Cloud.
    LicensedOwners,
}

/// A release email to send: the running release's notes, to `audience`.
#[derive(Debug, PartialEq, Eq)]
struct Mailing<'a> {
    note: &'a ReleaseNote,
    audience: Audience,
}

/// A version this start recorded, with its email and who that email routes to.
type Claimed = (ReleaseAnnouncement, Mailing<'static>, Licensed);

/// What starting on the running version calls for.
#[derive(Debug, PartialEq, Eq)]
enum Plan<'a> {
    /// Already recorded, or older than a recorded version.
    Nothing,
    /// Record the version, and send this email when there is one.
    Record(Option<Mailing<'a>>),
}

/// Decide what starting on `running` calls for, given every recorded version.
/// `cloud` is whether this is Scanopy Cloud, the only server that emails
/// licence owners.
///
/// No recorded version and no users is a fresh install: it records its
/// version and sends nothing. No recorded version with users is an upgrade
/// from a release before this table existed, and is announced like any other.
/// An email carries the running release alone, never releases before it that
/// sent none.
fn plan<'a>(
    running: &Version,
    recorded: &[ReleaseAnnouncement],
    users_exist: bool,
    cloud: bool,
    catalog: &'a Catalog,
) -> Plan<'a> {
    let versions: Vec<Version> = recorded
        .iter()
        .filter_map(|row| Version::parse(&row.base.version).ok())
        .collect();

    if versions.iter().any(|version| version >= running) {
        return Plan::Nothing;
    }
    if versions.is_empty() && !users_exist {
        return Plan::Record(None);
    }
    let Some(note) = catalog.get(running) else {
        tracing::warn!(version = %running, "No release notes for this version; sending no release email");
        return Plan::Record(None);
    };
    let audience = if note.email {
        Some(Audience::Everyone)
    } else if note.licensed_email && cloud {
        Some(Audience::LicensedOwners)
    } else {
        None
    };
    Plan::Record(audience.map(|audience| Mailing { note, audience }))
}

/// The self-hosted licence holders on Scanopy Cloud, for routing a release
/// email. Empty on a self-hosted server.
#[derive(Debug, Default, PartialEq, Eq)]
struct Licensed {
    /// Owners of an org whose self-hosted licence is current.
    owners: HashSet<Uuid>,
    /// Orgs on a self-hosted plan, current or not. They are locked out of the
    /// cloud app, so their members get no general release email.
    locked_orgs: HashSet<Uuid>,
}

/// Of `organizations`, those on a self-hosted plan, and of those the ones
/// whose licence is still paid through `now`.
fn licensed_orgs(organizations: &[Organization], now: DateTime<Utc>) -> (HashSet<Uuid>, Vec<Uuid>) {
    let locked: Vec<&Organization> = organizations
        .iter()
        .filter(|org| org.base.plan.license_plan().is_some())
        .collect();
    let current = locked
        .iter()
        .filter(|org| org.base.license_paid_through.is_some_and(|paid| paid > now))
        .map(|org| org.id)
        .collect();
    (locked.iter().map(|org| org.id).collect(), current)
}

/// Which release email a user gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variant {
    General,
    Licensed,
}

/// Each verified user in `users` that `audience` reaches, with the email they
/// get. A licence owner gets the licensed variant, never the general email
/// too.
fn route<'u>(
    users: &'u [User],
    licensed: &Licensed,
    audience: Audience,
) -> Vec<(&'u User, Variant)> {
    users
        .iter()
        .filter(|user| user.base.email_verified)
        .filter_map(|user| {
            if licensed.owners.contains(&user.id) {
                Some((user, Variant::Licensed))
            } else if licensed.locked_orgs.contains(&user.base.organization_id) {
                None
            } else {
                (audience == Audience::Everyone).then_some((user, Variant::General))
            }
        })
        .collect()
}

/// The releases after `seen` up to `running`, or `running` alone when nothing
/// is seen yet. They open the modal when any of them asks for it, even one
/// past the cap.
fn unseen<'a>(catalog: &'a Catalog, seen: Option<&Version>, running: &Version) -> Unseen<'a> {
    let notes = match seen {
        Some(seen) => catalog.between(seen, running),
        None => catalog.get(running).into_iter().collect(),
    };
    Unseen {
        auto_open: notes.iter().any(|note| note.modal),
        shown: Shown::capped(notes),
    }
}

impl ReleaseNotesService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<ReleaseAnnouncement>>,
        user_service: Arc<UserService>,
        email_service: Option<Arc<EmailService>>,
        organization_service: Option<Arc<OrganizationService>>,
        bulk_quota: Quota,
        hold_email: bool,
    ) -> Self {
        let mut service = Self::with_catalog(
            storage,
            user_service,
            email_service,
            organization_service,
            bulk_quota,
            Catalog::embedded(),
            Version::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION is semver"),
        );
        service.hold_email = hold_email;
        service
    }

    fn with_catalog(
        storage: Arc<GenericPostgresStorage<ReleaseAnnouncement>>,
        user_service: Arc<UserService>,
        email_service: Option<Arc<EmailService>>,
        organization_service: Option<Arc<OrganizationService>>,
        bulk_quota: Quota,
        catalog: &'static Catalog,
        running: Version,
    ) -> Self {
        Self {
            storage,
            user_service,
            email_service,
            organization_service,
            bulk_limiter: RateLimiter::direct(bulk_quota),
            catalog,
            running,
            hold_email: false,
        }
    }

    pub fn running_version(&self) -> &Version {
        &self.running
    }

    /// The releases `user` hasn't dismissed, up to the running version. A user
    /// from before release notes existed sees the running release only.
    ///
    /// The modal opens on its own when any unseen release asks for it, and then
    /// lists every unseen release. Releases that don't ask for it stay unseen
    /// until it opens.
    pub fn unseen(&self, user: &User) -> Unseen<'static> {
        let seen = user
            .base
            .release_notes_seen_version
            .as_deref()
            .and_then(|v| Version::parse(v).ok());
        unseen(self.catalog, seen.as_ref(), &self.running)
    }

    /// Record that `user` has seen every release up to the running one.
    pub async fn mark_seen(&self, user: User) -> Result<()> {
        self.user_service
            .mark_release_notes_seen(user, &self.running.to_string())
            .await
    }

    /// Record the running version and send its release email, once per
    /// version across every container. Never fails startup: errors are logged.
    pub async fn announce_running_version(&self) {
        let readiness = if self.email_service.is_none() {
            EmailReadiness::NotConfigured
        } else if self.hold_email {
            EmailReadiness::Held
        } else {
            EmailReadiness::Ready
        };
        let claimed = match self.claim(readiness).await {
            Ok(claimed) => claimed,
            Err(e) => {
                tracing::error!(error = %e, version = %self.running, "Could not record the server version");
                return;
            }
        };
        if let Some((row, mailing, licensed)) = claimed
            && let Err(e) = self.send(row, mailing, licensed).await
        {
            tracing::error!(error = %e, version = %self.running, "Release email failed");
        }
    }

    /// Record the running version unless it already is. Returns the new row,
    /// the email to send and the licence holders it routes by, when this call
    /// recorded the version and the release asks for an email. Serialized by an
    /// advisory lock, so of several containers starting together exactly one
    /// records the version.
    ///
    /// A release that asks for an email is left unrecorded unless email is
    /// `Ready` and the licence holders could be read, so the first start that
    /// can send it still does.
    async fn claim(&self, readiness: EmailReadiness) -> Result<Option<Claimed>> {
        let guard = self
            .storage
            .session_lock(LockKey::ReleaseAnnouncement, DEFAULT_LOCK_TIMEOUT)
            .await?;
        let result = self.claim_locked(readiness).await;
        guard.release().await?;
        result
    }

    async fn claim_locked(&self, readiness: EmailReadiness) -> Result<Option<Claimed>> {
        let recorded = self
            .storage
            .get_all(StorableFilter::new_for_server_wide_job())
            .await?;
        let users_exist = self.user_service.any_exist().await?;
        let cloud = self.organization_service.is_some();
        let email = match plan(&self.running, &recorded, users_exist, cloud, self.catalog) {
            Plan::Nothing => return Ok(None),
            Plan::Record(email) => email,
        };
        let mut licensed = Licensed::default();
        if email.is_some() {
            match readiness {
                EmailReadiness::Ready => {}
                EmailReadiness::NotConfigured => {
                    tracing::warn!(
                        version = %self.running,
                        "Email is not configured; the release email waits for a start with email configured"
                    );
                    return Ok(None);
                }
                EmailReadiness::Held => {
                    tracing::info!(
                        version = %self.running,
                        "Release email held by SCANOPY_HOLD_RELEASE_EMAIL; it sends on the first start without it"
                    );
                    return Ok(None);
                }
            }
            licensed = self.licensed().await?;
        }
        let row = self
            .storage
            .create(&ReleaseAnnouncement::new(ReleaseAnnouncementBase {
                version: self.running.to_string(),
                email_sent_at: None,
            }))
            .await?;
        tracing::info!(version = %self.running, email = email.is_some(), "Recorded a new server version");
        Ok(email.map(|mailing| (row, mailing, licensed)))
    }

    /// The self-hosted licence holders, read on Scanopy Cloud only.
    async fn licensed(&self) -> Result<Licensed> {
        let Some(organization_service) = &self.organization_service else {
            return Ok(Licensed::default());
        };
        let organizations = organization_service
            .get_all(StorableFilter::new_for_server_wide_job())
            .await?;
        let (locked_orgs, current) = licensed_orgs(&organizations, Utc::now());
        let mut owners = HashSet::new();
        for organization_id in current {
            owners.extend(
                self.user_service
                    .get_organization_owners(&organization_id)
                    .await?
                    .into_iter()
                    .map(|owner| owner.id),
            );
        }
        Ok(Licensed {
            owners,
            locked_orgs,
        })
    }

    /// Send `mailing` to the users it reaches, paced by the bulk limiter. A
    /// relay that keeps refusing for now (a daily quota, an outage) stops the
    /// run rather than failing every remaining recipient the same way.
    async fn send(
        &self,
        mut row: ReleaseAnnouncement,
        mailing: Mailing<'static>,
        licensed: Licensed,
    ) -> Result<()> {
        let Some(email_service) = &self.email_service else {
            tracing::info!("Email is not configured; the release email was not sent");
            return Ok(());
        };
        let users = self
            .user_service
            .get_all(StorableFilter::new_for_server_wide_job())
            .await?;
        let recipients = route(&users, &licensed, mailing.audience);
        let general = ReleaseNotes { note: mailing.note };
        let upgrade = LicensedServerUpgrade { note: mailing.note };

        let (mut processed, mut failed) = (0usize, 0usize);
        // Each send is an `email_sent` event and each failure has its own line, so the run
        // writes no summary.
        for (user, variant) in &recipients {
            self.bulk_limiter.until_ready().await;
            let to = user.base.email.clone();
            let sent = match variant {
                Variant::General => email_service.send_release_notes(to, &general).await,
                Variant::Licensed => {
                    email_service
                        .send_licensed_server_upgrade(to, &upgrade)
                        .await
                }
            };
            match sent {
                Ok(()) => processed += 1,
                Err(e) if rate_limit::is_transient(&e) => {
                    tracing::error!(
                        error = %e,
                        processed,
                        unsent = recipients.len() - processed - failed,
                        "The relay keeps refusing the release email; stopping"
                    );
                    break;
                }
                Err(e) => {
                    failed += 1;
                    tracing::warn!(user_id = %user.id, error = %e, "Release email not sent");
                }
            }
        }
        row.base.email_sent_at = Some(Utc::now());
        row.updated_at = Utc::now();
        self.storage.update(&mut row).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::release_notes::catalog::tests::{note_file, note_file_with};
    use crate::server::shared::storage::factory::StorageFactory;
    use crate::server::users::r#impl::base::UserBase;
    use crate::tests::test_storage;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn row(version: &str, emailed: bool) -> ReleaseAnnouncement {
        ReleaseAnnouncement::new(ReleaseAnnouncementBase {
            version: version.into(),
            email_sent_at: emailed.then(Utc::now),
        })
    }

    /// Releases 0.18.0 through 0.18.7 with both emails off, except 0.18.5,
    /// which emails licence owners only, and 0.19.0, which emails everyone.
    fn catalog() -> &'static Catalog {
        let mut files: Vec<_> = (0..=7)
            .map(|p| match p {
                5 => note_file_with("0.18.5", "email: false\nlicensed_email: true"),
                p => note_file(&format!("0.18.{p}"), false),
            })
            .collect();
        files.push(note_file("0.19.0", true));
        Box::leak(Box::new(Catalog::from_files(files)))
    }

    /// The version and audience `plan` emails, `None` when it records without
    /// one, or a panic when it records nothing.
    fn emailed(plan: Plan<'_>) -> Option<(String, Audience)> {
        match plan {
            Plan::Record(mailing) => mailing.map(|m| (m.note.version.to_string(), m.audience)),
            Plan::Nothing => panic!("expected the version to be recorded"),
        }
    }

    #[test]
    fn a_recorded_or_older_version_does_nothing() {
        let catalog = catalog();
        for cloud in [false, true] {
            assert_eq!(
                plan(&v("0.19.0"), &[row("0.19.0", true)], true, cloud, catalog),
                Plan::Nothing
            );
            assert_eq!(
                plan(&v("0.18.7"), &[row("0.19.0", true)], true, cloud, catalog),
                Plan::Nothing
            );
        }
    }

    #[test]
    fn a_fresh_install_records_without_email() {
        assert_eq!(
            plan(&v("0.19.0"), &[], false, true, catalog()),
            Plan::Record(None)
        );
    }

    #[test]
    fn the_first_upgrade_after_this_feature_announces_the_running_release() {
        assert_eq!(
            emailed(plan(&v("0.19.0"), &[], true, false, catalog())),
            Some(("0.19.0".into(), Audience::Everyone))
        );
    }

    /// Each combination of the two flags, on Scanopy Cloud and self-hosted.
    /// Only Cloud emails licence owners, and an email carries the running
    /// release alone, however many releases since the last one sent none.
    #[test]
    fn the_flags_pick_the_audience() {
        let catalog = catalog();
        let recorded = [
            row("0.18.0", true),
            row("0.18.2", false),
            row("0.18.4", false),
        ];
        let cases = [
            ("0.19.0", true, Some(Audience::Everyone)),
            ("0.19.0", false, Some(Audience::Everyone)),
            ("0.18.5", true, Some(Audience::LicensedOwners)),
            ("0.18.5", false, None),
            ("0.18.6", true, None),
            ("0.18.6", false, None),
        ];
        for (running, cloud, audience) in cases {
            assert_eq!(
                emailed(plan(&v(running), &recorded, true, cloud, catalog)),
                audience.map(|audience| (running.to_string(), audience)),
                "{running} on cloud={cloud}"
            );
        }
    }

    fn user_in(organization_id: Uuid, verified: bool) -> User {
        let mut user = crate::tests::user(&organization_id);
        user.base.email =
            email_address::EmailAddress::new_unchecked(format!("{}@example.com", user.id));
        user.base.email_verified = verified;
        user
    }

    /// A cloud user, a licence owner and another member of the owner's org,
    /// and an unverified licence owner, routed to `audience`.
    fn routed(audience: Audience) -> Vec<(&'static str, Variant)> {
        let (cloud_org, licensed_org) = (Uuid::new_v4(), Uuid::new_v4());
        let users = vec![
            ("cloud user", user_in(cloud_org, true)),
            ("owner", user_in(licensed_org, true)),
            ("member", user_in(licensed_org, true)),
            ("unverified owner", user_in(licensed_org, false)),
        ];
        let licensed = Licensed {
            owners: [&users[1].1, &users[3].1].map(|user| user.id).into(),
            locked_orgs: [licensed_org].into(),
        };
        let (names, users): (Vec<_>, Vec<_>) = users.into_iter().unzip();
        route(&users, &licensed, audience)
            .into_iter()
            .map(|(user, variant)| {
                let index = users.iter().position(|u| u.id == user.id).unwrap();
                (names[index], variant)
            })
            .collect()
    }

    #[test]
    fn licence_owners_get_the_licensed_variant_in_place_of_the_general_email() {
        assert_eq!(
            routed(Audience::Everyone),
            [
                ("cloud user", Variant::General),
                ("owner", Variant::Licensed)
            ]
        );
    }

    #[test]
    fn a_licensed_only_release_reaches_licence_owners_only() {
        assert_eq!(
            routed(Audience::LicensedOwners),
            [("owner", Variant::Licensed)]
        );
    }

    /// Every org on a self-hosted plan is locked out of the cloud app, and
    /// those with a current licence, online or air-gapped, have owners to email.
    #[test]
    fn licensed_orgs_are_self_hosted_plans_and_current_ones_are_paid_through_now() {
        use crate::server::billing::types::base::{BillingPlan, PlanConfig};
        use crate::server::license::types::LicenseKeyType;

        let now = Utc::now();
        let org = |plan: BillingPlan, paid_through: Option<DateTime<Utc>>, key| {
            let mut org = crate::tests::organization();
            org.base.plan = plan;
            org.base.license_paid_through = paid_through;
            org.base.license_key_type = key;
            org
        };
        let later = Some(now + chrono::Duration::days(30));
        let earlier = Some(now - chrono::Duration::days(1));
        let online = org(
            BillingPlan::SelfHostedStandard(PlanConfig::default()),
            later,
            None,
        );
        let offline = org(
            BillingPlan::SelfHostedPlus(PlanConfig::default()),
            later,
            Some(LicenseKeyType::Offline),
        );
        let lapsed = org(
            BillingPlan::SelfHostedStandard(PlanConfig::default()),
            earlier,
            None,
        );
        let hand_issued = org(
            BillingPlan::CommercialSelfHosted(PlanConfig::default()),
            later,
            None,
        );
        let cloud = org(BillingPlan::Pro(PlanConfig::default()), later, None);

        let (locked, current) = licensed_orgs(
            &[
                online.clone(),
                offline.clone(),
                lapsed.clone(),
                hand_issued,
                cloud,
            ],
            now,
        );
        assert_eq!(locked, [online.id, offline.id, lapsed.id].into());
        assert_eq!(current, [online.id, offline.id]);
    }

    fn service(storage: &StorageFactory, running: &str, cloud: bool) -> ReleaseNotesService {
        let event_bus = Arc::new(crate::server::shared::events::bus::EventBus::new());
        let user_service = Arc::new(UserService::new(
            storage.users.clone(),
            Arc::new(
                crate::server::site_assignments::service::SiteAssignmentService::new(
                    storage.site_assignments.clone(),
                ),
            ),
            event_bus.clone(),
        ));
        let organization_service = cloud.then(|| {
            Arc::new(OrganizationService::new(
                storage.organizations.clone(),
                event_bus,
            ))
        });
        ReleaseNotesService::with_catalog(
            storage.release_announcements.clone(),
            user_service,
            None,
            organization_service,
            rate_limit::per_second(1000),
            catalog(),
            v(running),
        )
    }

    /// Containers starting on the same new version at once: exactly one
    /// records it and goes on to send, and a restart afterwards records nothing.
    #[tokio::test]
    async fn concurrent_starts_claim_the_version_once() {
        let (storage, _container) = test_storage().await;
        storage
            .release_announcements
            .create(&row("0.18.0", true))
            .await
            .unwrap();

        let services: Vec<_> = (0..8).map(|_| service(&storage, "0.19.0", false)).collect();
        let claims =
            futures::future::join_all(services.iter().map(|s| s.claim(EmailReadiness::Ready)))
                .await;
        let claimed = claims
            .into_iter()
            .map(|claim| claim.unwrap())
            .filter(Option::is_some)
            .count();
        assert_eq!(claimed, 1);

        let recorded = storage
            .release_announcements
            .get_all(StorableFilter::new_for_server_wide_job())
            .await
            .unwrap();
        assert_eq!(recorded.len(), 2);

        assert!(
            service(&storage, "0.19.0", false)
                .claim(EmailReadiness::Ready)
                .await
                .unwrap()
                .is_none()
        );
    }

    /// A start that can't send yet, without email configured or with the email
    /// held, leaves an emailed release pending, so the next start that can send
    /// it does instead of finding it recorded.
    #[tokio::test]
    async fn an_emailed_release_waits_for_a_start_that_can_send_it() {
        let (storage, _container) = test_storage().await;
        storage
            .release_announcements
            .create(&row("0.18.0", true))
            .await
            .unwrap();
        let service = service(&storage, "0.19.0", false);

        for readiness in [EmailReadiness::NotConfigured, EmailReadiness::Held] {
            assert!(service.claim(readiness).await.unwrap().is_none());
        }
        let recorded = storage
            .release_announcements
            .get_all(StorableFilter::new_for_server_wide_job())
            .await
            .unwrap();
        assert_eq!(recorded.len(), 1, "0.19.0 stays unrecorded");

        assert!(
            service
                .claim(EmailReadiness::Ready)
                .await
                .unwrap()
                .is_some()
        );
    }

    /// On Cloud, a release that emails licence owners only claims them once:
    /// the start that records it gets the licence org's owner, and a restart
    /// on the same version gets nothing.
    #[tokio::test]
    async fn a_licensed_only_release_claims_its_owners_once() {
        use crate::server::billing::types::base::{BillingPlan, PlanConfig};
        use crate::server::users::r#impl::permissions::UserOrgPermissions;

        let (storage, _container) = test_storage().await;
        storage
            .release_announcements
            .create(&row("0.18.4", false))
            .await
            .unwrap();
        let mut organization = crate::tests::organization();
        organization.base.plan = BillingPlan::SelfHostedStandard(PlanConfig::default());
        organization.base.license_paid_through = Some(Utc::now() + chrono::Duration::days(30));
        let organization = storage.organizations.create(&organization).await.unwrap();
        let mut owner = user_in(organization.id, true);
        owner.base.permissions = UserOrgPermissions::Owner;
        let owner = storage.users.create(&owner).await.unwrap();
        let mut member = user_in(organization.id, true);
        member.base.permissions = UserOrgPermissions::Member;
        let member = storage.users.create(&member).await.unwrap();

        let (_, mailing, licensed) = service(&storage, "0.18.5", true)
            .claim(EmailReadiness::Ready)
            .await
            .unwrap()
            .expect("the first start claims the release");
        assert_eq!(mailing.audience, Audience::LicensedOwners);
        // The test database's seed holds other orgs, so only this org's users are checked.
        assert!(licensed.owners.contains(&owner.id));
        assert!(!licensed.owners.contains(&member.id));
        assert!(licensed.locked_orgs.contains(&organization.id));

        assert!(
            service(&storage, "0.18.5", true)
                .claim(EmailReadiness::Ready)
                .await
                .unwrap()
                .is_none()
        );
    }

    /// Dismissing advances the user to the running version, and the stored
    /// value survives the reload a second device would do.
    #[tokio::test]
    async fn unseen_spans_the_seen_version_to_the_running_one() {
        let (storage, _container) = test_storage().await;
        let service = service(&storage, "0.18.6", false);
        let versions = |user: &User| -> Vec<String> {
            let unseen = service.unseen(user);
            unseen
                .shown
                .notes
                .iter()
                .map(|n| n.version.to_string())
                .collect()
        };

        let mut user = User::new(UserBase::default());
        user.base.release_notes_seen_version = Some("0.18.4".into());
        assert_eq!(versions(&user), ["0.18.6", "0.18.5"]);

        user.base.release_notes_seen_version = None;
        assert_eq!(
            versions(&user),
            ["0.18.6"],
            "a user from before release notes"
        );

        let organization = storage
            .organizations
            .create(&crate::tests::organization())
            .await
            .unwrap();
        let user = storage
            .users
            .create(&crate::tests::user(&organization.id))
            .await
            .unwrap();
        service.mark_seen(user.clone()).await.unwrap();
        let reloaded = storage.users.get_by_id(&user.id).await.unwrap().unwrap();
        assert!(versions(&reloaded).is_empty());
    }

    /// `auto_open` and the versions listed for a user who last saw `seen`
    /// (`None`: from before release notes), over releases 0.18.0 through
    /// 0.18.8 running 0.18.8, with `modal` on only for `with_modal`.
    fn unseen_from(with_modal: &[&str], seen: Option<&str>) -> (bool, Vec<String>) {
        let files = (0..=8).map(|p| {
            let version = format!("0.18.{p}");
            let modal = with_modal.contains(&version.as_str());
            note_file_with(&version, &format!("email: false\nmodal: {modal}"))
        });
        let catalog = Catalog::from_files(files);
        let unseen = unseen(&catalog, seen.map(v).as_ref(), &v("0.18.8"));
        let versions = unseen
            .shown
            .notes
            .iter()
            .map(|n| n.version.to_string())
            .collect();
        (unseen.auto_open, versions)
    }

    #[test]
    fn a_single_release_opens_the_modal_only_when_it_asks_to() {
        for seen in [Some("0.18.7"), None] {
            let (auto_open, versions) = unseen_from(&["0.18.8"], seen);
            assert!(auto_open);
            assert_eq!(versions, ["0.18.8"]);

            let (auto_open, versions) = unseen_from(&[], seen);
            assert!(!auto_open);
            assert_eq!(versions, ["0.18.8"], "the notes are still there to open");
        }
    }

    /// Across missed releases the modal opens when any of them asks for it,
    /// even one past the cap, and lists the ones that didn't ask too.
    #[test]
    fn missed_releases_open_the_modal_when_any_asks_to() {
        let (auto_open, versions) = unseen_from(&[], Some("0.18.5"));
        assert!(!auto_open);
        assert_eq!(versions, ["0.18.8", "0.18.7", "0.18.6"]);

        let (auto_open, versions) = unseen_from(&["0.18.6"], Some("0.18.5"));
        assert!(auto_open);
        assert_eq!(versions, ["0.18.8", "0.18.7", "0.18.6"]);

        let (auto_open, _) = unseen_from(&["0.18.1"], Some("0.18.0"));
        assert!(
            auto_open,
            "0.18.1 is past the cap but still asks for the modal"
        );
    }

    /// A user created now starts at the running release, so the release it
    /// signed up on never shows as new.
    #[test]
    fn a_new_user_has_nothing_unseen() {
        let running = v(env!("CARGO_PKG_VERSION"));
        let catalog: &'static Catalog = Box::leak(Box::new(Catalog::from_files([note_file(
            &running.to_string(),
            false,
        )])));
        let user = User::new(UserBase::new_password(
            email_address::EmailAddress::new_unchecked("new@example.com"),
            true,
            "hash".into(),
            uuid::Uuid::new_v4(),
            crate::server::users::r#impl::permissions::UserOrgPermissions::Member,
            vec![],
            None,
        ));
        let seen =
            Version::parse(user.base.release_notes_seen_version.as_deref().unwrap()).unwrap();
        assert!(catalog.between(&seen, &running).is_empty());
    }
}
