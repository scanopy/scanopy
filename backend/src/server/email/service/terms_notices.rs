//! Terms-update notices: Scanopy Cloud emails each user notice of a change to
//! its terms before the change takes effect, once per update. Who was sent
//! what is kept by the SES event archive, from the message tags every cloud
//! send carries; `users.terms_notice_version` only stops a second send.

use chrono::NaiveDate;
use governor::DefaultDirectRateLimiter;

use super::*;
use crate::server::{
    email::{messages::TermsUpdate, rate_limit},
    release_notes::terms::{MIN_NOTICE_DAYS, TermsCatalog, TermsUpdate as Update},
    shared::storage::lock::{DEFAULT_LOCK_TIMEOUT, LockError, LockKey},
};

/// Whether `user` is due notice of `update`: their address is verified, they
/// signed up before it takes effect, and they haven't been sent it or a later
/// one.
fn is_due(user: &User, update: &Update) -> bool {
    user.base.email_verified
        && user.created_at.date_naive() < update.effective
        && user
            .base
            .terms_notice_version
            .is_none_or(|sent| sent < update.effective)
}

/// The updates sending on `today` that someone is due, oldest first, each
/// with the users due it.
fn plan<'a, 'u>(
    today: NaiveDate,
    catalog: &'a TermsCatalog,
    users: &'u [User],
) -> Vec<(&'a Update, Vec<&'u User>)> {
    catalog
        .sending_on(today)
        .into_iter()
        .map(|update| {
            let due: Vec<&User> = users.iter().filter(|user| is_due(user, update)).collect();
            (update, due)
        })
        .filter(|(_, due)| !due.is_empty())
        .collect()
}

impl EmailService {
    /// Email every user the terms notices due to them today. Scanopy Cloud
    /// only: a self-hosted server's users have no agreement with Scanopy LLC.
    /// `bulk` paces the sends below the transport's own limit. Containers
    /// contend on an advisory lock, so one sweep runs at a time.
    pub async fn send_terms_notices(&self, bulk: &DefaultDirectRateLimiter) -> Result<()> {
        self.send_terms_notices_from(
            TermsCatalog::embedded(),
            crate::server::shared::clock::now().date_naive(),
            bulk,
        )
        .await
    }

    pub(crate) async fn send_terms_notices_from(
        &self,
        catalog: &TermsCatalog,
        today: NaiveDate,
        bulk: &DefaultDirectRateLimiter,
    ) -> Result<()> {
        if self.deployment_type.is_self_hosted() {
            return Ok(());
        }
        let sending: Vec<NaiveDate> = catalog
            .sending_on(today)
            .iter()
            .map(|update| update.effective)
            .collect();
        // Names the updates this build carries, so a file that never reached
        // the binary shows here instead of as silence.
        tracing::info!(
            versions = ?catalog.versions(),
            ?sending,
            "Terms notice sweep"
        );
        if sending.is_empty() {
            return Ok(());
        }
        let guard = match self
            .user_service
            .storage()
            .session_lock(LockKey::TermsNotice, DEFAULT_LOCK_TIMEOUT)
            .await
        {
            Ok(guard) => guard,
            // Another container is part way through the same sweep.
            Err(LockError::Timeout(..)) => {
                tracing::info!("Another server is sending the terms notices");
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };
        let result = self.send_terms_notices_locked(catalog, today, bulk).await;
        guard.release().await?;
        result
    }

    /// Send, then record each user's notice as it goes out. A relay that keeps
    /// refusing (a daily quota, an outage) stops the sweep, and the next one
    /// picks up whoever is left. A crash between a send and its record can
    /// repeat that one email.
    async fn send_terms_notices_locked(
        &self,
        catalog: &TermsCatalog,
        today: NaiveDate,
        bulk: &DefaultDirectRateLimiter,
    ) -> Result<()> {
        let users = self
            .user_service
            .get_all(StorableFilter::<User>::new_for_server_wide_job())
            .await?;
        for (update, due) in plan(today, catalog, &users) {
            let version = update.effective;
            // On an update's first send, short notice means the release
            // carrying it deployed later than its file allowed for. Later
            // sends reach users who signed up during the notice period, and
            // those always get what is left of it.
            let first_send = !users
                .iter()
                .any(|user| user.base.terms_notice_version >= Some(version));
            let notice_days = (version - today).num_days();
            if first_send && notice_days < MIN_NOTICE_DAYS as i64 {
                tracing::error!(
                    %version,
                    notice_days,
                    users = due.len(),
                    "Sending a terms notice with less than {MIN_NOTICE_DAYS} days' notice"
                );
            }
            tracing::info!(%version, recipients = due.len(), "Sending a terms notice");
            let email = TermsUpdate { update };
            let (mut sent, mut failed) = (0usize, 0usize);
            for user in &due {
                bulk.until_ready().await;
                match self.dispatch(user.base.email.clone(), &email).await {
                    Ok(()) => {
                        sent += 1;
                        self.user_service
                            .mark_terms_notified(&user.id, version)
                            .await?;
                    }
                    Err(e) if rate_limit::is_transient(&e) => {
                        tracing::error!(
                            error = %e,
                            %version,
                            sent,
                            unsent = due.len() - sent - failed,
                            "The relay keeps refusing the terms notice; the next sweep sends the rest"
                        );
                        return Ok(());
                    }
                    Err(e) => {
                        failed += 1;
                        tracing::warn!(user_id = %user.id, error = %e, "Terms notice not sent");
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::release_notes::terms::tests::{d, terms_file};
    use crate::server::shared::storage::traits::Storable;
    use crate::server::users::r#impl::base::UserBase;

    fn catalog() -> TermsCatalog {
        TermsCatalog::from_files([
            terms_file("2026-12-01", "2026-10-20"),
            terms_file("2027-02-01", "2026-11-10"),
        ])
    }

    fn user(created: &str, verified: bool, notified: Option<&str>) -> User {
        let mut user = User::new(UserBase {
            email_verified: verified,
            terms_notice_version: notified.map(d),
            ..UserBase::default()
        });
        user.created_at = d(created).and_hms_opt(12, 0, 0).unwrap().and_utc();
        user
    }

    fn due_versions(today: &str, users: &[User]) -> Vec<(String, usize)> {
        plan(d(today), &catalog(), users)
            .into_iter()
            .map(|(update, due)| (update.effective.to_string(), due.len()))
            .collect()
    }

    #[test]
    fn only_verified_users_from_before_the_update_who_lack_it_are_due() {
        let users = [
            user("2026-01-01", true, None),
            user("2026-01-01", false, None),
            user("2026-12-01", true, None),
            user("2026-01-01", true, Some("2026-12-01")),
        ];
        assert_eq!(
            due_versions("2026-10-25", &users),
            [("2026-12-01".into(), 1)]
        );
    }

    #[test]
    fn nothing_is_due_outside_an_updates_window() {
        let users = [user("2026-01-01", true, None)];
        assert!(due_versions("2026-10-19", &users).is_empty());
        assert!(due_versions("2027-03-03", &users).is_empty());
    }

    /// Two updates sending at once go out oldest first, and a user already
    /// sent the later one is not sent the earlier one.
    #[test]
    fn overlapping_updates_go_oldest_first() {
        let users = [
            user("2026-01-01", true, None),
            user("2026-01-01", true, Some("2027-02-01")),
        ];
        assert_eq!(
            due_versions("2026-11-15", &users),
            [("2026-12-01".into(), 1), ("2027-02-01".into(), 1)]
        );
    }
}
