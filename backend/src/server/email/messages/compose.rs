//! Reusable HTML chrome for transactional emails.
//!
//! [`Body`] composes an email's inner `<tr>` rows from declarative content, so
//! individual messages declare *what* they say rather than re-implementing the
//! repeated table layout and inline styling. Output is byte-for-byte identical
//! to the hand-written blocks these helpers replace — the `email_body_snapshots`
//! test locks that contract.
//!
//! A typical message:
//!
//! ```ignore
//! Body::new()
//!     .content(
//!         Content::new()
//!             .heading("Reset Your Password")
//!             .paragraph("Hi there,")
//!             .paragraph("We received a request to reset your password."),
//!     )
//!     .cta(&reset_url, "Reset Password")
//!     .alt_link(&reset_url)
//!     .notice("Security Notice", "This link expires in 24 hours.")
//!     .render()
//! ```

/// Shared closing line for the forward-looking billing emails. They no longer
/// quote a dollar figure (a fixture list price ignores active discounts), so
/// each one ends with this single consistent pointer to the Billing page.
pub const BILLING_DETAILS_TAGLINE: &str = "Visit your Billing page for full details.";

/// The trial-ending emails' offer of the org's one self-serve extension,
/// linking to the Billing tab that holds the extend action.
pub fn trial_extension_offer() -> String {
    format!(
        r#"Need more time? <a href="{}" style="color: #2563eb; text-decoration: none;">Extend your trial by {} days</a>, once."#,
        super::links::SETTINGS_BILLING,
        crate::server::billing::plans::TRIAL_EXTENSION_DAYS
    )
}

/// Escape text for an HTML element body or a quoted attribute.
pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// 20-space indent for section-level tags (`<tr>`, `<!-- … -->`).
const SECTION: &str = "                    ";
/// 28-space indent for inline children inside a content `<td>`.
const CHILD: &str = "                            ";

/// Inline content that lives inside a content section's `<td>`: headings,
/// paragraphs, and (via [`raw`](Content::raw)) bespoke blocks like lists or
/// tables.
#[derive(Default)]
pub struct Content {
    inner: String,
}

impl Content {
    pub fn new() -> Self {
        Self::default()
    }

    /// Centered page `<h1>` heading.
    pub fn heading(self, text: &str) -> Self {
        self.child(&format!(
            r#"<h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">{text}</h1>"#
        ))
    }

    /// Left-aligned `<h2>` subheading.
    pub fn subheading(self, text: &str) -> Self {
        self.child(&format!(
            r#"<h2 style="margin: 0 0 12px 0; font-size: 18px; font-weight: 600; color: #1a1a1a;">{text}</h2>"#
        ))
    }

    /// Standard body paragraph. `html` may carry inline markup (`<strong>`,
    /// `<a>`, …).
    pub fn paragraph(self, html: &str) -> Self {
        self.child(&format!(
            r#"<p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">{html}</p>"#
        ))
    }

    /// Small grey fine print (disclaimers, footnotes).
    pub fn fine_print(self, html: &str) -> Self {
        self.child(&format!(
            r#"<p style="margin: 0 0 20px 0; font-size: 12px; line-height: 18px; color: #9ca3af;">{html}</p>"#
        ))
    }

    /// A text link on its own line, quieter than the section-level
    /// [`Body::cta`] button.
    pub fn link(self, href: &str, label: &str) -> Self {
        self.child(&format!(
            r#"<p style="margin: 0 0 24px 0; font-size: 14px; line-height: 20px;"><a href="{href}" style="color: #2563eb; font-weight: 500;">{label}</a></p>"#
        ))
    }

    /// A blue box with a link, the email form of the app's inline info.
    pub fn info(self, text: &str, href: &str, label: &str) -> Self {
        self.child(&format!(
            r#"<div style="margin: 0 0 20px 0; padding: 12px 16px; background-color: #eff6ff; border: 1px solid #bfdbfe; border-radius: 8px; font-size: 14px; line-height: 20px; color: #1e40af;"><div style="font-weight: 500;">{text}</div><a href="{href}" style="color: #2563eb; font-weight: 500;">{label}</a></div>"#
        ))
    }

    /// A command or other preformatted text in a dark monospace block, escaped.
    pub fn code(mut self, text: &str) -> Self {
        self.inner.push_str(&format!(
            r#"{CHILD}<div style="margin: 0 0 20px 0; padding: 16px; background-color: #1e293b; border-radius: 6px; overflow-x: auto;">
{CHILD}    <pre style="margin: 0; font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace; font-size: 13px; line-height: 20px; color: #e2e8f0; white-space: pre-wrap; word-break: break-all;">{}</pre>
{CHILD}</div>
"#,
            html_escape(text)
        ));
        self
    }

    /// Append a fully-formed, already-indented inline block verbatim — for
    /// bespoke content (lists, tables, code blocks) the typed helpers don't
    /// cover. The caller supplies exact 28-space indentation and a trailing
    /// newline so output stays byte-identical.
    pub fn raw(mut self, html: &str) -> Self {
        self.inner.push_str(html);
        self
    }

    fn child(mut self, element: &str) -> Self {
        self.inner.push_str(CHILD);
        self.inner.push_str(element);
        self.inner.push('\n');
        self
    }
}

/// The inner `<tr>` rows of an email body, assembled section by section.
#[derive(Default)]
pub struct Body {
    sections: Vec<String>,
}

impl Body {
    pub fn new() -> Self {
        Self::default()
    }

    /// Main content section (`<!-- Main Content -->`).
    pub fn content(self, content: Content) -> Self {
        self.content_named("Main Content", content)
    }

    /// Content section with a custom HTML comment label (e.g. "Trial Recap").
    pub fn content_named(mut self, comment: &str, content: Content) -> Self {
        self.sections.push(format!(
            "{SECTION}<!-- {comment} -->\n\
             {SECTION}<tr>\n\
             {SECTION}    <td style=\"padding: 0 40px 20px 40px;\">\n\
             {inner}\
             {SECTION}    </td>\n\
             {SECTION}</tr>",
            inner = content.inner,
        ));
        self
    }

    /// Primary call-to-action button section.
    pub fn cta(mut self, href: &str, label: &str) -> Self {
        self.sections.push(format!(
            "{SECTION}<!-- CTA Button -->\n\
             {SECTION}<tr>\n\
             {SECTION}    <td align=\"center\" style=\"padding: 0 40px 30px 40px;\">\n\
             {SECTION}        <a href=\"{href}\" style=\"display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;\">{label}</a>\n\
             {SECTION}    </td>\n\
             {SECTION}</tr>",
        ));
        self
    }

    /// "If the button doesn't work" copy-and-paste fallback link section.
    pub fn alt_link(mut self, url: &str) -> Self {
        self.sections.push(format!(
            "{SECTION}<!-- Alternative Link -->\n\
             {SECTION}<tr>\n\
             {SECTION}    <td style=\"padding: 0 40px 20px 40px;\">\n\
             {SECTION}        <p style=\"margin: 0 0 10px 0; font-size: 14px; line-height: 20px; color: #6b7280;\">If the button doesn't work, copy and paste this link into your browser:</p>\n\
             {SECTION}        <p style=\"margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #2563eb; word-break: break-all;\">{url}</p>\n\
             {SECTION}    </td>\n\
             {SECTION}</tr>",
        ));
        self
    }

    /// Border-topped fine-print notice section with a custom comment label
    /// (e.g. "Security Notice", "Expiration Notice").
    pub fn notice(mut self, comment: &str, text: &str) -> Self {
        self.sections.push(format!(
            "{SECTION}<!-- {comment} -->\n\
             {SECTION}<tr>\n\
             {SECTION}    <td style=\"padding: 0 40px 30px 40px; border-top: 1px solid #e5e7eb;\">\n\
             {SECTION}        <p style=\"margin: 20px 0 0 0; font-size: 14px; line-height: 20px; color: #6b7280;\">{text}</p>\n\
             {SECTION}    </td>\n\
             {SECTION}</tr>",
        ));
        self
    }

    /// Join the sections with the standard blank-line separator and a trailing
    /// newline, matching the shape `render_html` wraps with the shared chrome.
    pub fn render(self) -> String {
        let mut out = self.sections.join("\n\n");
        out.push('\n');
        out
    }
}
