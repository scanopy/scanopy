//! What an Issue is about, and how the Issues list is filtered, sorted and grouped.

use super::*;

/// What an Issue's fix is about, which groups the Issues list, in display order.
#[derive(
    Serialize,
    Deserialize,
    ToSchema,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    EnumIter,
    IntoStaticStr,
)]
pub enum IssueCategory {
    Daemons,
    Credentials,
    Devices,
    Scans,
    Review,
}

impl IssueCategory {
    /// The entity whose icon and color the category takes.
    fn subject(&self) -> EntityDiscriminants {
        match self {
            Self::Daemons => EntityDiscriminants::Daemon,
            Self::Credentials => EntityDiscriminants::Credential,
            Self::Devices | Self::Review => EntityDiscriminants::Host,
            Self::Scans => EntityDiscriminants::Discovery,
        }
    }
}

impl HasId for IssueCategory {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for IssueCategory {
    fn color(&self) -> Color {
        self.subject().color()
    }

    fn icon(&self) -> Icon {
        self.subject().icon()
    }
}

impl TypeMetadataProvider for IssueCategory {
    fn name(&self) -> &'static str {
        match self {
            Self::Daemons => "Daemons",
            Self::Credentials => "Credentials",
            Self::Devices => "Devices",
            Self::Scans => "Scans",
            Self::Review => "Review",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Daemons => "Daemons to fix, upgrade or install.",
            Self::Credentials => "Credentials that failed, or that no scan uses.",
            Self::Devices => "Devices that answered a scan with less than they hold.",
            Self::Scans => "Scans that stopped early or couldn't reach part of the network.",
            Self::Review => "Hosts and subnets other devices reported, to confirm or dismiss.",
        }
    }
}

/// What the Issues list sorts and groups by. Derived in Rust, so its `to_sql` is never read.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, ToSchema, EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum IssueOrderField {
    /// The Issue's [`IssueCategory`].
    #[default]
    Category,
    /// The Issue's type.
    Type,
}

impl crate::server::shared::handlers::ordering::OrderField for IssueOrderField {
    /// Never read: the list is ordered in memory.
    fn to_sql(&self) -> &'static str {
        "id"
    }
}

/// Query parameters for paging the Issues list.
#[derive(Deserialize, Default, Debug, Clone, utoipa::IntoParams)]
pub struct IssueFilterQuery {
    /// Only Issues on one of these sites, plus the Issues about the whole organization. Repeat
    /// for several. Unset returns every site's.
    pub site_ids: Option<Vec<Uuid>>,
    /// Only Issues in one of these categories. Repeat for several.
    pub categories: Option<Vec<IssueCategory>>,
    /// Only Issues of one of these types. Repeat for several.
    pub types: Option<Vec<SetupItemDiscriminants>>,
    /// What the list is grouped by.
    pub group_by: Option<IssueOrderField>,
    /// What the list sorts by, inside its groups.
    pub order_by: Option<IssueOrderField>,
    /// Direction for `order_by` (`group_by` always sorts ascending).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl IssueFilterQuery {
    pub fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}
