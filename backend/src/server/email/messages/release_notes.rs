use super::{Body, Content, Email, EmailCategory, compose::html_escape, links};
use crate::server::release_notes::catalog::ReleaseNote;

/// Sent to every user when the server starts on a release marked for email.
/// Carries that release's notes only.
pub struct ReleaseNotes<'a> {
    pub note: &'a ReleaseNote,
}

/// The subject both release emails share.
pub(super) fn subject(note: &ReleaseNote) -> String {
    format!("Scanopy {}: {}", note.version, note.title)
}

/// `note`'s title, the daemon upgrade callout when the release asks for one,
/// then each section of its notes closed by a text link to `section_link`, a
/// `(href, label)` pair. `daemon_upgrade` is the callout's `(text, href, label)`.
pub(super) fn note_content(
    note: &ReleaseNote,
    section_link: (&str, &str),
    daemon_upgrade: (&str, &str, &str),
) -> Content {
    let (href, label) = section_link;
    let mut content = Content::new()
        .heading("What's new in Scanopy")
        .subheading(&format!(
            "{} <span style=\"font-weight: 400; color: #9ca3af;\">v{}</span>",
            html_escape(&note.title),
            note.version
        ));
    if note.daemon_upgrade_notice {
        let (text, href, label) = daemon_upgrade;
        content = content.info(text, href, label);
    }
    for section in &note.sections {
        content = content
            .raw(&format!(
                "<div style=\"margin: 0 0 12px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;\">{}</div>\n",
                email_images(section)
            ))
            .link(href, label);
    }
    content
}

impl Email for ReleaseNotes<'_> {
    fn subject(&self) -> String {
        subject(self.note)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::ProductUpdates
    }

    fn campaign(&self) -> &'static str {
        "release_notes"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(note_content(
                self.note,
                (links::APP_HOME, "Open Scanopy"),
                (
                    &format!("Upgrade your daemons to v{}", self.note.version),
                    links::APP_DAEMONS,
                    "Go to Daemons",
                ),
            ))
            .cta(links::APP_HOME, "Open Scanopy")
            .render()
    }
}

/// The width of the email's content column: the 600px card less its 40px padding.
const CONTENT_WIDTH: u32 = 520;

/// Size each image to the content column. Outlook desktop ignores CSS
/// `max-width` and honours the `width` attribute; every other client scales
/// down to a narrow screen with `max-width`. The rendered notes carry no raw
/// HTML, so every `<img ` is a markdown image.
fn email_images(html: &str) -> String {
    html.replace(
        "<img ",
        &format!(
            "<img width=\"{CONTENT_WIDTH}\" style=\"display: block; width: 100%; max-width: {CONTENT_WIDTH}px; height: auto; margin: 16px 0; border: 0; border-radius: 6px;\" "
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::release_notes::catalog::tests::{note_file_with, note_with_body};

    /// Every section of the notes, the lead-in included, closes with its own
    /// link, in order, before the next section starts.
    #[test]
    fn each_section_closes_with_a_link() {
        let note = ReleaseNote::parse(
            "---\nversion: 0.18.0\ndate: 2026-10-11\ntitle: 'T'\nemail: true\n---\n\nLead-in.\n\n## New Features\n\n- One\n\n## Fixes\n\n- Two\n",
        )
        .unwrap();
        let link = "Open Scanopy";
        let html = ReleaseNotes { note: &note }.body_html();
        let found = ["Lead-in.", link, "New Features", link, "Fixes", link, link]
            .iter()
            .scan(0, |from, needle| {
                let at = *from + html[*from..].find(needle)?;
                *from = at + needle.len();
                Some(at)
            })
            .count();
        assert_eq!(
            found, 7,
            "sections and links interleave, then the button: {html}"
        );
        assert_eq!(html.matches(link).count(), 4);
    }

    #[test]
    fn the_callout_links_the_daemons_tab_only_when_the_file_asks() {
        let html = |extra| {
            let note = ReleaseNote::parse(&note_file_with("0.18.0", extra).1).unwrap();
            ReleaseNotes { note: &note }.body_html()
        };
        assert!(html("email: true\ndaemon_upgrade: true").contains(links::APP_DAEMONS));
        assert!(!html("email: true").contains(links::APP_DAEMONS));
        assert!(!html("email: true\ndaemon_upgrade: false").contains(links::APP_DAEMONS));
    }

    #[test]
    fn an_image_fits_the_content_column() {
        let note = ReleaseNote::parse(&note_with_body(
            "![The topology view](https://scanopy.net/changelog/v0.18.0/topology.png)\n\nText with `<img ` in it.",
        ))
        .unwrap();
        let html = ReleaseNotes { note: &note }.body_html();
        assert_eq!(
            html.matches(&format!("width=\"{CONTENT_WIDTH}\"")).count(),
            1
        );
    }
}
