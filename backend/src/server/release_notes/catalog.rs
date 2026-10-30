//! The release notes compiled into this binary.
//!
//! `prepare_release.yml` writes one `vX.Y.Z.md` per release into
//! `backend/release-notes/`, and the file is reviewed in the release PR before
//! the build. Every server carries the notes of its own release and every
//! earlier one, so an air-gapped server shows them without fetching anything.

use std::sync::OnceLock;

use chrono::NaiveDate;
use comrak::nodes::NodeValue;
use rust_embed::Embed;
use semver::Version;

/// Releases shown at once, in the modal and in the email. Older ones are behind
/// the changelog link.
pub const MAX_SHOWN: usize = 5;

/// The public changelog, for releases past [`MAX_SHOWN`].
pub const CHANGELOG_URL: &str = "https://scanopy.net/changelog";

/// `backend/release-notes/`: the release notes, and the terms updates under
/// `terms/` (see [`super::terms`]).
#[derive(Embed)]
#[folder = "release-notes/"]
pub(crate) struct EmbeddedNotes;

/// The `key: value` lines between a file's `---` fences.
pub(crate) struct Frontmatter<'a>(&'a str);

impl Frontmatter<'_> {
    pub(crate) fn field(&self, key: &str) -> Option<String> {
        self.0.lines().find_map(|line| {
            let (k, v) = line.split_once(':')?;
            (k.trim() == key).then(|| unquote(v.trim()))
        })
    }

    pub(crate) fn required(&self, key: &str) -> Result<String, String> {
        self.field(key).ok_or(format!("missing `{key}`"))
    }
}

/// Split a file into its YAML-style frontmatter and markdown body.
pub(crate) fn split_frontmatter(source: &str) -> Result<(Frontmatter<'_>, &str), String> {
    let rest = source.strip_prefix("---\n").ok_or("missing frontmatter")?;
    let (frontmatter, body) = rest
        .split_once("\n---\n")
        .ok_or("unterminated frontmatter")?;
    Ok((Frontmatter(frontmatter), body))
}

/// Render a markdown body with raw HTML left out.
pub(crate) fn render_markdown(body: &str) -> String {
    comrak::markdown_to_html(body.trim(), &comrak::Options::default())
}

/// Where a release's images live: the website's `static/changelog/vX.Y.Z/`.
/// Self-hosted servers send the email too, so the URL has to be public.
fn image_base_url(version: &Version) -> String {
    format!("https://scanopy.net/changelog/v{version}/")
}

/// Image formats every mail client shows. Outlook desktop doesn't render WebP.
const IMAGE_EXTENSIONS: [&str; 3] = [".png", ".jpg", ".jpeg"];

/// Reject an image that a mail client or another server couldn't load, or one
/// without alt text, which is all a reader sees while remote images are blocked.
fn check_images(body: &str, version: &Version) -> Result<(), String> {
    let base = image_base_url(version);
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, body, &comrak::Options::default());
    for node in root.descendants() {
        let comrak::nodes::NodeValue::Image(ref link) = node.data().value else {
            continue;
        };
        let url = &link.url;
        if !url.starts_with(&base) {
            return Err(format!("image `{url}` is not under {base}"));
        }
        let path = url
            .split(['?', '#'])
            .next()
            .unwrap_or(url)
            .to_ascii_lowercase();
        if !IMAGE_EXTENSIONS.iter().any(|ext| path.ends_with(ext)) {
            return Err(format!("image `{url}` is not PNG or JPEG"));
        }
        if comrak::html::collect_text(node).trim().is_empty() {
            return Err(format!("image `{url}` has no alt text"));
        }
    }
    Ok(())
}

/// Render a markdown body as sections, split before each top-level heading.
/// Content before the first heading is a section of its own. A heading inside
/// a code block or a list doesn't split.
fn render_sections(body: &str) -> Vec<String> {
    let body = body.trim();
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, body, &comrak::Options::default());
    let lines: Vec<&str> = body.lines().collect();
    let mut bounds = vec![0];
    bounds.extend(
        root.children()
            .filter(|node| matches!(node.data().value, NodeValue::Heading(_)))
            .map(|node| node.data().sourcepos.start.line - 1),
    );
    bounds.push(lines.len());
    bounds.dedup();
    bounds
        .windows(2)
        .map(|range| lines[range[0]..range[1]].join("\n"))
        .filter(|section| !section.trim().is_empty())
        .map(|section| render_markdown(&section))
        .collect()
}

/// One release's notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseNote {
    pub version: Version,
    pub date: NaiveDate,
    pub title: String,
    /// Whether upgrading to this release sends the release email.
    pub email: bool,
    /// Whether this release opens the release notes modal on its own. Notes
    /// from before the flag existed have none and open it.
    pub modal: bool,
    /// Whether Scanopy Cloud emails this release to the owners of self-hosted
    /// licences when it doesn't email everyone. Notes from before the flag
    /// existed have none and don't.
    pub licensed_email: bool,
    /// Whether this release asks users to upgrade their daemons. Notes from
    /// before the flag existed have none and ask.
    pub daemon_upgrade: bool,
    /// The body, rendered from markdown with raw HTML left out.
    pub html: String,
    /// The body rendered as sections, split before each heading. The release
    /// email closes each one with a link.
    pub sections: Vec<String>,
}

impl ReleaseNote {
    /// This release's page on the public changelog.
    pub fn url(&self) -> String {
        format!("{CHANGELOG_URL}/v{}", self.version)
    }

    /// Parse a notes file: YAML-style frontmatter (`version`, `date`, `title`,
    /// `email`, optional `modal`, `licensed_email` and `daemon_upgrade`), then the
    /// markdown body.
    pub fn parse(source: &str) -> Result<Self, String> {
        let (frontmatter, body) = split_frontmatter(source)?;
        let required = |key: &str| frontmatter.required(key);

        let version = required("version")?;
        let version = Version::parse(version.trim_start_matches('v'))
            .map_err(|e| format!("version `{version}`: {e}"))?;
        let date = required("date")?;
        let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
            .map_err(|e| format!("date `{date}`: {e}"))?;
        let email = match required("email")?.as_str() {
            "true" => true,
            "false" => false,
            other => return Err(format!("email `{other}` is not true or false")),
        };
        let modal = match frontmatter.field("modal").as_deref() {
            None | Some("true") => true,
            Some("false") => false,
            Some(other) => return Err(format!("modal `{other}` is not true or false")),
        };
        check_images(body, &version)?;
        let licensed_email = match frontmatter.field("licensed_email").as_deref() {
            None | Some("false") => false,
            Some("true") => true,
            Some(other) => return Err(format!("licensed_email `{other}` is not true or false")),
        };
        let daemon_upgrade = match frontmatter.field("daemon_upgrade").as_deref() {
            None | Some("true") => true,
            Some("false") => false,
            Some(other) => return Err(format!("daemon_upgrade `{other}` is not true or false")),
        };

        Ok(Self {
            version,
            date,
            title: required("title")?,
            email,
            modal,
            licensed_email,
            daemon_upgrade,
            html: render_markdown(body),
            sections: render_sections(body),
        })
    }
}

/// `vX.Y.Z.md`; anything else in the folder, such as its README, is not a release.
fn is_notes_file(name: &str) -> bool {
    name.starts_with('v') && name.ends_with(".md")
}

/// Undo YAML quoting: a single-quoted scalar doubles a literal apostrophe.
fn unquote(value: &str) -> String {
    if let Some(inner) = value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
        inner.replace("''", "'")
    } else if let Some(inner) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        inner.to_string()
    } else {
        value.to_string()
    }
}

/// Every release's notes, newest first.
#[derive(Debug, Default)]
pub struct Catalog {
    notes: Vec<ReleaseNote>,
}

impl Catalog {
    /// The notes compiled into this binary. A file that doesn't parse is logged
    /// and left out; it never stops the server.
    pub fn embedded() -> &'static Catalog {
        static CATALOG: OnceLock<Catalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            Catalog::from_files(EmbeddedNotes::iter().filter_map(|name| {
                if !is_notes_file(&name) {
                    return None;
                }
                let file = EmbeddedNotes::get(&name)?;
                Some((
                    name.to_string(),
                    String::from_utf8_lossy(&file.data).into_owned(),
                ))
            }))
        })
    }

    pub fn from_files(files: impl IntoIterator<Item = (String, String)>) -> Self {
        let mut notes: Vec<ReleaseNote> = files
            .into_iter()
            .filter_map(|(name, source)| match ReleaseNote::parse(&source) {
                Ok(note) => Some(note),
                Err(error) => {
                    tracing::error!(file = %name, %error, "Skipping release notes that don't parse");
                    None
                }
            })
            .collect();
        notes.sort_by(|a, b| b.version.cmp(&a.version));
        Self { notes }
    }

    pub fn get(&self, version: &Version) -> Option<&ReleaseNote> {
        self.notes.iter().find(|note| &note.version == version)
    }

    /// Releases newer than `after` up to and including `up_to`, newest first.
    pub fn between(&self, after: &Version, up_to: &Version) -> Vec<&ReleaseNote> {
        self.notes
            .iter()
            .filter(|note| &note.version > after && &note.version <= up_to)
            .collect()
    }

    /// The newest release at or below `up_to` that asks users to upgrade their
    /// daemons.
    pub fn daemon_upgrade_target(&self, up_to: &Version) -> Option<&Version> {
        self.notes
            .iter()
            .find(|note| note.daemon_upgrade && &note.version <= up_to)
            .map(|note| &note.version)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn note_file(version: &str, email: bool) -> (String, String) {
        note_file_with(version, &format!("email: {email}"))
    }

    /// A notes file with `extra` frontmatter lines after `version`, `date` and `title`.
    pub(crate) fn note_file_with(version: &str, extra: &str) -> (String, String) {
        (
            format!("v{version}.md"),
            format!(
                "---\nversion: {version}\ndate: 2026-10-11\ntitle: 'It''s {version}'\n{extra}\n---\n\n## Fixes\n\n- Fix a thing <script>alert(1)</script>\n"
            ),
        )
    }

    #[test]
    fn parses_frontmatter_and_renders_the_body_without_raw_html() {
        let (_, source) = note_file("0.18.0", true);
        let note = ReleaseNote::parse(&source).unwrap();
        assert_eq!(note.version, Version::new(0, 18, 0));
        assert_eq!(note.title, "It's 0.18.0");
        assert!(note.email);
        assert!(note.html.contains("<h2>Fixes</h2>"));
        assert!(!note.html.contains("<script>"));
    }

    /// Notes written before the `modal` flag existed open the modal, as they did.
    #[test]
    fn modal_defaults_to_on_when_the_flag_is_absent() {
        let modal = |extra: &str| {
            ReleaseNote::parse(&note_file_with("0.18.0", extra).1).map(|note| note.modal)
        };
        assert_eq!(modal("email: false"), Ok(true));
        assert_eq!(modal("email: false\nmodal: false"), Ok(false));
        assert!(modal("email: false\nmodal: maybe").is_err());
    }

    /// A notes file for 0.18.0 whose body is `body`.
    pub(crate) fn note_with_body(body: &str) -> String {
        format!("---\nversion: 0.18.0\ndate: 2026-10-11\ntitle: T\nemail: true\n---\n\n{body}\n")
    }

    #[test]
    fn an_image_hosted_for_its_release_with_alt_text_parses() {
        let note = ReleaseNote::parse(&note_with_body(
            "![The topology view](https://scanopy.net/changelog/v0.18.0/topology.png)",
        ))
        .unwrap();
        assert!(note.html.contains("<img "));
        assert!(note.html.contains(r#"alt="The topology view""#));
    }

    #[test]
    fn an_image_a_reader_could_not_load_is_rejected() {
        for image in [
            "![Topology](topology.png)",
            "![Topology](/changelog/v0.18.0/topology.png)",
            "![Topology](http://scanopy.net/changelog/v0.18.0/topology.png)",
            "![Topology](https://example.com/changelog/v0.18.0/topology.png)",
            "![Topology](https://scanopy.net.example.com/changelog/v0.18.0/topology.png)",
            "![Topology](https://scanopy.net/changelog/v0.17.0/topology.png)",
            "![Topology](https://scanopy.net/changelog/v0.18.0/topology.webp)",
            "![](https://scanopy.net/changelog/v0.18.0/topology.png)",
            "![ ](https://scanopy.net/changelog/v0.18.0/topology.png)",
            "[![](https://scanopy.net/changelog/v0.18.0/topology.png)](https://scanopy.net)",
            "Text\n\n![Topology][ref]\n\n[ref]: topology.png",
        ] {
            assert!(
                ReleaseNote::parse(&note_with_body(image)).is_err(),
                "accepted {image}"
            );
        }
    }

    /// Notes written before the `licensed_email` flag existed email no licence
    /// owners on their own, as they didn't.
    #[test]
    fn licensed_email_defaults_to_off_when_the_flag_is_absent() {
        let licensed = |extra: &str| {
            ReleaseNote::parse(&note_file_with("0.18.0", extra).1).map(|note| note.licensed_email)
        };
        assert_eq!(licensed("email: false"), Ok(false));
        assert_eq!(licensed("email: false\nlicensed_email: true"), Ok(true));
        assert!(licensed("email: false\nlicensed_email: yes").is_err());
    }

    /// Notes written before the `daemon_upgrade` flag existed ask for an upgrade,
    /// as every release did.
    #[test]
    fn daemon_upgrade_defaults_to_on_when_the_flag_is_absent() {
        let flagged = |extra: &str| {
            ReleaseNote::parse(&note_file_with("0.18.0", extra).1).map(|note| note.daemon_upgrade)
        };
        assert_eq!(flagged("email: false"), Ok(true));
        assert_eq!(flagged("email: false\ndaemon_upgrade: false"), Ok(false));
        assert!(flagged("email: false\ndaemon_upgrade: maybe").is_err());
    }

    /// The target is the newest flagged release at or below the bound, whatever
    /// unflagged releases sit above it.
    #[test]
    fn daemon_upgrade_target_is_the_newest_flagged_release_up_to_the_bound() {
        let unflagged = "email: false\ndaemon_upgrade: false";
        let catalog = Catalog::from_files([
            note_file("0.18.0", false),
            note_file_with("0.18.1", unflagged),
            note_file_with("0.18.2", "email: false\ndaemon_upgrade: true"),
            note_file_with("0.18.3", unflagged),
            note_file_with("0.18.4", unflagged),
            note_file("0.19.0", false),
        ]);
        let target = |up_to: Version| catalog.daemon_upgrade_target(&up_to).cloned();

        assert_eq!(target(Version::new(0, 18, 4)), Some(Version::new(0, 18, 2)));
        assert_eq!(target(Version::new(0, 18, 1)), Some(Version::new(0, 18, 0)));
        assert_eq!(target(Version::new(0, 19, 0)), Some(Version::new(0, 19, 0)));
        assert_eq!(target(Version::new(0, 17, 0)), None);
    }

    /// Sections split before every top-level heading, keep any lead-in as their
    /// own, and render the same markup as the whole body.
    #[test]
    fn sections_split_before_each_heading() {
        let body = "Lead-in.\n\n## New Features\n\n- One\n\n```\n## not a heading\n```\n\nSetext\n------\n\n- Two\n";
        let sections = render_sections(body);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0], "<p>Lead-in.</p>\n");
        assert!(sections[1].starts_with("<h2>New Features</h2>"));
        assert!(sections[1].contains("## not a heading"));
        assert!(sections[2].starts_with("<h2>Setext</h2>"));
        assert_eq!(sections.concat(), render_markdown(body));
    }

    #[test]
    fn a_malformed_file_is_left_out() {
        let catalog = Catalog::from_files([
            note_file("0.18.0", false),
            ("v0.18.1.md".into(), "no frontmatter".into()),
            (
                "v0.18.2.md".into(),
                "---\nversion: 0.18.2\n---\nbody".into(),
            ),
        ]);
        let versions: Vec<_> = catalog
            .notes
            .iter()
            .map(|n| n.version.to_string())
            .collect();
        assert_eq!(versions, ["0.18.0"]);
    }

    #[test]
    fn between_is_newest_first_and_bounded_on_both_sides() {
        let catalog = Catalog::from_files(
            ["0.17.9", "0.18.0", "0.18.1", "0.18.2", "0.19.0"].map(|v| note_file(v, false)),
        );
        let versions: Vec<_> = catalog
            .between(&Version::new(0, 18, 0), &Version::new(0, 18, 2))
            .iter()
            .map(|n| n.version.to_string())
            .collect();
        assert_eq!(versions, ["0.18.2", "0.18.1"]);
    }

    /// Every file shipped in `backend/release-notes/` parses, so a bad merge
    /// fails CI instead of silently dropping a release from the modal.
    #[test]
    fn every_embedded_file_parses() {
        for name in EmbeddedNotes::iter().filter(|name| is_notes_file(name)) {
            let file = EmbeddedNotes::get(&name).unwrap();
            let source = String::from_utf8_lossy(&file.data);
            if let Err(e) = ReleaseNote::parse(&source) {
                panic!("{name}: {e}");
            }
        }
    }
}
