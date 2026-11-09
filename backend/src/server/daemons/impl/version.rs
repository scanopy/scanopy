use chrono::{DateTime, TimeZone, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::server::release_notes::catalog::Catalog;

// ===========================================================================
// Version lifecycle registry
//
// Single source of truth for "what daemon versions are supported, and when do
// they sunset." Replaces the former constant `minimum_supported` and the dead
// hard-coded `sunset_date` string literal.
//
// One convention for every daemon version gate in the app: a gate names the
// **last published release without** the thing it gates, and a daemon has it
// when its version is above that. A gate never names a release that has not
// shipped, so no gate has to guess whether the next release is 0.18.10 or
// 0.19.0, and every value here is a git tag.
//
// One data table drives the sunset policy: `scheduled_sunsets()` — the
// hand-curated list of announced cutovers, each a (last unsupported version,
// absolute effective date) pair.
//
// There is deliberately NO policy: no support window, no version-count rule, no
// fixed support duration. A sunset exists because someone decided to announce it
// and wrote down one entry; nothing is inferred from release cadence. Adding a
// cutover ("everything up to 1.1.4 stops working on 2027-08-01") is one literal
// appended to that list, and is the only maintenance this module needs.
//
// The enforced ceiling and every lifecycle stage are *derived* from that list
// plus the current date, so no date is a literal in a match arm and nothing can
// silently rot the way `2025-02-01` did.
//
// While the list holds no entry the whole machinery is OFF: the enforced ceiling
// stays at the historical baseline (0.11.9), nothing is marked
// `Deprecated`/`Unsupported`, and no daemon is rejected that wasn't already.
// ===========================================================================

/// The historical ceiling: daemons at or below it were never supported by any server that
/// enforces versions. Used while no sunset is in force, and as the lower bound the derived
/// ceiling never drops below.
fn baseline_last_unsupported() -> Version {
    Version::new(0, 11, 9)
}

/// What `ServerCapabilities.minimum_daemon_version` carries for daemons at or below
/// [`last_without_unsupported_version_field`], which require that field and only log it.
///
/// The one floor left in this module, and on purpose: it is the value those daemons were always
/// sent, the first release after [`baseline_last_unsupported`]. It is never compared against.
pub fn legacy_minimum_daemon_version_field() -> Version {
    Version::new(0, 12, 0)
}

/// This binary's own version. A sunset naming this version or a newer one is ignored: a server
/// never rejects a daemon of its own generation, so a pinned or stale self-hosted server
/// converges to a ceiling it can actually reason about and stops.
pub(crate) fn own_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION is valid semver")
}

/// The release daemons are asked to reach: the newest one at or below `own` whose release notes
/// set `daemon_upgrade`, or `own` when no release in this binary does.
fn daemon_upgrade_target(own: &Version) -> Version {
    Catalog::embedded()
        .daemon_upgrade_target(own)
        .unwrap_or(own)
        .clone()
}

/// Helper: a UTC timestamp at midnight for the given calendar date. Unused while no cutover is
/// announced; it is how every appended entry spells its absolute effective date, so it stays.
#[allow(dead_code)]
fn dt(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
        .single()
        .expect("valid date")
}

/// An announced support cutover: at `effective_on`, daemons at or below `last_unsupported`
/// become `Unsupported` and are rejected. Before it (but once announced) they are `Deprecated`
/// and carry the date.
#[derive(Clone)]
struct ScheduledSunset {
    last_unsupported: Version,
    effective_on: DateTime<Utc>,
}

/// Every announced daemon-sunset cutover, oldest first. Add one entry per announcement — the
/// ONLY maintenance the sunset system needs. Each carries an absolute effective date; there is no
/// automatic window or version-count policy.
fn scheduled_sunsets() -> Vec<ScheduledSunset> {
    vec![
        // ScheduledSunset {
        //     last_unsupported: Version::new(0, 17, 4),
        //     effective_on: dt(2026, 11, 1),
        // },

        // Future announcements are appended here with an absolute effective date —
        // do the "release + N months" math when you announce, not in code, e.g.:
        // ScheduledSunset { last_unsupported: Version::new(1, 1, 4), effective_on: dt(2027, 8, 1) },
    ]
}

/// The enforced ceiling at `now`: daemons at or below it are rejected. Derived, never a literal.
///
/// With no cutover in force this is exactly `baseline_last_unsupported()` (0.11.9) — the
/// historical behavior. Otherwise the highest cutover whose date has arrived applies.
pub fn enforced_last_unsupported(now: DateTime<Utc>) -> Version {
    last_unsupported_from(&scheduled_sunsets(), now)
}

/// The enforced ceiling implied by `sunsets` at `now`: the highest ceiling whose cutover has
/// taken effect, never below `baseline_last_unsupported()`. A cutover naming this binary's own
/// version or a newer one is ignored rather than clamped, since clamping would have to invent the
/// release before it.
///
/// Takes the list as a parameter so tests drive the real rule with synthetic cutovers instead of
/// re-implementing it.
fn last_unsupported_from(sunsets: &[ScheduledSunset], now: DateTime<Utc>) -> Version {
    let own = own_version();
    sunsets
        .iter()
        .filter(|s| s.effective_on <= now && s.last_unsupported < own)
        .map(|s| s.last_unsupported.clone())
        .fold(baseline_last_unsupported(), |acc, ceiling| acc.max(ceiling))
}

/// The cutover a daemon at version `v` actually faces, if any: among the announced sunsets that
/// cover `v`, the one arriving **soonest**. A daemon covered by several is only ever told about
/// the nearest deadline — surfacing a later cutover it is also covered by would misstate when it
/// stops working.
pub fn applicable_sunset(v: Option<&Version>) -> Option<(Version, DateTime<Utc>)> {
    applicable_sunset_in(&scheduled_sunsets(), v)
}

/// [`applicable_sunset`] over an explicit list (production passes `scheduled_sunsets()`; tests
/// pass synthetic cutovers). Returns the cutover's ceiling and date.
///
/// A version-less daemon (`None`) is covered by every cutover: a daemon that reports no version
/// is almost always genuinely old (modern daemons always report one), so it is covered by a
/// sunset just like an old versioned daemon.
fn applicable_sunset_in(
    sunsets: &[ScheduledSunset],
    v: Option<&Version>,
) -> Option<(Version, DateTime<Utc>)> {
    sunsets
        .iter()
        .map(|s| (s.last_unsupported.clone(), s.effective_on))
        .filter(|(ceiling, _)| v.is_none_or(|v| v <= ceiling))
        // Soonest deadline wins; the lower ceiling breaks a date tie so the result never depends
        // on the order entries happen to be written in.
        .min_by(|(a_ceiling, a_eff), (b_ceiling, b_eff)| {
            a_eff.cmp(b_eff).then_with(|| a_ceiling.cmp(b_ceiling))
        })
}

// ===========================================================================
// Capability gates (single source; permanent until their cutover cleanup)
//
// Each gate names the last release without a server↔daemon feature. They live near the policy
// so the rot-guard tests can assert every one is a release this build has already passed.
// Credential-type ceilings stay in the credentials module (they own their domain) but are covered
// by the same invariant test here.
// ===========================================================================

/// Whether a reported daemon version is on a release line newer than this server's. No daemon
/// this server knows reports one, so metrics label it `unknown` rather than let a
/// client-supplied version mint new label values.
pub fn is_beyond_server_release_line(version: &Version) -> bool {
    let server = Version::parse(crate::server::openapi::SERVER_VERSION)
        .expect("CARGO_PKG_VERSION is semver");
    (version.major, version.minor) > (server.major, server.minor)
}

/// The last release without unified discovery.
pub fn last_without_unified_discovery() -> Version {
    Version::new(0, 14, 19)
}

/// Whether the daemon supports unified discovery.
pub fn supports_unified_discovery(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v > &last_without_unified_discovery())
}

/// The last release without the full ServerPoll flow. Absorbed here from `daemons/impl/base.rs`
/// so the registry owns every gate.
pub fn last_without_full_server_poll() -> Version {
    Version::new(0, 13, 6)
}

/// Whether the daemon supports the full ServerPoll flow. A daemon without a recorded version is
/// assumed legacy (`false`).
pub fn supports_full_server_poll(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v > &last_without_full_server_poll())
}

/// The last release whose status omits `ready_for_work`. Those daemons' status is read with the
/// field's serde default (`true` on `DaemonStatus`), so their "ready" says nothing about whether
/// they are running a session.
pub fn last_without_ready_for_work() -> Version {
    Version::new(0, 14, 16)
}

/// Whether the daemon's `ready_for_work` can be believed.
pub fn reports_ready_for_work(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v > &last_without_ready_for_work())
}

/// The last release that sends no version header. A daemon with no version is only known to be
/// at or below it, so it can be rejected only once the enforced ceiling covers this release; see
/// `enforce_daemon_version` in `auth/middleware/auth.rs`.
pub fn last_without_version_header() -> Version {
    Version::new(0, 14, 9)
}

/// The last release without server-provisioned identity. Later daemons are installed against a
/// pre-provisioned record bound to a 1:1 api key, learn their identity from the register /
/// first-contact handshake, and no longer rely on the client-supplied X-Daemon-ID header. Up to
/// this release, daemons self-register with a shared site key. Coexistence is enforced by key
/// shape (1:1 vs NULL), not by this gate — this documents the boundary and gates any
/// version-specific UI.
pub fn last_without_server_provisioned_identity() -> Version {
    Version::new(0, 17, 4)
}

/// Whether the daemon version supports server-provisioned identity.
pub fn supports_server_provisioned_identity(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v > &last_without_server_provisioned_identity())
}

/// The last release not installed by `scanopy-daemon install`, which registers each daemon as a
/// slot-named service the `restart` subcommand can find by daemon id. Up to this release the
/// service was set up by hand or named after the daemon, so the upgrade command names it the old
/// way.
pub fn last_without_install_engine() -> Version {
    Version::new(0, 17, 4)
}

/// Whether the daemon version was installed by the install engine.
pub fn installed_by_install_engine(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v > &last_without_install_engine())
}

/// The last release that does not understand `DiscoveryType::Rescan`.
///
/// This gate is not cosmetic — it is required for correctness. `DiscoveryType` has no
/// `#[serde(other)]` fallback (see `daemon/shared/forward_compat.rs`: an unknown discovery type is
/// not actionable and must be rejected, not degraded), so an older daemon cannot deserialize a
/// rescan request at all.
pub fn last_without_targeted_rescan() -> Version {
    Version::new(0, 17, 6)
}

/// Whether the daemon version supports single-host rescan.
pub fn supports_targeted_rescan(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v > &last_without_targeted_rescan())
}

/// The last release from before the Interface → IPAddress binding type rename. Those daemons
/// expect `"type": "Interface"` / `"interface_id"` in binding responses.
pub fn last_without_ip_address_bindings() -> Version {
    Version::new(0, 15, 6)
}

/// Whether the daemon predates the Interface → IPAddress binding rename. A daemon without a
/// recorded or parseable version is assumed to.
/// Legacy cleanup: remove once the enforced ceiling passes [`last_without_ip_address_bindings`].
pub fn pre_interface_to_ip_address_rename(version: Option<&str>) -> bool {
    version
        .and_then(|v| Version::parse(v).ok())
        .is_none_or(|v| v <= last_without_ip_address_bindings())
}

/// The last release whose daemon image writes `config.json` outside the `daemon-config` volume
/// (GH #760). Those images keep config at `/root/.config/daemon` while the composes since v0.16.1
/// mount the volume at [`DOCKER_CONFIG_DIR`], so the config lives in the container layer and the
/// upgrade's own recreate erases it. Later images symlink `/root/.config/daemon` onto the volume.
///
/// [`DOCKER_CONFIG_DIR`]: crate::daemon::shared::config::DOCKER_CONFIG_DIR
pub fn last_without_persisted_docker_config() -> Version {
    Version::new(0, 17, 21)
}

/// Whether a Docker daemon at this version keeps its config on the `daemon-config` volume. When it
/// does not, the upgrade modal has the user copy the config onto the volume before recreating. A
/// daemon without a recorded version is assumed not to.
pub fn persists_docker_config(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v > &last_without_persisted_docker_config())
}

/// The last release whose daemon submits the pre-#701 scalar LLDP/CDP shape — twelve fields flat
/// on each interface, rather than the `neighbor_candidates` array that replaced them.
///
/// Read by `legacy_neighbor_wire_shim_still_needed`, which fails once the enforced ceiling passes
/// it. That is the whole end-condition: see
/// [`DiscoveryInterface`](crate::server::interfaces::r#impl::wire::DiscoveryInterface).
pub fn last_without_neighbor_candidates() -> Version {
    Version::new(0, 17, 14)
}

/// The last release whose daemon does not report its OS on the handshake. Later daemons send
/// [`DaemonOs::current`](super::base::DaemonOs::current) on registration, startup and every
/// status, and the server enforces the recorded `os` against it.
pub fn last_without_os_reporting() -> Version {
    Version::new(0, 17, 19)
}

/// Whether a daemon at this version reports its OS on every handshake. A daemon without a recorded
/// version is assumed not to.
pub fn supports_os_reporting(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v > &last_without_os_reporting())
}

/// The last release whose daemon names the site `network_id` on the wire (and its credential
/// scope `"Network"`). Daemons at or below it are sent those names by
/// [`rewrite_for_network_wire`](crate::server::shared::legacy::rewrite_for_network_wire); later
/// daemons get `site_id`. Their requests need no gate: every daemon request is read with
/// [`rewrite_from_network_wire`](crate::server::shared::legacy::rewrite_from_network_wire).
pub fn last_without_site_wire() -> Version {
    Version::new(0, 17, 21)
}

/// Whether a daemon at this version reads `network_id` rather than `site_id`. A daemon without a
/// recorded or parseable version is assumed to.
pub fn speaks_network_wire(version: Option<&str>) -> bool {
    version
        .and_then(|v| Version::parse(v).ok())
        .is_none_or(|v| v <= last_without_site_wire())
}

/// The last release whose daemon reads a rescan's target as the single required
/// `target_host_id`, before one rescan could cover several hosts (`target_host_ids`). Those
/// daemons reject a `Rescan` without it, so
/// [`rewrite_for_single_target_rescan`](crate::server::shared::legacy::rewrite_for_single_target_rescan)
/// adds it back for them. They only scan `ips`, so a rescan covering several hosts still scans
/// every address.
///
/// Equal to this build's own version until the release carrying multi-host rescans is tagged, so
/// a build of this branch still gets the rewrite, which it reads.
pub fn last_without_multi_target_rescan() -> Version {
    Version::new(0, 18, 2)
}

/// Whether a daemon at this version needs a rescan's single `target_host_id`. A daemon without a
/// recorded or parseable version is assumed to.
pub fn reads_single_target_rescan(version: Option<&str>) -> bool {
    version
        .and_then(|v| Version::parse(v).ok())
        .is_none_or(|v| v <= last_without_multi_target_rescan())
}

/// The last release without the unified server→daemon credential wire (`credential_mappings`).
/// Credential types that shipped with it are gated here, and a daemon with no recorded version
/// counts as at most this release.
pub fn last_without_credential_wire() -> Version {
    Version::new(0, 16, 1)
}

/// The last release whose daemon requires `ServerCapabilities.minimum_daemon_version` and has no
/// `last_unsupported_daemon_version`. Read by `unsupported_version_field_shim_still_needed`.
pub fn last_without_unsupported_version_field() -> Version {
    Version::new(0, 18, 2)
}

/// The last release whose daemon never tries a credential at a container's bridge address. Later
/// daemons run integration dispatch at each running bridge container on their own host after the
/// container scan; older ones only report such an override as a target they did not scan. Read by
/// the `TargetsContainerBridgeWithCredentials` capability.
///
/// Equal to this build's own version until the release carrying it is tagged, so a build of this
/// branch is sent no bridge overrides until then.
pub fn last_without_bridge_credentials() -> Version {
    Version::new(0, 18, 2)
}

/// The last release whose daemon rescans only addresses on a subnet it has an interface on. It
/// skips any other target, and fails the rescan when none is left, so the server sends it only
/// those addresses. Later daemons scan the rest by TCP through the site subnet that holds them.
pub fn last_without_routed_rescan() -> Version {
    Version::new(0, 18, 2)
}

/// Whether a daemon at this version rescans addresses it has no interface on. A daemon without a
/// recorded version is assumed not to.
pub fn resolves_routed_rescan(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v > &last_without_routed_rescan())
}

/// The last release whose daemon reads a DaemonPoll work response as exactly two elements, the next
/// session and a cancel flag, and fails on a third. Later daemons read the session to cancel.
pub fn last_without_cancel_session_id() -> Version {
    Version::new(0, 19, 5)
}

/// Whether a daemon at this version reads the session to cancel from a work response. A daemon
/// without a recorded version is assumed not to.
pub fn reads_cancel_session_id(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v > &last_without_cancel_session_id())
}

/// The last release whose log lines do not carry the session they belong to, so a run it made cannot
/// be picked out of its log by session id.
pub fn last_without_session_log_lines() -> Version {
    Version::new(0, 17, 16)
}

/// Whether a run made by a daemon at this version can be found in its log by session id. The
/// version is the run's own, as recorded with it: the version that wrote the lines.
pub fn tags_log_lines_with_session(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v > &last_without_session_log_lines())
}

/// The gates the UI names in its copy ("installed at v0.17.4 or earlier") or checks a version
/// against, written to `daemon-version-gates.json` by `generate_ui_data_fixtures` so no version is
/// typed into the UI.
#[derive(Debug, Serialize)]
pub struct UiVersionGates {
    pub last_without_unified_discovery: String,
    pub last_without_server_provisioned_identity: String,
    pub last_without_install_engine: String,
    pub last_without_session_log_lines: String,
}

impl UiVersionGates {
    pub fn current() -> Self {
        Self {
            last_without_unified_discovery: last_without_unified_discovery().to_string(),
            last_without_server_provisioned_identity: last_without_server_provisioned_identity()
                .to_string(),
            last_without_install_engine: last_without_install_engine().to_string(),
            last_without_session_log_lines: last_without_session_log_lines().to_string(),
        }
    }
}

/// Every daemon version gate this build applies, labelled so a failure names the offender. The
/// rot-guard test asserts each is a release this build has reached, so a gate can never quietly
/// reference a version this build doesn't know about.
///
/// Covers two sets, because a gate is a gate wherever it is declared:
///
/// * the gates owned by this module, and
/// * every credential type's [`last_unsupported_daemon_version`], which gates server→daemon
///   credential dispatch.
///
/// The second set is here because omitting it hid a live one. A credential ceiling at or above
/// `own_version()` is not inert — it silently drops that credential from every dispatch, so the
/// integration behind it does nothing at all, on every deployment of the branch.
///
/// [`last_unsupported_daemon_version`]: crate::server::credentials::r#impl::types::CredentialTypeDiscriminants::last_unsupported_daemon_version
#[cfg(test)]
fn capability_gates() -> Vec<(String, Version)> {
    use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;
    use strum::IntoEnumIterator;

    let mut gates: Vec<(String, Version)> = [
        (
            "last_without_unified_discovery",
            last_without_unified_discovery(),
        ),
        (
            "last_without_full_server_poll",
            last_without_full_server_poll(),
        ),
        ("last_without_ready_for_work", last_without_ready_for_work()),
        (
            "last_without_session_log_lines",
            last_without_session_log_lines(),
        ),
        ("last_without_version_header", last_without_version_header()),
        (
            "last_without_credential_wire",
            last_without_credential_wire(),
        ),
        (
            "last_without_server_provisioned_identity",
            last_without_server_provisioned_identity(),
        ),
        ("last_without_install_engine", last_without_install_engine()),
        (
            "last_without_targeted_rescan",
            last_without_targeted_rescan(),
        ),
        (
            "last_without_ip_address_bindings",
            last_without_ip_address_bindings(),
        ),
        (
            "last_without_persisted_docker_config",
            last_without_persisted_docker_config(),
        ),
        (
            "last_without_neighbor_candidates",
            last_without_neighbor_candidates(),
        ),
        ("last_without_os_reporting", last_without_os_reporting()),
        ("last_without_site_wire", last_without_site_wire()),
        (
            "last_without_multi_target_rescan",
            last_without_multi_target_rescan(),
        ),
        (
            "last_without_unsupported_version_field",
            last_without_unsupported_version_field(),
        ),
        (
            "last_without_bridge_credentials",
            last_without_bridge_credentials(),
        ),
        ("last_without_routed_rescan", last_without_routed_rescan()),
    ]
    .into_iter()
    .map(|(name, v)| (name.to_string(), v))
    .collect();

    gates.extend(CredentialTypeDiscriminants::iter().map(|disc| {
        (
            format!("{} credential", disc.display_name()),
            disc.last_unsupported_daemon_version(),
        )
    }));

    gates
}

// ===========================================================================
// Policy + lifecycle evaluation
// ===========================================================================

/// Version policy for daemons.
///
/// `last_unsupported` is the derived enforced ceiling (see [`enforced_last_unsupported`]);
/// `recommended` is the upgrade target a daemon below it is outdated against (see
/// [`daemon_upgrade_target`]); `latest` is this server's own version, which sunset and rejection
/// messages name. `now` is captured at construction so lifecycle evaluation and sunset dates are
/// deterministic within one request (tests pin it).
pub struct DaemonVersionPolicy {
    pub last_unsupported: Version,
    pub recommended: Version,
    pub latest: Version,
    pub now: DateTime<Utc>,
    /// Every announced cutover, captured on construction so evaluation is deterministic and
    /// testable.
    scheduled_sunsets: Vec<ScheduledSunset>,
}

impl Default for DaemonVersionPolicy {
    fn default() -> Self {
        Self::at(crate::server::shared::clock::now())
    }
}

impl DaemonVersionPolicy {
    /// Construct the policy as of `now` (real time in production, fixed in tests).
    pub fn at(now: DateTime<Utc>) -> Self {
        let current = own_version();
        Self {
            last_unsupported: enforced_last_unsupported(now),
            recommended: daemon_upgrade_target(&current),
            latest: current,
            now,
            scheduled_sunsets: scheduled_sunsets(),
        }
    }

    /// Test-only constructor pinning both the clock and the announced cutovers, so every
    /// lifecycle branch (Deprecated / Unsupported / version-less) can be exercised.
    #[cfg(test)]
    fn at_with_sunsets(now: DateTime<Utc>, sunsets: Vec<ScheduledSunset>) -> Self {
        let current = own_version();
        Self {
            last_unsupported: last_unsupported_from(&sunsets, now),
            recommended: daemon_upgrade_target(&current),
            latest: current,
            now,
            scheduled_sunsets: sunsets,
        }
    }

    /// Whether a daemon reporting `version` is rejected.
    pub fn rejects(&self, version: &Version) -> bool {
        version <= &self.last_unsupported
    }

    pub fn evaluate(&self, version: Option<&Version>) -> DaemonVersionStatus {
        let supports_unified = supports_unified_discovery(version);
        let persists_config = persists_docker_config(version);
        let supports_rescan = supports_targeted_rescan(version);
        let reports_os = supports_os_reporting(version);

        let (status, warnings, sunset_date) = self.lifecycle(version);

        DaemonVersionStatus {
            version: version.map(|v| v.to_string()),
            status,
            warnings,
            sunset_date,
            supports_unified_discovery: supports_unified,
            persists_docker_config: persists_config,
            supports_targeted_rescan: supports_rescan,
            supports_os_reporting: reports_os,
        }
    }

    /// The lifecycle stage, warnings, and sunset date for a (possibly absent) version. A
    /// version-less daemon flows through the same sunset logic as an old versioned one — it is
    /// treated as genuinely old — and only degrades to the benign `Unknown` stage when no sunset
    /// is announced.
    fn lifecycle(
        &self,
        v: Option<&Version>,
    ) -> (VersionHealthStatus, Vec<DeprecationWarning>, Option<String>) {
        let label = match v {
            Some(v) => format!("Daemon {v}"),
            None => "This daemon (no reported version)".to_string(),
        };

        if let Some((_ceiling, effective_on)) = applicable_sunset_in(&self.scheduled_sunsets, v) {
            let sunset_str = effective_on.format("%Y-%m-%d").to_string();
            let (status, message) = if effective_on <= self.now {
                // Past its announced cutover — unsupported and rejected.
                (
                    VersionHealthStatus::Unsupported,
                    format!(
                        "{label} is no longer supported (support ended {sunset_str}). \
                         Update to {} or later to reconnect.",
                        self.latest
                    ),
                )
            } else {
                // Announced but still in the window — deprecated, with a date.
                (
                    VersionHealthStatus::Deprecated,
                    format!(
                        "{label} is deprecated and support ends {sunset_str}. \
                         Update to {} or later before then to avoid interruption.",
                        self.latest
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
            // Version-less and nothing announced — benign Unknown.
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

        if self.rejects(v) {
            // At or below the enforced ceiling with no cutover naming a date for it — i.e. at or
            // below the historical baseline, which predates every announcement. Unsupported, but
            // there is no date to quote.
            (
                VersionHealthStatus::Unsupported,
                vec![DeprecationWarning {
                    message: format!(
                        "Daemon {v} is no longer supported. Update to {} or later to reconnect.",
                        self.latest
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
                        "Daemon {v} is outdated. Update to {} or later.",
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
    /// Whether a Docker daemon at this version keeps its config on the `daemon-config` volume.
    /// When false, the upgrade modal has the user copy the config onto the volume first.
    #[serde(default)]
    pub persists_docker_config: bool,
    /// Whether this daemon can run a single-host rescan. Server-computed so the
    /// frontend never has to hardcode a version gate.
    #[serde(default)]
    pub supports_targeted_rescan: bool,
    /// Whether this daemon reports its OS on every handshake. When it does, the daemon's `os` is
    /// what it runs on; when it does not, `os` is the OS picked at creation, or absent.
    #[serde(default)]
    pub supports_os_reporting: bool,
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
    /// Past its sunset / at or below the enforced ceiling. Rejected by the server.
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

    fn v(major: u64, minor: u64, patch: u64) -> Version {
        Version::new(major, minor, patch)
    }

    // --- No cutover announced: nothing new happens. ---------------------------

    #[test]
    fn unannounced_ceiling_is_baseline() {
        assert_eq!(
            enforced_last_unsupported(dt(2026, 7, 27)),
            baseline_last_unsupported()
        );
        assert_eq!(
            enforced_last_unsupported(dt(2030, 1, 1)),
            baseline_last_unsupported()
        );
    }

    #[test]
    fn unannounced_old_daemon_is_not_deprecated() {
        let policy = policy_at(dt(2026, 7, 27));
        let status = policy.evaluate(Some(&v(0, 16, 0)));
        assert_eq!(status.status, VersionHealthStatus::Outdated);
        assert!(status.sunset_date.is_none());
    }

    #[test]
    fn unannounced_current_daemon_is_current() {
        let policy = policy_at(dt(2026, 7, 27));
        let status = policy.evaluate(Some(&own_version()));
        assert_eq!(status.status, VersionHealthStatus::Current);
        assert!(status.warnings.is_empty());
    }

    #[test]
    fn unknown_version_is_unknown_not_outdated() {
        let policy = policy_at(dt(2026, 7, 27));
        let status = policy.evaluate(None);
        assert_eq!(status.status, VersionHealthStatus::Unknown);
        assert!(status.version.is_none());
    }

    /// The baseline rejects exactly the releases at or below it.
    #[test]
    fn baseline_rejects_up_to_and_including_its_ceiling() {
        let policy = policy_at(dt(2026, 7, 27));
        assert!(policy.rejects(&baseline_last_unsupported()));
        assert!(!policy.rejects(&legacy_minimum_daemon_version_field()));
    }

    #[test]
    fn announced_sunset_deprecates_old_and_versionless() {
        // Future sunset date: an old versioned daemon AND a version-less one are both Deprecated
        // with that date. A current daemon is unaffected.
        let policy = DaemonVersionPolicy::at_with_sunsets(
            dt(2026, 7, 27),
            vec![ScheduledSunset {
                last_unsupported: v(0, 17, 4),
                effective_on: dt(2026, 10, 1),
            }],
        );

        let old = policy.evaluate(Some(&v(0, 16, 0)));
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
    fn passed_sunset_is_unsupported_at_its_ceiling_and_versionless() {
        let policy = DaemonVersionPolicy::at_with_sunsets(
            dt(2026, 11, 1),
            vec![ScheduledSunset {
                last_unsupported: v(0, 17, 4),
                effective_on: dt(2026, 10, 1),
            }],
        );
        assert_eq!(
            policy.evaluate(Some(&v(0, 17, 4))).status,
            VersionHealthStatus::Unsupported
        );
        assert_eq!(
            policy.evaluate(None).status,
            VersionHealthStatus::Unsupported
        );
        assert_ne!(
            policy.evaluate(Some(&v(0, 17, 5))).status,
            VersionHealthStatus::Unsupported
        );
    }

    #[test]
    fn a_cutover_at_or_above_own_version_is_ignored() {
        // A server never rejects a daemon of its own generation.
        let sunsets = vec![ScheduledSunset {
            last_unsupported: own_version(),
            effective_on: dt(2026, 1, 1),
        }];
        assert_eq!(
            last_unsupported_from(&sunsets, dt(2027, 1, 1)),
            baseline_last_unsupported()
        );
    }

    // --- Upgrade target -------------------------------------------------------

    /// A policy whose target sits below this server, as after releases without `daemon_upgrade`.
    fn policy_targeting(target: Version, sunsets: Vec<ScheduledSunset>) -> DaemonVersionPolicy {
        let mut policy = DaemonVersionPolicy::at_with_sunsets(dt(2026, 7, 27), sunsets);
        policy.recommended = target;
        policy.latest = v(99, 0, 0);
        policy
    }

    #[test]
    fn the_target_comes_from_this_binarys_releases() {
        let policy = policy_at(dt(2026, 7, 27));
        assert!(policy.recommended <= own_version());
        assert_eq!(policy.latest, own_version());
    }

    #[test]
    fn a_daemon_at_or_above_the_target_is_current() {
        let policy = policy_targeting(v(0, 19, 2), vec![]);
        for daemon in [v(0, 19, 2), v(0, 19, 3)] {
            let status = policy.evaluate(Some(&daemon));
            assert_eq!(status.status, VersionHealthStatus::Current, "{daemon}");
            assert!(status.warnings.is_empty(), "{daemon}");
        }
    }

    #[test]
    fn a_daemon_below_the_target_is_outdated_against_it() {
        let policy = policy_targeting(v(0, 19, 2), vec![]);
        let status = policy.evaluate(Some(&v(0, 19, 1)));
        assert_eq!(status.status, VersionHealthStatus::Outdated);
        assert!(status.warnings[0].message.contains("0.19.2"));
    }

    /// A target at or below a cutover is a version that cutover rejects, so sunset messages name
    /// this server's version instead.
    #[test]
    fn a_sunset_message_never_names_a_rejected_target() {
        let policy = policy_targeting(
            v(0, 17, 4),
            vec![ScheduledSunset {
                last_unsupported: v(0, 17, 4),
                effective_on: dt(2026, 10, 1),
            }],
        );
        let status = policy.evaluate(Some(&v(0, 17, 0)));
        assert_eq!(status.status, VersionHealthStatus::Deprecated);
        assert!(status.warnings[0].message.contains("99.0.0"));
        assert!(!status.warnings[0].message.contains("0.17.4"));
    }

    // --- Multiple announced cutovers -----------------------------------------

    /// Two announcements live at once: 0.17.4 already in force, 1.1.4 upcoming.
    fn two_cutovers() -> Vec<ScheduledSunset> {
        vec![
            ScheduledSunset {
                last_unsupported: v(0, 17, 4),
                effective_on: dt(2026, 12, 1),
            },
            ScheduledSunset {
                last_unsupported: v(1, 1, 4),
                effective_on: dt(2027, 8, 1),
            },
        ]
    }

    #[test]
    fn ceiling_takes_highest_cutover_already_in_force() {
        let sunsets = two_cutovers();
        assert_eq!(
            last_unsupported_from(&sunsets, dt(2026, 11, 1)),
            baseline_last_unsupported()
        );
        assert_eq!(last_unsupported_from(&sunsets, dt(2027, 1, 1)), v(0, 17, 4));
        // The 1.1.4 cutover names a version above this build, so it is ignored even once in
        // force (see `a_cutover_at_or_above_own_version_is_ignored`).
        let expected = if v(1, 1, 4) < own_version() {
            v(1, 1, 4)
        } else {
            v(0, 17, 4)
        };
        assert_eq!(last_unsupported_from(&sunsets, dt(2027, 9, 1)), expected);
    }

    #[test]
    fn daemon_is_told_the_soonest_cutover_it_faces() {
        let sunsets = two_cutovers();

        // Covered by both: told about the nearer deadline only.
        let (ceiling, eff) = applicable_sunset_in(&sunsets, Some(&v(0, 16, 0))).unwrap();
        assert_eq!(ceiling, v(0, 17, 4));
        assert_eq!(eff, dt(2026, 12, 1));

        // Version-less counts as covered by every cutover — same nearest deadline.
        assert_eq!(applicable_sunset_in(&sunsets, None).unwrap().0, v(0, 17, 4));

        // Between the ceilings: only the higher cutover applies.
        let (ceiling, eff) = applicable_sunset_in(&sunsets, Some(&v(1, 0, 0))).unwrap();
        assert_eq!(ceiling, v(1, 1, 4));
        assert_eq!(eff, dt(2027, 8, 1));

        // Above both: nothing applies.
        assert!(applicable_sunset_in(&sunsets, Some(&v(1, 1, 5))).is_none());
    }

    #[test]
    fn lifecycle_reflects_the_applicable_cutover_of_two() {
        let policy = DaemonVersionPolicy::at_with_sunsets(dt(2027, 1, 1), two_cutovers());

        assert_eq!(
            policy.evaluate(Some(&v(0, 16, 0))).status,
            VersionHealthStatus::Unsupported
        );

        let mid = policy.evaluate(Some(&v(1, 0, 0)));
        assert_eq!(mid.status, VersionHealthStatus::Deprecated);
        assert_eq!(mid.sunset_date.as_deref(), Some("2027-08-01"));
    }

    // --- Gate boundaries ----------------------------------------------------------

    /// A gate case: its name, the last version without the feature, and the predicate.
    type GateCase = (&'static str, Version, fn(Option<&Version>) -> bool);

    /// Each `Option<&Version>` predicate is false at its gate and true one release above it, and
    /// a daemon with no recorded version is treated as lacking the feature.
    #[test]
    fn every_predicate_turns_on_just_above_its_gate() {
        let cases: [GateCase; 8] = [
            (
                "unified_discovery",
                last_without_unified_discovery(),
                supports_unified_discovery,
            ),
            (
                "full_server_poll",
                last_without_full_server_poll(),
                supports_full_server_poll,
            ),
            (
                "ready_for_work",
                last_without_ready_for_work(),
                reports_ready_for_work,
            ),
            (
                "server_provisioned_identity",
                last_without_server_provisioned_identity(),
                supports_server_provisioned_identity,
            ),
            (
                "install_engine",
                last_without_install_engine(),
                installed_by_install_engine,
            ),
            (
                "targeted_rescan",
                last_without_targeted_rescan(),
                supports_targeted_rescan,
            ),
            (
                "os_reporting",
                last_without_os_reporting(),
                supports_os_reporting,
            ),
            (
                "routed_rescan",
                last_without_routed_rescan(),
                resolves_routed_rescan,
            ),
        ];
        for (name, gate, has) in cases {
            let above = v(gate.major, gate.minor, gate.patch + 1);
            assert!(!has(Some(&gate)), "{name}: on at its own gate {gate}");
            assert!(has(Some(&above)), "{name}: off above its gate {gate}");
            assert!(!has(None), "{name}: on for a daemon with no version");
        }
    }

    /// A legacy shim case: its name, its gate version, and the string-version shim.
    type ShimCase = (&'static str, Version, fn(Option<&str>) -> bool);

    /// The string-version shims are the inverse: on up to and including their gate, and for a
    /// daemon with no parseable version.
    #[test]
    fn every_legacy_shim_turns_off_just_above_its_gate() {
        let cases: [ShimCase; 3] = [
            (
                "ip_address_bindings",
                last_without_ip_address_bindings(),
                pre_interface_to_ip_address_rename,
            ),
            ("site_wire", last_without_site_wire(), speaks_network_wire),
            (
                "multi_target_rescan",
                last_without_multi_target_rescan(),
                reads_single_target_rescan,
            ),
        ];
        for (name, gate, legacy) in cases {
            let above = v(gate.major, gate.minor, gate.patch + 1).to_string();
            assert!(legacy(Some(&gate.to_string())), "{name}: off at its gate");
            assert!(!legacy(Some(&above)), "{name}: on above its gate");
            assert!(legacy(None), "{name}: off for a daemon with no version");
        }
    }

    /// The upgrade modal's pre-upgrade copy step shows for the last unpersisted release and for a
    /// daemon with no recorded version, and stops at the first release that persists.
    #[test]
    fn docker_config_copy_step_stops_at_first_persisting_release() {
        let policy = policy_at(dt(2026, 10, 5));
        let last = last_without_persisted_docker_config();
        let first_fixed = v(last.major, last.minor, last.patch + 1);

        assert!(!policy.evaluate(Some(&last)).persists_docker_config);
        assert!(!policy.evaluate(None).persists_docker_config);
        assert!(policy.evaluate(Some(&first_fixed)).persists_docker_config);
    }

    // --- Rot guards -----------------------------------------------------------

    /// A gate names the last release without its feature, so it is always a release this build
    /// has reached: its own version at most, which a gate equals only until the release that
    /// carries the feature is tagged.
    #[test]
    fn capability_gates_already_reached() {
        let own = own_version();
        for (name, gate) in capability_gates() {
            assert!(
                gate <= own,
                "{name} is {gate}, above server version {own}: a gate names the last release \
                 *without* its feature, which this build has necessarily reached. Name the \
                 release before the one that ships it."
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
        let ceiling = enforced_last_unsupported(crate::server::shared::clock::now());
        let last = last_without_neighbor_candidates();
        assert!(
            ceiling < last,
            "the enforced daemon ceiling is {ceiling}, at or above {last}: every supported daemon \
             now submits `neighbor_candidates`, so the pre-#701 scalar LLDP/CDP shape is dead. \
             Delete `DiscoveryInterface`'s `legacy_neighbor_evidence` field and its translation \
             (server/interfaces/impl/wire.rs), the `OutdatedDaemonFormat` warning if nothing \
             else raises it, `last_without_neighbor_candidates`, and this test."
        );
    }

    /// Same end-condition for the Network → Site wire rename: once no supported daemon reads
    /// `network_id`, the rewrite has no reader left.
    #[test]
    fn network_wire_shim_still_needed() {
        let ceiling = enforced_last_unsupported(crate::server::shared::clock::now());
        let last = last_without_site_wire();
        assert!(
            ceiling < last,
            "the enforced daemon ceiling is {ceiling}, at or above {last}: every supported daemon \
             now speaks `site_id`. Delete the network-wire rewrites and middlewares in \
             server/shared/legacy.rs and their callers (daemon api_client and router, \
             daemons/service/http.rs), `last_without_site_wire`, and this test. The daemon \
             config loader's use of `rewrite_from_network_wire` stays: config files written \
             before the rename live on disk until the daemon next saves."
        );
    }

    /// Same end-condition for the legacy discovery types: once every supported daemon runs
    /// Unified, nothing can be created for or run by a pre-unified daemon.
    #[test]
    fn legacy_discovery_types_still_needed() {
        let ceiling = enforced_last_unsupported(crate::server::shared::clock::now());
        let last = last_without_unified_discovery();
        assert!(
            ceiling < last,
            "the enforced daemon ceiling is {ceiling}, at or above {last}: every supported daemon \
             runs Unified discovery. Remove `DiscoveryType::{{SelfReport, Network, Docker}}`: \
             first a migration converting or deleting live and Historical rows holding them in \
             `discovery.discovery_type` and `discovery.run_type->'results'->'discovery_type'` \
             (carry data forward, prove it on a scratch DB), then the variants, `is_legacy`, \
             `migrate_discoveries_to_unified`, the daemon's `spawn_legacy_stub`, the legacy SNMP \
             dispatch, a new `Default`, their `db_enum_baseline.json` entry, the compat \
             fixtures that send them, `last_without_unified_discovery`, and this test. See the \
             DiscoveryType row in docs/db-expand-contract-ledger.md."
        );
    }

    /// Same end-condition for multi-host rescans: once no supported daemon requires a single
    /// `target_host_id`, the rewrite has no reader left.
    #[test]
    fn single_target_rescan_shim_still_needed() {
        let ceiling = enforced_last_unsupported(crate::server::shared::clock::now());
        let last = last_without_multi_target_rescan();
        assert!(
            ceiling < last,
            "the enforced daemon ceiling is {ceiling}, at or above {last}: every supported daemon \
             now reads `target_host_ids`. Delete `rewrite_for_single_target_rescan` in \
             server/shared/legacy.rs and its callers (the daemon wire middleware there and \
             daemons/service/http.rs), `last_without_multi_target_rescan`, \
             `reads_single_target_rescan`, and this test. The `target_host_id` alias on \
             `DiscoveryType::Rescan` stays while stored historical rows carry it."
        );
    }

    /// Same end-condition for `ServerCapabilities.minimum_daemon_version`: once no supported
    /// daemon requires it, it has no reader left.
    #[test]
    fn unsupported_version_field_shim_still_needed() {
        let ceiling = enforced_last_unsupported(crate::server::shared::clock::now());
        let last = last_without_unsupported_version_field();
        assert!(
            ceiling < last,
            "the enforced daemon ceiling is {ceiling}, at or above {last}: every supported daemon \
             now reads `last_unsupported_daemon_version`. Delete \
             `ServerCapabilities::minimum_daemon_version`, \
             `legacy_minimum_daemon_version_field`, `last_without_unsupported_version_field`, \
             and this test."
        );
    }
}
