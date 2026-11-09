use crate::server::users::r#impl::permissions::UserOrgPermissions;
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateInviteRequest {
    /// How long the invite stays valid, in hours.
    pub expiration_hours: Option<i64>,
    /// Role the invited user gets on acceptance.
    pub permissions: UserOrgPermissions,
    /// The sites this entity applies to.
    pub site_ids: Vec<Uuid>,
    /// Address to email the invite to. Omit to create a link without sending.
    #[schema(value_type = Option<String>)]
    pub send_to: Option<EmailAddress>,
}

/// The signed-in user's organization, with what the server derives about it.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OrganizationResponse {
    #[serde(flatten)]
    pub organization: crate::server::organizations::r#impl::base::Organization,
    /// Whether any of the organization's daemons has ever connected. A daemon created but not
    /// yet started doesn't count.
    #[schema(read_only)]
    pub daemon_connected: bool,
}
