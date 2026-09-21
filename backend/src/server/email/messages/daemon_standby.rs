use super::{Body, Content, Email, EmailCategory, EmailPreference, PausableCategory, links};

/// Notifies that a daemon was put on standby after 30 days without a discovery.
pub struct DaemonStandby<'a> {
    pub daemon_name: &'a str,
    pub network_name: &'a str,
}

impl Email for DaemonStandby<'_> {
    fn subject(&self) -> String {
        "Your Daemon Has Been Put on Standby".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Daemon
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Pausable(PausableCategory::DaemonAlerts)
    }

    fn campaign(&self) -> &'static str {
        "daemon_standby"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Daemon on Standby")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        r#"Your daemon <strong>{}</strong> on <strong>{}</strong> has been placed on <a href="https://scanopy.net/docs/reference/daemon-status/" style="color: #2563eb; text-decoration: none;">standby</a> because it hasn't completed a discovery session in over 30 days. While on standby, scheduled discoveries targeting this daemon will be skipped."#,
                        self.daemon_name, self.network_name
                    ))
                    .paragraph("To resume:")
                    .raw(&format!(
r#"                            <ol style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
                                <li>Ensure your daemon is running and connected (<a href="{}" style="color: #2563eb; text-decoration: none;">troubleshooting guide</a>).</li>
                                <li>Manually start a discovery by pressing the Play button on the Discoveries page.</li>
                            </ol>
"#,
                        links::DOCS_DAEMON_TROUBLESHOOTING
                    ))
                    .paragraph("Your daemon will come off standby and scheduled discoveries will resume automatically."),
            )
            .cta("{base_url}/?{utm}#discovery-scans", "Queue Discovery")
            .render()
    }
}
