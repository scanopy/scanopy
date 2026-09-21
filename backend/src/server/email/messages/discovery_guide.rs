use super::{Body, Content, Email, EmailCategory, EmailPreference, PausableCategory, links};

/// Post-daemon-registration walkthrough; the free/paid variant differs in
/// copy and CTA based on whether the org is on the Free plan.
pub struct DiscoveryGuide<'a> {
    pub daemon_name: &'a str,
    pub network_name: &'a str,
}

impl Email for DiscoveryGuide<'_> {
    fn subject(&self) -> String {
        "Your Daemon is Connected - Discovery is Running".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Onboarding
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Pausable(PausableCategory::ProductOnboarding)
    }

    fn campaign(&self) -> &'static str {
        "discovery_guide"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Your Daemon is Connected!")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Great news — your daemon <strong>{}</strong> just registered on <strong>{}</strong>. Scanopy is now running an initial discovery to map out your network.",
                        self.daemon_name, self.network_name
                    ))
                    .paragraph("Here's what happens next:")
                    .raw(
r#"                            <ul style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
                                <li><strong>Self-report:</strong> The daemon host's own services and IP addresses are mapped automatically.</li>
                                <li><strong>Network scan:</strong> Scanopy scans your local subnets for other hosts, ports, and services.</li>
                                <li><strong>Topology:</strong> Once discovery finishes, your interactive topology map will be ready.</li>
                                <li><strong>Docker discovery:</strong> If your daemon has access to the Docker socket, it'll also discover all your containers — images, ports, networks, and labels — automatically.</li>
                            </ul>
"#,
                    ),
            )
            .cta(links::APP_HOME, "Open Scanopy")
            .render()
    }
}
