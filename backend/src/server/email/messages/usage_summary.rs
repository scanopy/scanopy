use super::{Body, Content, Email, EmailAttachment, EmailCategory, EmailPreference, links};

/// Monthly billing summary. The authoritative amount lives in the Stripe
/// invoice itself (attached as a PDF, or linked via its hosted URL when the
/// PDF isn't ready) — we never re-render line items here, because a
/// hand-built table doesn't reconcile once discounts, account credits, or
/// pause credits land as separate Stripe lines. The plain-text total we show
/// is `amount_paid`, which already reflects all of those.
pub struct UsageSummary<'a> {
    pub period: &'a str,
    pub invoice_date: &'a str,
    pub total: &'a str,
    /// The invoice PDF, fetched before send. When present it's attached and
    /// the body points the reader at the attachment.
    pub attachment: Option<EmailAttachment>,
    /// Stripe's hosted invoice page — the fallback CTA when the PDF couldn't
    /// be attached in time.
    pub hosted_invoice_url: Option<&'a str>,
}

impl Email for UsageSummary<'_> {
    fn subject(&self) -> String {
        format!("Your {} Invoice", self.period)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "usage_summary"
    }

    fn body_html(&self) -> String {
        let invoice_pointer = if self.attachment.is_some() {
            "Your full itemized invoice is attached to this email as a PDF."
        } else {
            "Your full itemized invoice is available from Stripe — use the button below to view it."
        };

        let content = Content::new()
            .heading("Monthly Billing Summary")
            .paragraph("Hi there,")
            .paragraph(&format!(
                "You were charged {} on {} for your Scanopy subscription ({}).",
                self.total, self.invoice_date, self.period
            ))
            .paragraph(invoice_pointer)
            .raw(
r#"                            <p style="margin: 0; font-size: 14px; line-height: 20px; color: #6b7280;">Questions? Please reach out to <a href="mailto:billing@scanopy.net" style="color: #2563eb; text-decoration: none;">billing@scanopy.net</a></p>
"#,
            );

        // When the PDF couldn't be attached, fall back to linking Stripe's
        // hosted invoice page; otherwise send the reader to the Billing tab.
        let (cta_href, cta_label) = match self.hosted_invoice_url {
            Some(url) if self.attachment.is_none() => (url.to_string(), "View Invoice"),
            _ => (links::SETTINGS_BILLING.to_string(), "View Billing"),
        };

        Body::new()
            .content(content)
            .cta(&cta_href, cta_label)
            .render()
    }

    fn attachments(&self) -> Vec<EmailAttachment> {
        self.attachment
            .as_ref()
            .map(|a| {
                vec![EmailAttachment {
                    filename: a.filename.clone(),
                    content_type: a.content_type.clone(),
                    bytes: a.bytes.clone(),
                }]
            })
            .unwrap_or_default()
    }
}
