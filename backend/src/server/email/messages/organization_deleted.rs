use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Confirms that an organization and all of its data have been deleted.
pub struct OrganizationDeleted;

impl Email for OrganizationDeleted {
    fn subject(&self) -> String {
        "Your Organization Has Been Deleted".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Account
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "organization_deleted"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Organization Deleted")
                    .paragraph("Hi there,")
                    .paragraph("Your Scanopy organization has been deleted. All of its data, along with every user account in the organization, has been removed.")
                    .paragraph("If you'd like to use Scanopy again, you can sign up for a new account at any time."),
            )
            .cta(links::APP_HOME, "Create a New Account")
            .render()
    }
}
