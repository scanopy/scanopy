//! Links used by more than one email. A link that appears in a single email
//! stays in that email's file.
//!
//! In-app links carry the `{base_url}` and `{utm}` tokens that
//! [`Email::render_html`](super::Email::render_html) expands over the whole
//! message, so they work anywhere in a body, including as a `cta_href` value
//! handed to an email by `EmailService`.
//!
//! An org locked to Settings by a self-hosted plan lands on Settings → License
//! whichever of these it follows (`ui/src/routes/+page.svelte` reopens Settings
//! on the blocking tab), so emails to those orgs use [`SETTINGS_LICENSE`].

/// The app, on whatever tab it opens to.
pub const APP_HOME: &str = "{base_url}/?{utm}";

/// The Daemons tab.
pub const APP_DAEMONS: &str = "{base_url}/?{utm}#daemons";

/// The plan picker.
pub const PLAN_PICKER: &str = "{base_url}/?modal=billing-plan&{utm}";

/// Settings → Billing: payment method, invoices, subscription state.
pub const SETTINGS_BILLING: &str = "{base_url}/?modal=settings&tab=billing&{utm}";

/// Settings → License: the license key of an org on a self-hosted plan.
pub const SETTINGS_LICENSE: &str = "{base_url}/?modal=settings&tab=license&{utm}";

/// Settings → Email: per-category email preferences.
pub const SETTINGS_EMAIL: &str = "{base_url}/?modal=settings&tab=email&{utm}";

/// Daemon troubleshooting guide.
pub const DOCS_DAEMON_TROUBLESHOOTING: &str =
    "https://scanopy.net/docs/setting-up-daemons/troubleshooting/";
