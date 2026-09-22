use chrono::{DateTime, TimeZone, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ===========================================================================
// Version lifecycle registry
//
// Single source of truth for "what daemon versions are supported, and when do
// they sunset." Replaces the former constant `minimum_supported` and the dead
// hard-coded `sunset_date` string literal.
//
// One data table drives everything: `scheduled_sunsets()` — the hand-curated
// list of announced cutovers, each a (floor, absolute effective date) pair.
//
// There is deliberately NO policy: no support window, no version-count rule, no
// fixed support duration. A sunset exists because someone decided to announce it
// and wrote down one entry; nothing is inferred from release cadence. Adding a
// cutover ("everything below 1.2.0 stops working on 2027-08-01") is one literal
// appended to that list, and is the only maintenance this module needs.
//
// The enforced floor and every lifecycle stage are *derived* from that list plus
// the current date, so no date is a literal in a match arm and nothing can
// silently rot the way `2025-02-01` did.
//
// Dormancy: an entry whose `effective_on` is `None` is ignored entirely. The v1
// entry — the one cutover that genuinely derives from the launch date rather
// than from a hand-picked date — is `None` until the real launch date is baked
// in before the release build. While every entry is dormant the whole machinery
// is OFF: the enforced floor stays at the historical baseline (0.12.0), nothing
// is marked `Deprecated`/`Unsupported`, and no daemon is rejected that wasn't
// already. This lets the code ship well ahead of launch without changing any
// daemon's behavior until the date is set.
// ===========================================================================

/// The historical floor. Used while the sunset machinery is dormant, and as the
/// lower bound the derived floor never drops below.
fn baseline_floor() -> Version {
    Version::new(0, 12, 0)
}

/// This binary's own version. The derived floor is capped here: a server never
/// enforces a floor newer than itself, so it can never reject a daemon of its
/// own generation. A pinned/stale self-hosted server therefore converges to a
/// floor it can actually reason about and stops.
fn own_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION is valid semver")
}

/// Helper: a UTC timestamp at midnight for the given calendar date. Unused while
/// the only cutover derives its date from `v1_launch()`; it is how every
/// appended entry spells its absolute effective date, so it stays.
#[allow(dead_code)]
fn dt(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
        .single()
        .expect("valid date")
}

/// An announced support cutover: at `effective_on`, daemons below `floor` become
/// `Unsupported` and are rejected. Before it (but once announced) they are
/// `Deprecated` and carry the date. `effective_on: None` is the dormant sentinel.
#[derive(Clone)]
struct ScheduledFloor {
    floor: Version,
    effective_on: DateTime<Utc>,
}

/// Every announced daemon-sunset cutover, oldest floor first. Add one entry per
/// announcement — the ONLY maintenance the sunset system needs. Each carries an
/// absolute effective date; there is no automatic window or version-count policy.
fn scheduled_sunsets() -> Vec<ScheduledFloor> {
    vec![
        // ScheduledFloor {
        //     floor: Version::new(0, 17, 5),
        //     effective_on: dt(2026,11,1)
        // },

        // Future announcements are appended here with an absolute effective date —
        // do the "release + N months" math when you announce, not in code, e.g.:
        // ScheduledFloor { floor: Version::new(1, 2, 0), effective_on: dt(2027, 8, 1)) ,
    ]
}

/// The enforced support floor at `now`: daemons below it are rejected (once the
/// gate is wired in). Derived, never a literal.
///
/// While every announced cutover is dormant this is exactly `baseline_floor()`
/// (0.12.0) — the historical behavior. Otherwise the highest cutover whose date
/// has arrived applies, capped at `own_version()` so a stale server can never
/// over-enforce.
pub fn enforced_floor(now: DateTime<Utc>) -> Version {
    floor_from(&scheduled_sunsets(), now)
}

/// The enforced floor implied by `sunsets` at `now`: the highest floor whose
/// cutover has taken effect, never below `baseline_floor()` and never above
/// `own_version()`. Dormant entries (`effective_on: None`) are ignored, so an
/// all-dormant list yields exactly the baseline.
///
/// Takes the list as a parameter so tests drive the real rule with synthetic
/// cutovers instead of re-implementing it.
fn floor_from(sunsets: &[ScheduledFloor], now: DateTime<Utc>) -> Version {
    let effective_floors = sunsets
        .iter()
        .filter(|s| s.effective_on <= now)
        .map(|s| s.floor.clone());

    // Safety cap: never enforce a floor newer than this binary.
    effective_floors
        .fold(baseline_floor(), |acc, floor| acc.max(floor))
        .min(own_version())
}

/// The cutover a daemon at version `v` actually faces, if any: among the
/// announced sunsets whose floor is above `v`, the one arriving **soonest**. A
/// daemon below several floors is only ever told about the nearest deadline —
/// surfacing a later, higher cutover it is also below would misstate when it
/// stops working.
pub fn applicable_sunset(v: Option<&Version>) -> Option<(Version, DateTime<Utc>)> {
    applicable_sunset_in(&scheduled_sunsets(), v)
}

/// [`applicable_sunset`] over an explicit list (production passes
/// `scheduled_sunsets()`; tests pass synthetic cutovers).
///
/// A version-less daemon (`None`) is treated as **below** every floor: a daemon
/// that reports no version is almost always genuinely old (modern daemons always
/// report one), so it is covered by a sunset just like an old versioned daemon.
///
/// Dormant entries (`effective_on: None`) are skipped, so an all-dormant list
/// yields `None` for every version and the whole machinery stays off.
fn applicable_sunset_in(
    sunsets: &[ScheduledFloor],
    v: Option<&Version>,
) -> Option<(Version, DateTime<Utc>)> {
    sunsets
        .iter()
        .map(|s| (s.floor.clone(), s.effective_on))
        .filter(|(floor, _)| v < Some(floor))
        // Soonest deadline wins; the lower floor breaks a date tie so the result
        // never depends on the order entries happen to be written in.
        .min_by(|(a_floor, a_eff), (b_floor, b_eff)| {
            a_eff.cmp(b_eff).then_with(|| a_floor.cmp(b_floor))
        })
}

// ===========================================================================
// Capability floors (single source; permanent until their cutover cleanup)
//
// These gate specific server↔daemon features. They legitimately live near the
// policy so the rot-guard test can assert every one is ≤ the current server
// version. Credential-type floors stay in the credentials module (they own
// their domain) but are covered by the same invariant test there.
// ===========================================================================

/// Minimum daemon version required for unified discovery support.
pub fn minimum_unified_discovery() -> Version {
    Version::new(0, 15, 0)
}

/// Returns true if the daemon version supports unified discovery (>= 0.15.0).
pub fn supports_unified_discovery(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v >= &minimum_unified_discovery())
}

/// Minimum daemon version that supports the full ServerPoll flow (>= 0.14.0).
/// Absorbed here from `daemons/impl/base.rs` so the registry owns every floor.
pub fn minimum_full_server_poll() -> Version {
    Version::new(0, 14, 0)
}

/// Returns true if the daemon supports the full ServerPoll flow (>= 0.14.0).
/// A daemon without a recorded version is assumed legacy (`false`).
pub fn supports_full_server_poll(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v >= &minimum_full_server_poll())
}

/// Minimum daemon version whose status reports `ready_for_work` (>= 0.14.17). Older daemons omit
/// the field and the server reads it as `true` (its serde default on `DaemonStatus`), so their
/// "ready" says nothing about whether they are running a session.
pub fn minimum_ready_for_work() -> Version {
    Version::new(0, 14, 17)
}

/// Returns true if the daemon's `ready_for_work` can be believed (>= 0.14.17).
pub fn reports_ready_for_work(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v >= &minimum_ready_for_work())
}

/// Minimum daemon version that ships with server-provisioned identity: it is
/// installed against a pre-provisioned record bound to a 1:1 api key, learns its
/// identity from the register / first-contact handshake, and no longer relies on
/// the client-supplied X-Daemon-ID header. Below this, daemons self-register with
/// a shared network key. Coexistence is enforced by key shape (1:1 vs NULL), not
/// by this floor — this documents the boundary and gates any version-specific UI.
pub fn minimum_server_provisioned_identity() -> Version {
    Version::new(0, 17, 5)
}

/// Returns true if the daemon version supports server-provisioned identity (>= 0.17.5).
pub fn supports_server_provisioned_identity(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v >= &minimum_server_provisioned_identity())
}

/// Minimum daemon version that understands `DiscoveryType::Rescan`.
///
/// This gate is not cosmetic — it is required for correctness. `DiscoveryType`
/// has no `#[serde(other)]` fallback (see `daemon/shared/forward_compat.rs`:
/// an unknown discovery kind is not actionable and must be rejected, not
/// degraded), so an older daemon cannot deserialize a rescan request at all.
pub fn minimum_targeted_rescan() -> Version {
    Version::new(0, 17, 7)
}

/// Returns true if the daemon version supports single-host rescan (>= 0.17.7).
pub fn supports_targeted_rescan(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v >= &minimum_targeted_rescan())
}

/// Returns true if the daemon predates the Interface → IPAddress binding type rename (< 0.16.0).
/// These daemons expect `"type": "Interface"` / `"interface_id"` in binding responses.
/// Legacy cleanup: remove once minimum_supported >= 0.16.0
pub fn pre_interface_to_ip_address_rename(version: Option<&str>) -> bool {
    version
        .and_then(|v| Version::parse(v).ok())
        .is_none_or(|v| v < Version::new(0, 16, 0))
}

/// First version that ships with the corrected Docker Compose daemon-config
/// volume mount (`/root/.config/scanopy/daemon`). Releases before this shipped
/// with `/root/.config/daemon`, which silently registered a new daemon on
/// upgrade because the volume mount didn't match the daemon's actual config
/// directory.
pub fn minimum_correct_docker_volume_mount() -> Version {
    Version::new(0, 16, 1)
}

/// Returns true if the daemon version is >= the first release that shipped
/// with the corrected docker-compose.yml volume mount. Used as a proxy for
/// "this user probably has the fixed compose file" — imperfect (a user could
/// be on the latest daemon with a stale compose) but catches the common case.
pub fn has_correct_docker_volume_mount(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v >= &minimum_correct_docker_volume_mount())
}

/// The last release whose daemon submits the pre-#701 scalar LLDP/CDP shape — twelve fields flat
/// on each interface, rather than the `neighbor_candidates` array that replaced them.
///
/// A ceiling, not a floor, which is why it is not in [`capability_floors`]: every other constant
/// here says "this version and above can do X", and this one says "this version and below still
/// speaks the old shape". Keyed on the last old release rather than the first new one so it names
/// a version that exists — the first release carrying #701 is unpublished, and a floor above
/// `own_version()` would fail `capability_floors_within_server_version` on day one.
///
/// Read by `legacy_neighbor_wire_shim_still_needed`, which fails once the enforced floor passes
/// it. That is the whole end-condition: see
/// [`DiscoveryInterface`](crate::server::interfaces::r#impl::wire::DiscoveryInterface).
pub fn last_legacy_neighbor_wire() -> Version {
    Version::new(0, 17, 14)
}

/// Every capability floor this build enforces, labelled so a failure names the
/// offender. The rot-guard test asserts each is ≤ the current server version, so
/// a floor can never quietly reference a version this build doesn't know about.
///
/// Covers two sets, because a floor is a floor wherever it is declared:
///
/// * the shim floors owned by this module, and
/// * every credential type's [`minimum_daemon_version`], which gates server→daemon
///   credential dispatch in `retain_daemon_compatible`.
///
/// The second set is here because omitting it hid a live one. A credential floor
/// above `own_version()` is not inert like a too-new shim floor — it silently drops
/// that credential from every dispatch, so the integration behind it does nothing at
/// all, on every deployment of the branch, with no operator-visible signal beyond one
/// server-side warning.
///
/// [`minimum_daemon_version`]: crate::server::credentials::r#impl::types::CredentialTypeDiscriminants::minimum_daemon_version
#[cfg(test)]
fn capability_floors() -> Vec<(String, Version)> {
    use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;
    use strum::IntoEnumIterator;

    let mut floors = vec![
        (
            "minimum_unified_discovery".to_string(),
            minimum_unified_discovery(),
        ),
        (
            "minimum_full_server_poll".to_string(),
            minimum_full_server_poll(),
        ),
        (
            "minimum_server_provisioned_identity".to_string(),
            minimum_server_provisioned_identity(),
        ),
        (
            "minimum_correct_docker_volume_mount".to_string(),
            minimum_correct_docker_volume_mount(),
        ),
        (
            "minimum_targeted_rescan".to_string(),
            minimum_targeted_rescan(),
        ),
    ];

    floors.extend(CredentialTypeDiscriminants::iter().map(|disc| {
        (
            format!("{} credential", disc.display_name()),
            disc.minimum_daemon_version(),
        )
    }));

    floors
}

// ===========================================================================
// Policy + lifecycle evaluation
// ===========================================================================

/// Version policy for daemons.
///
/// `minimum_supported` is the derived enforced floor (see [`enforced_floor`]);
/// `recommended`/`latest` are this server's own version. `now` is captured at
/// construction so lifecycle evaluation and sunset dates are deterministic
/// within one request (tests pin it).
pub struct DaemonVersionPolicy {
    pub minimum_supported: Version,
    pub recommended: Version,
    pub latest: Version,
    pub now: DateTime<Utc>,
    /// Every announced cutover, captured on construction so evaluation is
    /// deterministic and testable. Empty of effective entries while dormant.
    scheduled_sunsets: Vec<ScheduledFloor>,
}

impl Default for DaemonVersionPolicy {
    fn default() -> Self {
        Self::at(Utc::now())
    }
}

impl DaemonVersionPolicy {
    /// Construct the policy as of `now` (real time in production, fixed in tests).
    pub fn at(now: DateTime<Utc>) -> Self {
        let current = own_version();
        Self {
            minimum_supported: enforced_floor(now),
            recommended: current.clone(),
            latest: current,
            now,
            scheduled_sunsets: scheduled_sunsets(),
        }
    }

    /// Test-only constructor pinning both the clock and the announced cutovers,
    /// so every lifecycle branch (Deprecated / Unsupported / version-less) can be
    /// exercised without depending on the hard-coded `v1_launch()` sentinel.
    #[cfg(test)]
    fn at_with_sunsets(now: DateTime<Utc>, sunsets: Vec<ScheduledFloor>) -> Self {
        let current = own_version();
        Self {
            minimum_supported: floor_from(&sunsets, now),
            recommended: current.clone(),
            latest: current,
            now,
            scheduled_sunsets: sunsets,
        }
    }

    pub fn evaluate(&self, version: Option<&Version>) -> DaemonVersionStatus {
        let supports_unified = supports_unified_discovery(version);
        let has_correct_mount = has_correct_docker_volume_mount(version);
        let supports_rescan = supports_targeted_rescan(version);

        let (status, warnings, sunset_date) = self.lifecycle(version);

        DaemonVersionStatus {
            version: version.map(|v| v.to_string()),
            status,
            warnings,
            sunset_date,
            supports_unified_discovery: supports_unified,
            has_correct_docker_volume_mount: has_correct_mount,
            supports_targeted_rescan: supports_rescan,
        }
    }

    /// The lifecycle stage, warnings, and sunset date for a (possibly absent)
    /// version. A version-less daemon flows through the same sunset logic as an
    /// old versioned one — it is treated as genuinely old — and only degrades to
    /// the benign `Unknown` stage when no sunset is announced (dormant).
    fn lifecycle(
        &self,
        v: Option<&Version>,
    ) -> (VersionHealthStatus, Vec<DeprecationWarning>, Option<String>) {
        let label = match v {
            Some(v) => format!("Daemon {v}"),
            None => "This daemon (no reported version)".to_string(),
        };

        if let Some((_floor, effective_on)) = applicable_sunset_in(&self.scheduled_sunsets, v) {
            let sunset_str = effective_on.format("%Y-%m-%d").to_string();
            let (status, message) = if effective_on <= self.now {
                // Past its announced cutover — unsupported and rejected.
                (
                    VersionHealthStatus::Unsupported,
                    format!(
                        "{label} is no longer supported (support ended {sunset_str}). \
                         Update to {} or later to reconnect.",
                        self.recommended
                    ),
                )
            } else {
                // Announced but still in the window — deprecated, with a date.
                (
                    VersionHealthStatus::Deprecated,
                    format!(
                        "{label} is deprecated and support ends {sunset_str}. \
                         Update to {} or later before then to avoid interruption.",
                        self.recommended
                    ),
                )
            };
            return (
                status,
                vec![DeprecationWarning {
                    message,
                    sunset_date: Some(sunset_str.clone()),
                    severity: DeprecationSeverity::Critical,
                }],
                Some(sunset_str),
            );
        }

        // No sunset covers this daemon.
        let Some(v) = v else {
            // Version-less and nothing announced (dormant) — benign Unknown.
            return (
                VersionHealthStatus::Unknown,
                vec![DeprecationWarning {
                    message: format!(
                        "Daemon version unknown. Update to {} or later.",
                        self.recommended
                    ),
                    sunset_date: None,
                    severity: DeprecationSeverity::Warning,
                }],
                None,
            );
        };

        if v < &self.minimum_supported {
            // Below the enforced floor with no cutover naming a date for it —
            // i.e. below the historical baseline, which predates every
            // announcement. Unsupported, but there is no date to quote.
            (
                VersionHealthStatus::Unsupported,
                vec![DeprecationWarning {
                    message: format!(
                        "Daemon {v} is no longer supported. Update to {} or later to reconnect.",
                        self.recommended
                    ),
                    sunset_date: None,
                    severity: DeprecationSeverity::Critical,
                }],
                None,
            )
        } else if v < &self.recommended {
            (
                VersionHealthStatus::Outdated,
                vec![DeprecationWarning {
                    message: format!(
                        "Daemon {v} is outdated. Update to {} for the latest features.",
                        self.recommended
                    ),
                    sunset_date: None,
                    severity: DeprecationSeverity::Warning,
                }],
                None,
            )
        } else {
            (VersionHealthStatus::Current, vec![], None)
        }
    }
}

/// Deprecation warning for daemon version
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeprecationWarning {
    /// What the operator needs to do, and by when.
    pub message: String,
    /// Date after which this daemon version stops being supported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sunset_date: Option<String>,
    /// How urgent the upgrade is.
    pub severity: DeprecationSeverity,
}

/// Severity level for deprecation warnings
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, Default, PartialEq, Eq)]
pub enum DeprecationSeverity {
    #[default]
    Info,
    Warning,
    Critical,
    /// Forward-compat: a severity a newer server emits that this daemon doesn't
    /// know. `#[serde(other)]` absorbs it (treated as an ordinary warning when
    /// logged) rather than failing to deserialize the whole `ServerCapabilities`.
    #[serde(other)]
    Unknown,
}

/// Daemon version status including health and any warnings
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonVersionStatus {
    /// Version the daemon reports.
    pub version: Option<String>,
    /// Whether that version is current, ageing, or out of support.
    pub status: VersionHealthStatus,
    /// Upgrade warnings that apply to this version.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<DeprecationWarning>,
    /// The date this daemon's version stops being supported, if a sunset is
    /// scheduled for it. Surfaced top-level (not only inside `warnings`) so the
    /// UI can render a countdown from the same value the email uses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sunset_date: Option<String>,
    /// Whether the daemon can run a combined discovery pass.
    #[serde(default)]
    pub supports_unified_discovery: bool,
    /// Whether a containerized daemon is mounted so it can read the Docker socket.
    #[serde(default)]
    pub has_correct_docker_volume_mount: bool,
    /// Whether this daemon can run a single-host rescan. Server-computed so the
    /// frontend never has to hardcode a version floor.
    #[serde(default)]
    pub supports_targeted_rescan: bool,
}

/// Health status for daemon versions.
///
/// Lifecycle order: `Current` → `Outdated` → `Deprecated` → `Unsupported`, with
/// `Unknown` for daemons whose version the server has no record of.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum VersionHealthStatus {
    Current,
    Outdated,
    /// A sunset date is scheduled for this version; it still works until then.
    Deprecated,
    /// Past its sunset / below the enforced floor. Rejected by the server.
    Unsupported,
    /// The server has no recorded version for this daemon.
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A policy pinned to a fixed instant for deterministic lifecycle tests.
    fn policy_at(now: DateTime<Utc>) -> DaemonVersionPolicy {
        DaemonVersionPolicy::at(now)
    }

    // --- Dormancy: with the launch anchor unset, nothing new happens. --------

    #[test]
    fn dormant_floor_is_baseline() {
        // v1_launch() is None in dev, so the floor must stay at the historical
        // baseline regardless of date.
        assert_eq!(enforced_floor(dt(2026, 7, 27)), baseline_floor());
        assert_eq!(enforced_floor(dt(2030, 1, 1)), baseline_floor());
    }

    #[test]
    fn dormant_old_daemon_is_not_deprecated() {
        // A <0.17.5 daemon while dormant is Outdated (as today), never Deprecated
        // or Unsupported — the machinery is off.
        let policy = policy_at(dt(2026, 7, 27));
        let status = policy.evaluate(Some(&Version::new(0, 16, 0)));
        assert_eq!(status.status, VersionHealthStatus::Outdated);
        assert!(status.sunset_date.is_none());
    }

    #[test]
    fn dormant_current_daemon_is_current() {
        let policy = policy_at(dt(2026, 7, 27));
        let status = policy.evaluate(Some(&own_version()));
        assert_eq!(status.status, VersionHealthStatus::Current);
        assert!(status.warnings.is_empty());
    }

    #[test]
    fn unknown_version_is_unknown_not_outdated() {
        // Dormant (no announced sunset): a version-less daemon is a benign Unknown.
        let policy = policy_at(dt(2026, 7, 27));
        let status = policy.evaluate(None);
        assert_eq!(status.status, VersionHealthStatus::Unknown);
        assert!(status.version.is_none());
    }

    #[test]
    fn announced_sunset_deprecates_old_and_versionless() {
        // Future sunset date: an old versioned daemon AND a version-less one are
        // both Deprecated with that date (a version-less daemon is treated as
        // genuinely old). A current daemon is unaffected.
        let policy = DaemonVersionPolicy::at_with_sunsets(
            dt(2026, 7, 27),
            vec![ScheduledFloor {
                floor: Version::new(0, 17, 5),
                effective_on: dt(2026, 10, 1),
            }],
        );

        let old = policy.evaluate(Some(&Version::new(0, 16, 0)));
        assert_eq!(old.status, VersionHealthStatus::Deprecated);
        assert_eq!(old.sunset_date.as_deref(), Some("2026-10-01"));

        let versionless = policy.evaluate(None);
        assert_eq!(versionless.status, VersionHealthStatus::Deprecated);
        assert_eq!(versionless.sunset_date.as_deref(), Some("2026-10-01"));

        assert_eq!(
            policy.evaluate(Some(&own_version())).status,
            VersionHealthStatus::Current
        );
    }

    #[test]
    fn passed_sunset_is_unsupported_including_versionless() {
        // Sunset date already elapsed: both an old versioned daemon and a
        // version-less one are Unsupported.
        let policy = DaemonVersionPolicy::at_with_sunsets(
            dt(2026, 11, 1),
            vec![ScheduledFloor {
                floor: Version::new(0, 17, 5),
                effective_on: dt(2026, 10, 1),
            }],
        );
        assert_eq!(
            policy.evaluate(Some(&Version::new(0, 16, 0))).status,
            VersionHealthStatus::Unsupported
        );
        assert_eq!(
            policy.evaluate(None).status,
            VersionHealthStatus::Unsupported
        );
    }

    #[test]
    fn floor_capped_at_own_version() {
        // A cutover naming a version newer than this binary is capped down —
        // a server never enforces a floor it can't reason about.
        let sunsets = vec![ScheduledFloor {
            floor: Version::new(9, 9, 9),
            effective_on: dt(2026, 1, 1),
        }];
        assert_eq!(floor_from(&sunsets, dt(2027, 1, 1)), own_version());
    }

    // --- Multiple announced cutovers -----------------------------------------

    /// Two announcements live at once: 0.17.5 already in force, 1.2.0 upcoming.
    fn two_cutovers() -> Vec<ScheduledFloor> {
        vec![
            ScheduledFloor {
                floor: Version::new(0, 17, 5),
                effective_on: dt(2026, 12, 1),
            },
            ScheduledFloor {
                floor: Version::new(1, 2, 0),
                effective_on: dt(2027, 8, 1),
            },
        ]
    }

    #[test]
    fn floor_takes_highest_cutover_already_in_force() {
        let sunsets = two_cutovers();
        // Neither in force yet.
        assert_eq!(floor_from(&sunsets, dt(2026, 11, 1)), baseline_floor());
        // Only the first — capped at own_version, which sits above it today.
        assert_eq!(
            floor_from(&sunsets, dt(2027, 1, 1)),
            Version::new(0, 17, 5).min(own_version())
        );
        // Both in force: the higher wins (still capped at this binary).
        assert_eq!(
            floor_from(&sunsets, dt(2027, 9, 1)),
            Version::new(1, 2, 0).min(own_version())
        );
    }

    #[test]
    fn daemon_is_told_the_soonest_cutover_it_faces() {
        let sunsets = two_cutovers();

        // Below both floors: told about the nearer deadline only, never the
        // later 1.2.0 one it is also below.
        let (floor, eff) = applicable_sunset_in(&sunsets, Some(&Version::new(0, 16, 0))).unwrap();
        assert_eq!(floor, Version::new(0, 17, 5));
        assert_eq!(eff, dt(2026, 12, 1));

        // Version-less counts as below every floor — same nearest deadline.
        assert_eq!(
            applicable_sunset_in(&sunsets, None).unwrap().0,
            Version::new(0, 17, 5)
        );

        // Between the floors: only the higher cutover applies.
        let (floor, eff) = applicable_sunset_in(&sunsets, Some(&Version::new(1, 0, 0))).unwrap();
        assert_eq!(floor, Version::new(1, 2, 0));
        assert_eq!(eff, dt(2027, 8, 1));

        // At or above both: nothing applies.
        assert!(applicable_sunset_in(&sunsets, Some(&Version::new(1, 2, 0))).is_none());
        assert!(applicable_sunset_in(&sunsets, Some(&Version::new(2, 0, 0))).is_none());
    }

    #[test]
    fn lifecycle_reflects_the_applicable_cutover_of_two() {
        // Between the cutovers in time: the 0.17.5 daemon is already past its
        // deadline (Unsupported) while the 1.0 daemon is Deprecated with the
        // later date — one policy, two different outcomes.
        let policy = DaemonVersionPolicy::at_with_sunsets(dt(2027, 1, 1), two_cutovers());

        assert_eq!(
            policy.evaluate(Some(&Version::new(0, 16, 0))).status,
            VersionHealthStatus::Unsupported
        );

        let mid = policy.evaluate(Some(&Version::new(1, 0, 0)));
        assert_eq!(mid.status, VersionHealthStatus::Deprecated);
        assert_eq!(mid.sunset_date.as_deref(), Some("2027-08-01"));
    }

    // --- Rot guards -----------------------------------------------------------

    #[test]
    fn capability_floors_within_server_version() {
        let own = own_version();
        for (name, floor) in capability_floors() {
            assert!(
                floor <= own,
                "capability floor {name} is {floor}, above server version {own} — it references a \
                 version this build doesn't know. A shim floor: move it or delete the shim. A \
                 credential floor: this build dispatches that credential to nobody, so bump the \
                 crate version to the release it ships in, or lower the floor."
            );
        }
    }

    /// The pre-#701 neighbour wire shim outlives its usefulness the moment no supported daemon
    /// can still send that shape. Nothing else notices when that happens: the shim keeps
    /// compiling, keeps passing, and keeps costing every reader of the ingest path an
    /// explanation. So this fails instead, on the date `scheduled_sunsets()` says, with no one
    /// having to remember.
    #[test]
    fn legacy_neighbor_wire_shim_still_needed() {
        let floor = enforced_floor(Utc::now());
        let last = last_legacy_neighbor_wire();
        assert!(
            floor <= last,
            "the enforced daemon floor is {floor}, above {last}: every supported daemon now \
             submits `neighbor_candidates`, so the pre-#701 scalar LLDP/CDP shape is dead. \
             Delete `DiscoveryInterface`'s `legacy_neighbor_evidence` field and its translation \
             (server/interfaces/impl/wire.rs), the `OutdatedDaemonFormat` warning if nothing \
             else raises it, `last_legacy_neighbor_wire`, and this test."
        );
    }
}
