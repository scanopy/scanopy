use super::{Body, Email, EmailCategory, release_notes};
use crate::server::release_notes::catalog::ReleaseNote;

/// How to upgrade a self-hosted server.
const UPGRADE_DOCS: &str = "https://scanopy.net/docs/self-hosted-server/upgrading/";

/// How to upgrade a daemon.
const DAEMON_UPGRADE_DOCS: &str = "https://scanopy.net/docs/setting-up-daemons/upgrading-daemons/";

/// Sent by Scanopy Cloud to the owners of a current self-hosted licence when a
/// release is emailed, in place of [`super::ReleaseNotes`]. Their own servers
/// run elsewhere and are still on the previous release, so every link leads
/// to an upgrade guide.
pub struct LicensedServerUpgrade<'a> {
    pub note: &'a ReleaseNote,
}

impl Email for LicensedServerUpgrade<'_> {
    fn subject(&self) -> String {
        release_notes::subject(self.note)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::ProductUpdates
    }

    fn campaign(&self) -> &'static str {
        "licensed_server_upgrade"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(release_notes::note_content(
                self.note,
                (UPGRADE_DOCS, "Upgrade your server"),
                (
                    &format!(
                        "Upgrade your daemons to v{} after you upgrade your server",
                        self.note.version
                    ),
                    DAEMON_UPGRADE_DOCS,
                    "How to upgrade daemons",
                ),
            ))
            .cta(UPGRADE_DOCS, "Upgrade your server")
            .alt_link(UPGRADE_DOCS)
            .render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::email::messages::links;
    use crate::server::release_notes::catalog::tests::note_file_with;

    /// A licence owner's daemons sit on their own server, which a Cloud link
    /// can't reach.
    #[test]
    fn the_callout_links_the_guide_instead_of_the_daemons_tab() {
        let source = note_file_with("0.18.0", "email: true\ndaemon_upgrade: true").1;
        let note = ReleaseNote::parse(&source).unwrap();
        let html = LicensedServerUpgrade { note: &note }.body_html();
        assert!(html.contains(DAEMON_UPGRADE_DOCS));
        assert!(!html.contains(links::APP_DAEMONS));
    }
}
