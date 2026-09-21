use super::{Body, Content, Email, EmailCategory, EmailPreference, PausableCategory, links};

/// Notifies an organization that one or more of its daemons run a version whose
/// support ends on a scheduled sunset date, and must be upgraded before then to
/// keep connecting and running discovery.
///
/// One email covers every affected daemon in the org (aggregated) — a multi-
/// daemon org gets a single coherent message, not one email per daemon.
pub struct DaemonSunset<'a> {
    /// Names of the affected daemons in this org.
    pub daemon_names: &'a [&'a str],
    /// The date support ends, formatted for display (e.g. "November 1, 2026").
    pub sunset_date: &'a str,
}

impl Email for DaemonSunset<'_> {
    fn subject(&self) -> String {
        "Action required: update your Scanopy daemon".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Daemon
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Pausable(PausableCategory::DaemonAlerts)
    }

    fn campaign(&self) -> &'static str {
        "daemon_sunset"
    }

    fn body_html(&self) -> String {
        let daemon_items = self
            .daemon_names
            .iter()
            .map(|name| {
                format!("                                <li><strong>{name}</strong></li>\n")
            })
            .collect::<String>();

        Body::new()
            .content(
                Content::new()
                    .heading("Update Your Daemon")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Support for the version running on the following daemon(s) ends on <strong>{}</strong>. After that date they will no longer connect to Scanopy or run network discovery until they are updated:",
                        self.sunset_date
                    ))
                    .raw(&format!(
r#"                            <ul style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
{daemon_items}                            </ul>
"#,
                    ))
                    .paragraph("Updating takes a couple of minutes and preserves all of your existing configuration. Update each daemon to the latest version from the Scanopy UI under Discover &gt; Daemons.")
                    .paragraph("Update before the date above to avoid any interruption to your scheduled discoveries."),
            )
            .cta(links::APP_DAEMONS, "Update Daemons")
            .render()
    }
}
