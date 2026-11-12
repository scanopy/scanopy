//! The terms updates compiled into this binary.
//!
//! Each change to Scanopy's terms is one `terms/YYYY-MM-DD.md` in
//! `backend/release-notes/`, named for the date it takes effect, which is also
//! its version. The release that ships the file announces it: the cloud server
//! emails every user who signed up before that date, once per update
//! (`EmailService::send_terms_notices`).

use std::sync::OnceLock;

use chrono::{Days, NaiveDate};

use super::catalog::{EmbeddedNotes, render_markdown, split_frontmatter};

/// The least notice an update gives: `effective` is at least this many days
/// after `date`. The notice goes out when the release carrying the file
/// deploys, so a release cut late eats into it; the sweep logs an error when
/// it sends with less.
pub const MIN_NOTICE_DAYS: u64 = 30;

/// The most an update is written ahead of taking effect. A larger gap is
/// almost always a mistyped year.
pub const MAX_LEAD_DAYS: u64 = 180;

/// How long after `effective` a notice that hasn't gone out yet still does,
/// for a deploy that lands late. After that the update is history.
pub const LATE_SEND_DAYS: u64 = 30;

/// The text `scripts/new-terms-update.sh` leaves for each change to be
/// described. A body still holding it doesn't parse, so an unedited file fails
/// CI instead of reaching customers.
pub const PLACEHOLDER: &str = "<describe the change>";

/// Where terms updates sit within the embedded folder.
const TERMS_DIR: &str = "terms/";

/// A document a terms update can change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermsDocument {
    Terms,
    Privacy,
    Refund,
    Security,
}

impl TermsDocument {
    /// The id used in a file's `documents` list.
    fn from_id(id: &str) -> Option<Self> {
        match id {
            "terms" => Some(Self::Terms),
            "privacy" => Some(Self::Privacy),
            "refund" => Some(Self::Refund),
            "security" => Some(Self::Security),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Terms => "Terms of Service",
            Self::Privacy => "Privacy Policy",
            Self::Refund => "Refund Policy",
            Self::Security => "Security page",
        }
    }

    /// The live page, which shows the version in force.
    pub fn url(self) -> &'static str {
        match self {
            Self::Terms => "https://scanopy.net/terms",
            Self::Privacy => "https://scanopy.net/privacy",
            Self::Refund => "https://scanopy.net/refund",
            Self::Security => "https://scanopy.net/security",
        }
    }

    /// The page for the version taking effect on `effective`. It goes up when
    /// the notice does, so the email can link to text not yet in force, and
    /// stays as the archived copy afterwards.
    pub fn version_url(self, effective: NaiveDate) -> String {
        format!("{}/{}", self.url(), effective.format("%Y-%m-%d"))
    }
}

/// One terms update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermsUpdate {
    /// The day the update takes effect. Also its version.
    pub effective: NaiveDate,
    /// The day it was written. Its notice may go out from this day, once a
    /// release carrying the file deploys.
    pub date: NaiveDate,
    /// The documents it changes, in the file's order.
    pub documents: Vec<TermsDocument>,
    /// What changed, rendered from markdown with raw HTML left out.
    pub html: String,
}

impl TermsUpdate {
    /// Parse a terms file: frontmatter (`effective`, `date`, and `documents`),
    /// then the markdown body.
    pub fn parse(source: &str) -> Result<Self, String> {
        let (frontmatter, body) = split_frontmatter(source)?;
        let date_field = |key: &str, value: String| {
            NaiveDate::parse_from_str(&value, "%Y-%m-%d")
                .map_err(|e| format!("{key} `{value}`: {e}"))
        };

        let effective = date_field("effective", frontmatter.required("effective")?)?;
        let date = date_field("date", frontmatter.required("date")?)?;
        let documents = frontmatter
            .required("documents")?
            .split(',')
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(|id| TermsDocument::from_id(id).ok_or(format!("unknown document `{id}`")))
            .collect::<Result<Vec<_>, _>>()?;
        if documents.is_empty() {
            return Err("`documents` is empty".into());
        }
        if body.trim().is_empty() {
            return Err("no description of what changed".into());
        }
        if body.contains(PLACEHOLDER) {
            return Err(format!("the body still says `{PLACEHOLDER}`"));
        }

        Ok(Self {
            effective,
            date,
            documents,
            html: render_markdown(body),
        })
    }

    /// Whether the notice may go out on `today`: from the day the release
    /// carrying it deploys until [`LATE_SEND_DAYS`] after `effective`.
    pub fn sends_on(&self, today: NaiveDate) -> bool {
        self.date <= today && today < self.effective + Days::new(LATE_SEND_DAYS)
    }
}

/// A terms file's name without `terms/` or `.md`; `None` for anything else.
fn file_stem(name: &str) -> Option<&str> {
    let stem = name.strip_prefix(TERMS_DIR)?.strip_suffix(".md")?;
    // A README or other note in the folder is not an update.
    stem.starts_with(|c: char| c.is_ascii_digit())
        .then_some(stem)
}

/// Every terms update, oldest first.
#[derive(Debug, Default)]
pub struct TermsCatalog {
    /// Each update with the stem of the file it came from.
    updates: Vec<(String, TermsUpdate)>,
}

impl TermsCatalog {
    /// The updates compiled into this binary. A file that doesn't parse is
    /// logged and left out; it never stops the server.
    pub fn embedded() -> &'static TermsCatalog {
        static CATALOG: OnceLock<TermsCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| Self::from_files(embedded_files()))
    }

    /// Build from `(stem, source)` pairs.
    pub fn from_files(files: impl IntoIterator<Item = (String, String)>) -> Self {
        let mut updates: Vec<(String, TermsUpdate)> = files
            .into_iter()
            .filter_map(|(stem, source)| match TermsUpdate::parse(&source) {
                Ok(update) => Some((stem, update)),
                Err(error) => {
                    tracing::error!(file = %stem, %error, "Skipping a terms update that doesn't parse");
                    None
                }
            })
            .collect();
        updates.sort_by_key(|(_, update)| update.effective);
        Self { updates }
    }

    /// Every update's version (effective date) in this build, oldest first.
    pub fn versions(&self) -> Vec<NaiveDate> {
        self.updates
            .iter()
            .map(|(_, update)| update.effective)
            .collect()
    }

    /// The updates whose notice may go out on `today`, oldest first.
    pub fn sending_on(&self, today: NaiveDate) -> Vec<&TermsUpdate> {
        self.updates
            .iter()
            .map(|(_, update)| update)
            .filter(|update| update.sends_on(today))
            .collect()
    }

    /// Every way the updates are misdated, one message each, as of `today`.
    pub fn check_dates(&self, today: NaiveDate) -> Vec<String> {
        let mut problems = Vec::new();
        for (stem, update) in &self.updates {
            let effective = update.effective;
            if stem != &effective.format("%Y-%m-%d").to_string() {
                problems.push(format!(
                    "{stem}: the file name doesn't match `effective: {effective}`"
                ));
            }
            if effective < update.date + Days::new(MIN_NOTICE_DAYS) {
                problems.push(format!(
                    "{stem}: takes effect less than {MIN_NOTICE_DAYS} days after `date`"
                ));
            }
            if effective > update.date + Days::new(MAX_LEAD_DAYS) {
                problems.push(format!(
                    "{stem}: takes effect more than {MAX_LEAD_DAYS} days after `date`; check the year"
                ));
            }
            if update.date > today {
                problems.push(format!("{stem}: `date` {} is in the future", update.date));
            }
        }
        for pair in self.updates.windows(2) {
            let [(earlier_stem, earlier), (later_stem, later)] = pair else {
                continue;
            };
            if later.effective == earlier.effective {
                problems.push(format!(
                    "{earlier_stem} and {later_stem} take effect on the same day"
                ));
            }
            if later.date < earlier.date {
                problems.push(format!(
                    "{later_stem} takes effect after {earlier_stem} but was dated before it"
                ));
            }
        }
        problems
    }
}

/// The embedded terms files as `(stem, source)`.
fn embedded_files() -> impl Iterator<Item = (String, String)> {
    EmbeddedNotes::iter().filter_map(|name| {
        let stem = file_stem(&name)?.to_string();
        let file = EmbeddedNotes::get(&name)?;
        Some((stem, String::from_utf8_lossy(&file.data).into_owned()))
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    /// A terms file taking effect on `effective`, written on `date`.
    pub(crate) fn terms_file(effective: &str, date: &str) -> (String, String) {
        (
            effective.to_string(),
            format!(
                "---\neffective: {effective}\ndate: {date}\ndocuments: terms, privacy\n---\n\n- Section 4 now promises notice by email <script>x</script>\n"
            ),
        )
    }

    fn problems(files: Vec<(String, String)>, today: &str) -> Vec<String> {
        TermsCatalog::from_files(files).check_dates(d(today))
    }

    #[test]
    fn parses_frontmatter_and_renders_the_body_without_raw_html() {
        let (_, source) = terms_file("2026-12-01", "2026-10-20");
        let update = TermsUpdate::parse(&source).unwrap();
        assert_eq!(update.effective, d("2026-12-01"));
        assert_eq!(
            update.documents,
            [TermsDocument::Terms, TermsDocument::Privacy]
        );
        assert!(update.html.contains("<li>"));
        assert!(!update.html.contains("<script>"));
    }

    #[test]
    fn a_file_missing_what_it_needs_does_not_parse() {
        for source in [
            "---\neffective: 2026-12-01\ndate: 2026-10-20\n---\nbody",
            "---\neffective: 2026-12-01\ndate: 2026-10-20\ndocuments: terms, eula\n---\nbody",
            "---\neffective: 2026-12-01\ndate: 2026-10-20\ndocuments: terms\n---\n\n",
            "---\neffective: 2026-13-01\ndate: 2026-10-20\ndocuments: terms\n---\nbody",
            "---\neffective: 2026-12-01\ndate: 2026-10-20\ndocuments: terms\n---\n- Section 4: <describe the change>",
        ] {
            assert!(TermsUpdate::parse(source).is_err(), "{source}");
        }
    }

    #[test]
    fn a_well_dated_sequence_has_no_problems() {
        assert!(
            problems(
                vec![
                    terms_file("2026-12-01", "2026-10-20"),
                    terms_file("2027-02-01", "2026-12-15"),
                ],
                "2026-12-20",
            )
            .is_empty()
        );
    }

    #[test]
    fn a_file_named_for_another_day_is_caught() {
        let (_, source) = terms_file("2026-12-01", "2026-10-20");
        assert_eq!(
            problems(vec![("2026-12-02".into(), source)], "2026-10-20").len(),
            1
        );
    }

    #[test]
    fn less_than_thirty_days_notice_is_caught() {
        assert_eq!(
            problems(vec![terms_file("2026-11-18", "2026-10-20")], "2026-10-20").len(),
            1
        );
    }

    #[test]
    fn a_mistyped_year_is_caught() {
        assert_eq!(
            problems(vec![terms_file("2027-12-01", "2026-10-20")], "2026-10-20").len(),
            1
        );
    }

    #[test]
    fn an_update_dated_before_an_earlier_one_is_caught() {
        assert_eq!(
            problems(
                vec![
                    terms_file("2026-12-01", "2026-10-20"),
                    terms_file("2027-01-15", "2026-10-01"),
                ],
                "2026-10-20",
            )
            .len(),
            1
        );
    }

    #[test]
    fn a_date_in_the_future_is_caught() {
        assert_eq!(
            problems(vec![terms_file("2026-12-01", "2026-10-20")], "2026-10-19").len(),
            1
        );
    }

    #[test]
    fn an_update_sends_from_its_date_until_the_late_window_closes() {
        let (_, source) = terms_file("2026-12-01", "2026-10-20");
        let update = TermsUpdate::parse(&source).unwrap();
        assert!(!update.sends_on(d("2026-10-19")));
        assert!(update.sends_on(d("2026-10-20")));
        assert!(update.sends_on(d("2026-12-30")));
        assert!(!update.sends_on(d("2026-12-31")));
    }

    /// Every terms file shipped in `backend/release-notes/terms/` parses and
    /// is dated in sequence, so a misdated notice fails CI instead of
    /// reaching customers.
    #[test]
    fn every_embedded_terms_file_parses_and_is_dated_in_sequence() {
        let files: Vec<_> = embedded_files().collect();
        for (stem, source) in &files {
            if let Err(e) = TermsUpdate::parse(source) {
                panic!("{stem}: {e}");
            }
        }
        let problems = TermsCatalog::from_files(files)
            .check_dates(crate::server::shared::clock::now().date_naive());
        assert!(problems.is_empty(), "{problems:#?}");
    }
}
