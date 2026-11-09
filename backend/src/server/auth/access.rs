//! What a principal may do: per resource type, per action, per site.
//!
//! A [`Resource`] is one thing an admin thinks about, so one per tab; `E::resource()` maps every
//! entity onto one. [`Grants`] are (resource, action) pairs, and a role is a source of them. A
//! principal holds one organization role, which answers for org-keyed resources, and any number of
//! [`Assignment`]s, each a [`SiteRole`] at a scope, which answer for site-keyed resources. An
//! [`AccessSet`] holds the resulting grants and is all the evaluator reads.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::{
    shared::{
        product_area::ProductArea,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
    users::r#impl::permissions::UserOrgPermissions,
};

/// What a role grants access to. Child entities share their parent's resource.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Display,
    EnumIter,
    IntoStaticStr,
    Serialize,
    ToSchema,
)]
pub enum Resource {
    Site,
    Host,
    Service,
    Subnet,
    Vlan,
    Dependency,
    Topology,
    Share,
    Discovery,
    Daemon,
    DaemonApiKey,
    Credential,
    Tag,
    Member,
    UserApiKey,
    Organization,
    Billing,
    License,
    /// Creating a site, which has no site to scope to. Updating and deleting one are `Site`.
    NewSite,
}

/// What a role may do to a resource. `Write` is create and update, plus operations such as
/// starting a scan or merging hosts.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display, EnumIter, Serialize, ToSchema,
)]
pub enum Action {
    Read,
    Write,
    Delete,
}

/// An action as the role comparison shows it. `Write` is one [`Action`] and reads as Edit, plus
/// Create on a resource where it adds rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Serialize, ToSchema)]
pub enum DisplayAction {
    View,
    Create,
    Edit,
    Delete,
}

/// What a resource's rows belong to, which decides whose grants answer for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keying {
    /// A site: the grants the principal holds on that site answer.
    Site,
    /// The organization: the organization grants answer.
    Organization,
}

/// A principal's role on a site. Owner has none: it adds nothing on site-keyed resources.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, EnumIter, EnumString)]
pub enum SiteRole {
    Admin,
    Member,
    #[default]
    Viewer,
}

impl From<UserOrgPermissions> for SiteRole {
    fn from(role: UserOrgPermissions) -> Self {
        match role {
            UserOrgPermissions::Owner | UserOrgPermissions::Admin => SiteRole::Admin,
            UserOrgPermissions::Member => SiteRole::Member,
            UserOrgPermissions::Viewer => SiteRole::Viewer,
        }
    }
}

const NONE: &[Action] = &[];
const R: &[Action] = &[Action::Read];
const W: &[Action] = &[Action::Write];
const RW: &[Action] = &[Action::Read, Action::Write];
const RWD: &[Action] = &[Action::Read, Action::Write, Action::Delete];

/// Which role answers for a resource, and what each of its values may do there.
#[derive(Clone, Copy)]
pub enum RoleTable {
    Organization(fn(UserOrgPermissions) -> &'static [Action]),
    Site(fn(SiteRole) -> &'static [Action]),
}

impl Resource {
    /// The role table. Viewer sees the network, Member keeps it documented, Admin decides how the
    /// org runs, Owner answers for the org.
    pub fn grant(self) -> RoleTable {
        use UserOrgPermissions::{Admin, Member, Owner, Viewer};
        match self {
            Resource::Host
            | Resource::Service
            | Resource::Subnet
            | Resource::Vlan
            | Resource::Dependency
            | Resource::Topology
            | Resource::Share
            | Resource::Discovery => RoleTable::Site(|role| match role {
                SiteRole::Viewer => R,
                SiteRole::Member | SiteRole::Admin => RWD,
            }),
            Resource::Site | Resource::Daemon => RoleTable::Site(|role| match role {
                SiteRole::Viewer | SiteRole::Member => R,
                SiteRole::Admin => RWD,
            }),
            Resource::DaemonApiKey => RoleTable::Site(|role| match role {
                SiteRole::Viewer => NONE,
                SiteRole::Member => R,
                SiteRole::Admin => RWD,
            }),
            Resource::Credential => RoleTable::Organization(|role| match role {
                Viewer | Member => R,
                Admin | Owner => RWD,
            }),
            Resource::Tag => RoleTable::Organization(|role| match role {
                Viewer => R,
                Member => RW,
                Admin | Owner => RWD,
            }),
            Resource::Member => RoleTable::Organization(|role| match role {
                Viewer | Member => NONE,
                Admin | Owner => RWD,
            }),
            Resource::UserApiKey => RoleTable::Organization(|role| match role {
                Viewer | Member | Admin | Owner => RWD,
            }),
            Resource::Organization => RoleTable::Organization(|role| match role {
                Viewer | Member | Admin => R,
                Owner => RWD,
            }),
            Resource::Billing | Resource::License => RoleTable::Organization(|role| match role {
                Viewer | Member | Admin => NONE,
                Owner => RW,
            }),
            Resource::NewSite => RoleTable::Organization(|role| match role {
                Viewer | Member => NONE,
                Admin | Owner => W,
            }),
        }
    }

    pub fn keying(self) -> Keying {
        match self.grant() {
            RoleTable::Organization(_) => Keying::Organization,
            RoleTable::Site(_) => Keying::Site,
        }
    }

    /// What `role` allows on this resource, reading the organization or the site table.
    pub fn actions(self, role: UserOrgPermissions) -> &'static [Action] {
        match self.grant() {
            RoleTable::Organization(table) => table(role),
            RoleTable::Site(table) => table(role.into()),
        }
    }

    /// Whether `Write` on this resource adds rows. Creating a site is [`Resource::NewSite`], and
    /// the rest of these are one record to edit.
    fn write_creates(self) -> bool {
        match self {
            Resource::Host
            | Resource::Service
            | Resource::Subnet
            | Resource::Vlan
            | Resource::Dependency
            | Resource::Topology
            | Resource::Share
            | Resource::Discovery
            | Resource::Daemon
            | Resource::DaemonApiKey
            | Resource::Credential
            | Resource::Tag
            | Resource::Member
            | Resource::UserApiKey => true,
            Resource::Site
            | Resource::NewSite
            | Resource::Organization
            | Resource::Billing
            | Resource::License => false,
        }
    }

    /// What `role` holds on this resource, as the role comparison shows it. `NewSite` shows no
    /// row of its own: it's the Create on `Site`.
    pub fn display_actions(self, role: UserOrgPermissions) -> Vec<DisplayAction> {
        if self == Resource::NewSite {
            return Vec::new();
        }
        let held = self.actions(role);
        let writes = held.contains(&Action::Write);
        let creates = (writes && self.write_creates())
            || (self == Resource::Site && Resource::NewSite.actions(role).contains(&Action::Write));
        DisplayAction::iter()
            .filter(|action| match action {
                DisplayAction::View => held.contains(&Action::Read),
                DisplayAction::Create => creates,
                DisplayAction::Edit => writes,
                DisplayAction::Delete => held.contains(&Action::Delete),
            })
            .collect()
    }

    /// The part of the product this resource sits under in the role comparison.
    pub fn area(self) -> ProductArea {
        match self {
            Resource::Topology | Resource::Share | Resource::Dependency => ProductArea::Topology,
            Resource::Site
            | Resource::NewSite
            | Resource::Host
            | Resource::Service
            | Resource::Subnet
            | Resource::Vlan => ProductArea::Inventory,
            Resource::Discovery
            | Resource::Daemon
            | Resource::DaemonApiKey
            | Resource::Credential => ProductArea::Discovery,
            Resource::Tag | Resource::Member | Resource::UserApiKey | Resource::Organization => {
                ProductArea::Platform
            }
            Resource::Billing | Resource::License => ProductArea::Billing,
        }
    }

    /// Whether a lapsed org can still write this resource: the Settings routes the billing
    /// middleware leaves open, and what sits outside that middleware.
    pub fn writable_when_lapsed(self) -> bool {
        matches!(
            self,
            Resource::Organization | Resource::Billing | Resource::License
        )
    }
}

impl HasId for Resource {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for Resource {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        Icon::ShieldCheck
    }
}

impl TypeMetadataProvider for Resource {
    fn name(&self) -> &'static str {
        match self {
            Resource::Site => "Sites",
            Resource::Host => "Hosts",
            Resource::Service => "Services",
            Resource::Subnet => "Subnets",
            Resource::Vlan => "VLANs",
            Resource::Dependency => "Dependencies",
            Resource::Topology => "Topologies",
            Resource::Share => "Shares",
            Resource::Discovery => "Scans",
            Resource::Daemon => "Daemons",
            Resource::DaemonApiKey => "Daemon API keys",
            Resource::Credential => "Credentials",
            Resource::Tag => "Tags",
            Resource::Member => "Users and invites",
            Resource::UserApiKey => "Your API keys",
            Resource::Organization => "Organization settings",
            Resource::Billing => "Billing",
            Resource::License => "Licenses",
            Resource::NewSite => "New sites",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Resource::Site => "Site details and the sites themselves",
            Resource::Host => "Hosts, their interfaces, addresses and ports",
            Resource::Service => "Services and their bindings",
            Resource::Subnet => "Subnets",
            Resource::Vlan => "VLANs",
            Resource::Dependency => "Dependencies between services",
            Resource::Topology => "Topologies and their snapshots",
            Resource::Share => "Share links and embeds",
            Resource::Discovery => "Scans, their schedules and their history",
            Resource::Daemon => "Daemons and their settings",
            Resource::DaemonApiKey => {
                "The keys daemons authenticate with, and their install and upgrade commands"
            }
            Resource::Credential => "Credentials scans use to reach devices",
            Resource::Tag => "Tags and tag groups",
            Resource::Member => "The people in the organization and their pending invites",
            Resource::UserApiKey => "API keys you own",
            Resource::Organization => "The organization's name and its deletion",
            Resource::Billing => "The plan, payment method and invoices",
            Resource::License => "Self-hosted license keys",
            Resource::NewSite => "Adding a site to the organization",
        }
    }

    fn category(&self) -> &'static str {
        self.area().id()
    }

    fn metadata(&self) -> serde_json::Value {
        let display_actions: serde_json::Map<String, serde_json::Value> =
            UserOrgPermissions::iter()
                .map(|role| {
                    let actions = self.display_actions(role);
                    (role.id().to_string(), serde_json::json!(actions))
                })
                .collect();
        serde_json::json!({
            "per_site": self.keying() == Keying::Site,
            "display_actions": display_actions,
            "writable_when_lapsed": self.writable_when_lapsed(),
        })
    }
}

/// A value holding (resource, action) pairs. The only thing the evaluator reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Grants(BTreeSet<(Resource, Action)>);

impl FromIterator<(Resource, Action)> for Grants {
    fn from_iter<I: IntoIterator<Item = (Resource, Action)>>(pairs: I) -> Self {
        Self(pairs.into_iter().collect())
    }
}

impl Grants {
    pub fn allows(&self, resource: Resource, action: Action) -> bool {
        self.0.contains(&(resource, action))
    }

    pub fn union(&self, other: &Grants) -> Grants {
        Self(self.0.union(&other.0).copied().collect())
    }

    pub fn intersection(&self, other: &Grants) -> Grants {
        Self(self.0.intersection(&other.0).copied().collect())
    }

    pub fn is_subset_of(&self, other: &Grants) -> bool {
        self.0.is_subset(&other.0)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn pairs(&self) -> Vec<Grant> {
        self.0
            .iter()
            .map(|&(resource, action)| Grant { resource, action })
            .collect()
    }

    /// Every grant a role can hold at `keying`.
    fn every(keying: Keying) -> Grants {
        match keying {
            Keying::Organization => UserOrgPermissions::iter()
                .fold(Grants::default(), |every, role| every.union(&role.grants())),
            Keying::Site => {
                SiteRole::iter().fold(Grants::default(), |every, role| every.union(&role.grants()))
            }
        }
    }

    /// The no-escalation rule for users and invites, at one scope: `granted` is a strict subset of
    /// these grants, unless these are every grant at that scope. An Admin can't invite an Admin,
    /// and an Owner can grant Owner.
    pub fn may_grant_to_person(&self, granted: &Grants, keying: Keying) -> bool {
        granted.is_subset_of(self) && (granted != self || Grants::every(keying).is_subset_of(self))
    }
}

impl UserOrgPermissions {
    /// The organization grants of this role.
    pub fn grants(self) -> Grants {
        Resource::iter()
            .flat_map(|resource| {
                let actions = match resource.grant() {
                    RoleTable::Organization(table) => table(self),
                    RoleTable::Site(_) => NONE,
                };
                actions.iter().map(move |action| (resource, *action))
            })
            .collect()
    }
}

impl SiteRole {
    /// The grants of this role on a site.
    pub fn grants(self) -> Grants {
        Resource::iter()
            .flat_map(|resource| {
                let actions = match resource.grant() {
                    RoleTable::Site(table) => table(self),
                    RoleTable::Organization(_) => NONE,
                };
                actions.iter().map(move |action| (resource, *action))
            })
            .collect()
    }
}

/// Where a site role holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AssignmentScope {
    /// Every site in the organization, including sites created later.
    #[default]
    AllSites,
    /// One site.
    Site(Uuid),
}

/// A site role at a scope.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Assignment {
    pub role: SiteRole,
    pub scope: AssignmentScope,
}

impl Assignment {
    /// What `permissions` plus `site_ids` mean on a user or invite: an Owner or Admin reaches every
    /// site whatever the list says, and anyone else holds their role on the listed sites.
    pub fn for_person(role: UserOrgPermissions, site_ids: &[Uuid]) -> Vec<Self> {
        match role {
            UserOrgPermissions::Owner | UserOrgPermissions::Admin => vec![Self {
                role: SiteRole::Admin,
                scope: AssignmentScope::AllSites,
            }],
            UserOrgPermissions::Member | UserOrgPermissions::Viewer => {
                Self::for_api_key(role, Some(site_ids))
            }
        }
    }

    /// What `permissions` plus `site_ids` mean on an API key: its role on every site when
    /// `site_ids` is `None`, otherwise on the listed sites and no others, whatever the role.
    pub fn for_api_key(role: UserOrgPermissions, site_ids: Option<&[Uuid]>) -> Vec<Self> {
        let Some(site_ids) = site_ids else {
            return vec![Self {
                role: role.into(),
                scope: AssignmentScope::AllSites,
            }];
        };
        site_ids
            .iter()
            .map(|site_id| Self {
                role: role.into(),
                scope: AssignmentScope::Site(*site_id),
            })
            .collect()
    }
}

/// What a principal can do: its organization grants, and its grants on each site.
#[derive(Debug, Clone, Default, Eq)]
pub struct AccessSet {
    organization: Grants,
    sites: BTreeMap<Uuid, Grants>,
    /// What an all-sites assignment holds on every site, including sites created after this was
    /// loaded. Already counted in `sites`, so the evaluator doesn't read it.
    all_sites: Grants,
}

/// Two principals hold the same access when they hold the same grants, however assigned.
impl PartialEq for AccessSet {
    fn eq(&self, other: &Self) -> bool {
        self.organization == other.organization && self.sites == other.sites
    }
}

impl AccessSet {
    /// The access of a principal holding `role` and `assignments`, in an organization with
    /// `organization_site_ids`: what an all-sites assignment covers.
    pub fn new(
        role: UserOrgPermissions,
        assignments: &[Assignment],
        organization_site_ids: &[Uuid],
    ) -> Self {
        let mut sites: BTreeMap<Uuid, Grants> = BTreeMap::new();
        let mut all_sites = Grants::default();
        for assignment in assignments {
            let grants = assignment.role.grants();
            let covered = match assignment.scope {
                AssignmentScope::AllSites => {
                    all_sites = all_sites.union(&grants);
                    organization_site_ids
                }
                AssignmentScope::Site(ref site_id) => std::slice::from_ref(site_id),
            };
            for site_id in covered {
                let held = sites.entry(*site_id).or_default();
                *held = held.union(&grants);
            }
        }
        Self {
            organization: role.grants(),
            sites,
            all_sites,
        }
    }

    pub fn from_grants(organization: Grants, sites: BTreeMap<Uuid, Grants>) -> Self {
        Self {
            organization,
            sites,
            all_sites: Grants::default(),
        }
    }

    /// Whether the principal holds `action` on `resource`: in its organization grants for an
    /// org-keyed resource, and on any site for a site-keyed one.
    pub fn can(&self, resource: Resource, action: Action) -> bool {
        match resource.keying() {
            Keying::Organization => self.organization.allows(resource, action),
            Keying::Site => self
                .sites
                .values()
                .any(|grants| grants.allows(resource, action)),
        }
    }

    /// The sites holding `action` on `resource`. An org-keyed resource is held on none.
    pub fn site_ids(&self, resource: Resource, action: Action) -> Vec<Uuid> {
        match resource.keying() {
            Keying::Organization => Vec::new(),
            Keying::Site => self
                .sites
                .iter()
                .filter(|(_, grants)| grants.allows(resource, action))
                .map(|(site_id, _)| *site_id)
                .collect(),
        }
    }

    pub fn organization(&self) -> &Grants {
        &self.organization
    }

    /// Whether the principal holds `action` on `resource` through All sites, so on sites created
    /// after this was loaded too. An org-keyed resource is held on no site.
    pub fn all_sites_allow(&self, resource: Resource, action: Action) -> bool {
        resource.keying() == Keying::Site && self.all_sites.allows(resource, action)
    }

    /// This access, limited to what `owner` holds at each level. Compares grants, so it holds for
    /// roles with no order between them.
    pub fn capped_by(&self, owner: &AccessSet) -> AccessSet {
        let sites = self
            .sites
            .iter()
            .filter_map(|(site_id, grants)| {
                let capped = grants.intersection(owner.sites.get(site_id)?);
                (!capped.is_empty()).then_some((*site_id, capped))
            })
            .collect();
        Self {
            organization: self.organization.intersection(&owner.organization),
            sites,
            all_sites: self.all_sites.intersection(&owner.all_sites),
        }
    }

    /// This access as sent to the person holding it.
    pub fn to_grants(&self) -> AccessGrants {
        AccessGrants {
            organization: self.organization.pairs(),
            all_sites: self.all_sites.pairs(),
            sites: self
                .sites
                .iter()
                .map(|(site_id, grants)| SiteGrants {
                    site_id: *site_id,
                    grants: grants.pairs(),
                })
                .collect(),
        }
    }

    /// Whether this principal may give, or manage, a person holding `granted`: the no-escalation
    /// rule at the organization and on every site `granted` reaches.
    pub fn may_grant_to_person(&self, granted: &AccessSet) -> bool {
        let none = Grants::default();
        self.organization
            .may_grant_to_person(&granted.organization, Keying::Organization)
            && granted.sites.iter().all(|(site_id, grants)| {
                self.sites
                    .get(site_id)
                    .unwrap_or(&none)
                    .may_grant_to_person(grants, Keying::Site)
            })
    }

    /// Whether this principal may edit a person who already holds `target`: what it may grant,
    /// short of a peer holding the same access, so one Owner can't demote another.
    pub fn may_edit_person(&self, target: &AccessSet) -> bool {
        self != target && self.may_grant_to_person(target)
    }

    /// Whether this principal holds no site, so a site-keyed read returns nothing rather than
    /// being refused.
    pub fn on_no_site(&self) -> bool {
        self.sites.is_empty()
    }

    /// Whether this principal may give a key `granted`. A subset is enough, so a user can make a
    /// key at their own level. An All sites grant needs All sites: holding every site that exists
    /// today doesn't cover the sites created later.
    pub fn may_grant_to_key(&self, granted: &AccessSet) -> bool {
        let none = Grants::default();
        granted.organization.is_subset_of(&self.organization)
            && granted.all_sites.is_subset_of(&self.all_sites)
            && granted.sites.iter().all(|(site_id, grants)| {
                grants.is_subset_of(self.sites.get(site_id).unwrap_or(&none))
            })
    }
}

/// One action held on one resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
pub struct Grant {
    /// The resource the action applies to.
    pub resource: Resource,
    /// The action held on it.
    pub action: Action,
}

/// The grants held on one site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct SiteGrants {
    /// The site these grants hold on.
    pub site_id: Uuid,
    /// What the principal can do on that site.
    pub grants: Vec<Grant>,
}

/// What a principal can do: organization grants for org-keyed resources, and per-site grants for
/// site-keyed ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct AccessGrants {
    /// Grants that answer for org-keyed resources.
    pub organization: Vec<Grant>,
    /// Grants on each site the principal reaches. Empty for a principal on no site.
    pub sites: Vec<SiteGrants>,
    /// Grants held on every site, including sites created since this was read.
    pub all_sites: Vec<Grant>,
}

#[cfg(test)]
tokio::task_local! {
    /// The checks the current request made, for the "authorization recorded" test.
    pub static RECORDED_CHECKS: std::cell::RefCell<Vec<(Resource, Action)>>;
}

/// Notes a check on the current request, when a test is recording.
#[cfg(test)]
pub fn record_check(resource: Resource, action: Action) {
    let _ = RECORDED_CHECKS.try_with(|checks| checks.borrow_mut().push((resource, action)));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small seeded generator: the same grants on every run, with no new dependency.
    struct Seeded(u64);

    impl Seeded {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        /// A random subset of the pairs of the resources `keying` selects.
        fn grants(&mut self, keying: Keying) -> Grants {
            Resource::iter()
                .filter(|resource| resource.keying() == keying)
                .flat_map(|resource| Action::iter().map(move |action| (resource, action)))
                .filter(|_| self.next().is_multiple_of(2))
                .collect()
        }
    }

    const SEED: u64 = 0x9E37_79B9_7F4A_7C15;
    const ROUNDS: usize = 500;

    /// The comparison never promises more or less than the role table grants.
    #[test]
    fn displayed_actions_match_what_the_role_holds() {
        for resource in Resource::iter().filter(|r| *r != Resource::NewSite) {
            for role in UserOrgPermissions::iter() {
                let shown = resource.display_actions(role);
                let held = resource.actions(role);
                let shows = |action| shown.contains(&action);
                assert_eq!(
                    shows(DisplayAction::View),
                    held.contains(&Action::Read),
                    "{resource} {role}"
                );
                assert_eq!(
                    shows(DisplayAction::Edit),
                    held.contains(&Action::Write),
                    "{resource} {role}"
                );
                assert_eq!(
                    shows(DisplayAction::Delete),
                    held.contains(&Action::Delete),
                    "{resource} {role}"
                );
                if shows(DisplayAction::Create) {
                    let creates = if resource == Resource::Site {
                        Resource::NewSite
                    } else {
                        resource
                    };
                    assert!(
                        creates.actions(role).contains(&Action::Write),
                        "{resource} {role}"
                    );
                }
            }
        }
    }

    /// Creating a site shows as Create on Sites, for exactly the roles that can do it.
    #[test]
    fn new_sites_show_as_create_on_sites() {
        for role in UserOrgPermissions::iter() {
            assert!(Resource::NewSite.display_actions(role).is_empty());
            assert_eq!(
                Resource::Site
                    .display_actions(role)
                    .contains(&DisplayAction::Create),
                Resource::NewSite.actions(role).contains(&Action::Write),
                "{role}"
            );
        }
    }

    /// What the UI reads per site is what the evaluator answers.
    #[test]
    fn grants_sent_to_the_client_match_the_evaluator() {
        let mut seeded = Seeded(SEED);
        let sites = [Uuid::new_v4(), Uuid::new_v4()];
        for _ in 0..ROUNDS {
            let access = generated(&mut seeded, &sites);
            let sent = access.to_grants();
            for resource in Resource::iter() {
                for action in Action::iter() {
                    let grant = Grant { resource, action };
                    assert_eq!(
                        sent.organization.contains(&grant),
                        access.organization.allows(resource, action)
                    );
                    let sites_sent: Vec<Uuid> = sent
                        .sites
                        .iter()
                        .filter(|site| site.grants.contains(&grant))
                        .map(|site| site.site_id)
                        .collect();
                    if resource.keying() == Keying::Site {
                        assert_eq!(sites_sent, access.site_ids(resource, action));
                    }
                }
            }
        }
    }

    #[test]
    fn a_higher_role_holds_everything_a_lower_role_does() {
        use UserOrgPermissions::{Admin, Member, Owner, Viewer};
        for resource in Resource::iter() {
            for (lower, higher) in [(Viewer, Member), (Member, Admin), (Admin, Owner)] {
                for action in resource.actions(lower) {
                    assert!(
                        resource.actions(higher).contains(action),
                        "{higher} lacks {action} on {resource}, which {lower} holds"
                    );
                }
            }
        }
    }

    /// Each role's grants name only the resources its own level answers for.
    #[test]
    fn role_grants_stay_on_their_own_keying() {
        for resource in Resource::iter() {
            for action in Action::iter() {
                if resource.keying() == Keying::Site {
                    for role in UserOrgPermissions::iter() {
                        assert!(!role.grants().allows(resource, action));
                    }
                } else {
                    for role in SiteRole::iter() {
                        assert!(!role.grants().allows(resource, action));
                    }
                }
            }
        }
    }

    /// Whatever the grants at either level, org-keyed answers read only the organization grants
    /// and site-keyed answers only that site's.
    #[test]
    fn answers_come_only_from_the_grants_at_their_level() {
        let mut seeded = Seeded(SEED);
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        for _ in 0..ROUNDS {
            // The wrong keying at each level, to prove it is ignored.
            let organization = seeded
                .grants(Keying::Organization)
                .union(&seeded.grants(Keying::Site));
            let on_a = seeded
                .grants(Keying::Site)
                .union(&seeded.grants(Keying::Organization));
            let on_b = seeded.grants(Keying::Site);
            let access = AccessSet::from_grants(
                organization.clone(),
                BTreeMap::from([(a, on_a.clone()), (b, on_b.clone())]),
            );
            for resource in Resource::iter() {
                for action in Action::iter() {
                    match resource.keying() {
                        Keying::Organization => {
                            assert_eq!(
                                access.can(resource, action),
                                organization.allows(resource, action),
                                "{resource} {action}"
                            );
                            assert!(access.site_ids(resource, action).is_empty());
                        }
                        Keying::Site => {
                            let sites = access.site_ids(resource, action);
                            assert_eq!(
                                sites.contains(&a),
                                on_a.allows(resource, action),
                                "{resource} {action}"
                            );
                            assert_eq!(
                                sites.contains(&b),
                                on_b.allows(resource, action),
                                "{resource} {action}"
                            );
                            assert_eq!(access.can(resource, action), !sites.is_empty());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn an_all_sites_assignment_covers_every_site_in_the_organization() {
        let sites = vec![Uuid::new_v4(), Uuid::new_v4()];
        let assignments = Assignment::for_person(UserOrgPermissions::Admin, &[]);
        let access = AccessSet::new(UserOrgPermissions::Admin, &assignments, &sites);
        let mut expected = sites;
        expected.sort();
        assert_eq!(access.site_ids(Resource::Host, Action::Delete), expected);
    }

    /// A site created after the access loaded isn't in `sites`, so the client reads `all_sites`.
    #[test]
    fn only_an_all_sites_assignment_reaches_a_site_created_later() {
        let site = Uuid::new_v4();
        let admin = AccessSet::new(
            UserOrgPermissions::Admin,
            &Assignment::for_person(UserOrgPermissions::Admin, &[]),
            &[site],
        );
        let member = AccessSet::new(
            UserOrgPermissions::Member,
            &Assignment::for_person(UserOrgPermissions::Member, &[site]),
            &[site],
        );
        let host_delete = Grant {
            resource: Resource::Host,
            action: Action::Delete,
        };
        assert!(admin.to_grants().all_sites.contains(&host_delete));
        assert!(member.to_grants().all_sites.is_empty());
    }

    /// The same grants assigned two ways are peers, which `may_edit_person` depends on.
    #[test]
    fn access_compares_by_grants_not_by_how_they_were_assigned() {
        let site = Uuid::new_v4();
        let everywhere = Assignment {
            role: SiteRole::Admin,
            scope: AssignmentScope::AllSites,
        };
        let listed = Assignment {
            role: SiteRole::Admin,
            scope: AssignmentScope::Site(site),
        };
        let role = UserOrgPermissions::Admin;
        assert_eq!(
            AccessSet::new(role, &[everywhere], &[site]),
            AccessSet::new(role, &[listed], &[site])
        );
    }

    #[test]
    fn different_roles_on_different_sites_answer_per_site() {
        let (viewed, edited) = (Uuid::new_v4(), Uuid::new_v4());
        let scoped = |role, site_id| Assignment {
            role,
            scope: AssignmentScope::Site(site_id),
        };
        let access = AccessSet::new(
            UserOrgPermissions::Viewer,
            &[
                scoped(SiteRole::Viewer, viewed),
                scoped(SiteRole::Member, edited),
            ],
            &[],
        );
        assert_eq!(access.site_ids(Resource::Host, Action::Write), vec![edited]);
        assert_eq!(access.site_ids(Resource::Host, Action::Read).len(), 2);
    }

    #[test]
    fn assignments_on_one_site_combine() {
        let site = Uuid::new_v4();
        let access = AccessSet::new(
            UserOrgPermissions::Viewer,
            &[
                Assignment {
                    role: SiteRole::Viewer,
                    scope: AssignmentScope::AllSites,
                },
                Assignment {
                    role: SiteRole::Admin,
                    scope: AssignmentScope::Site(site),
                },
            ],
            &[site, Uuid::new_v4()],
        );
        assert_eq!(access.site_ids(Resource::Site, Action::Write), vec![site]);
        assert_eq!(access.site_ids(Resource::Host, Action::Read).len(), 2);
    }

    /// Generated access: organization grants plus grants on some of `sites`.
    fn generated(seeded: &mut Seeded, sites: &[Uuid]) -> AccessSet {
        let on_sites = sites
            .iter()
            .filter(|_| !seeded.next().is_multiple_of(4))
            .copied()
            .collect::<Vec<_>>();
        AccessSet::from_grants(
            seeded.grants(Keying::Organization),
            on_sites
                .into_iter()
                .map(|site_id| (site_id, seeded.grants(Keying::Site)))
                .collect(),
        )
    }

    /// True of every pair, including pairs with no order between them.
    #[test]
    fn a_capped_key_never_exceeds_its_owner() {
        let mut seeded = Seeded(SEED);
        let sites = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        let mut unordered = 0;
        for _ in 0..ROUNDS {
            let key = generated(&mut seeded, &sites);
            let owner = generated(&mut seeded, &sites);
            if !key.may_grant_to_key(&owner) && !owner.may_grant_to_key(&key) {
                unordered += 1;
            }
            let capped = key.capped_by(&owner);
            for resource in Resource::iter() {
                for action in Action::iter() {
                    if capped.can(resource, action) {
                        assert!(owner.can(resource, action), "{resource} {action}");
                        assert!(key.can(resource, action), "{resource} {action}");
                    }
                    for site_id in capped.site_ids(resource, action) {
                        assert!(owner.site_ids(resource, action).contains(&site_id));
                        assert!(key.site_ids(resource, action).contains(&site_id));
                    }
                }
            }
            assert!(owner.may_grant_to_key(&capped));
        }
        assert!(unordered > 0, "the generator produced only ordered pairs");
    }

    /// A pair is refused when it holds anything the caller doesn't, at either level.
    #[test]
    fn a_grant_beyond_the_callers_is_refused() {
        let mut seeded = Seeded(SEED);
        let sites = [Uuid::new_v4(), Uuid::new_v4()];
        for _ in 0..ROUNDS {
            let caller = generated(&mut seeded, &sites);
            let granted = generated(&mut seeded, &sites);
            let within = |pairs: &Grants, held: &Grants| pairs.is_subset_of(held);
            let none = Grants::default();
            let beyond = !within(&granted.organization, &caller.organization)
                || granted.sites.iter().any(|(site_id, grants)| {
                    !within(grants, caller.sites.get(site_id).unwrap_or(&none))
                });
            assert_eq!(caller.may_grant_to_key(&granted), !beyond);
            if beyond {
                assert!(!caller.may_grant_to_person(&granted));
            }
        }
    }

    /// Holding exactly what is granted lets a user make a key, and a person only with every grant.
    #[test]
    fn a_person_needs_strictly_more_unless_the_caller_holds_every_grant() {
        let mut seeded = Seeded(SEED);
        let site = Uuid::new_v4();
        for _ in 0..ROUNDS {
            let access = generated(&mut seeded, &[site]);
            assert!(access.may_grant_to_key(&access));
            assert!(!access.may_grant_to_person(&access));
        }
        let every = AccessSet::from_grants(
            Grants::every(Keying::Organization),
            BTreeMap::from([(site, Grants::every(Keying::Site))]),
        );
        assert!(every.may_grant_to_person(&every));
    }

    #[test]
    fn the_rule_holds_for_the_built_in_roles() {
        use UserOrgPermissions::{Admin, Member, Owner};
        let site = Uuid::new_v4();
        let person = |role| AccessSet::new(role, &Assignment::for_person(role, &[site]), &[site]);
        let key =
            |role| AccessSet::new(role, &Assignment::for_api_key(role, Some(&[site])), &[site]);

        assert!(!person(Admin).may_grant_to_person(&person(Admin)));
        assert!(!person(Admin).may_grant_to_person(&person(Owner)));
        assert!(person(Admin).may_grant_to_person(&person(Member)));
        assert!(person(Owner).may_grant_to_person(&person(Owner)));
        assert!(!person(Owner).may_edit_person(&person(Owner)));
        assert!(person(Owner).may_edit_person(&person(Admin)));
        assert!(person(Admin).may_grant_to_key(&key(Admin)));
        assert!(!person(Admin).may_grant_to_key(&key(Owner)));
    }

    /// An all-sites grant reaches sites a limited caller doesn't hold.
    #[test]
    fn a_limited_owner_key_cannot_grant_every_site() {
        use UserOrgPermissions::{Admin, Owner};
        let (held, other) = (Uuid::new_v4(), Uuid::new_v4());
        let limited = AccessSet::new(Owner, &Assignment::for_api_key(Owner, Some(&[held])), &[]);
        let admin = AccessSet::new(Admin, &Assignment::for_person(Admin, &[]), &[held, other]);
        assert!(!limited.may_grant_to_person(&admin));
    }

    /// Holding every site there is today isn't All sites: an All sites key also reaches the next
    /// site, which a caller with a site list wouldn't.
    #[test]
    fn only_all_sites_can_grant_an_all_sites_key() {
        use UserOrgPermissions::{Admin, Member};
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let every_listed = AccessSet::new(Member, &Assignment::for_person(Member, &[a, b]), &[]);
        let admin = AccessSet::new(Admin, &Assignment::for_person(Admin, &[]), &[a, b]);
        let all_sites_key = AccessSet::new(Member, &Assignment::for_api_key(Member, None), &[a, b]);
        let listed_key =
            AccessSet::new(Member, &Assignment::for_api_key(Member, Some(&[a, b])), &[]);

        assert!(!every_listed.may_grant_to_key(&all_sites_key));
        assert!(every_listed.may_grant_to_key(&listed_key));
        assert!(admin.may_grant_to_key(&all_sites_key));
    }

    #[test]
    fn capping_drops_what_a_demoted_owner_lost() {
        use UserOrgPermissions::{Admin, Member};
        let (kept, lost) = (Uuid::new_v4(), Uuid::new_v4());
        let key = AccessSet::new(
            Admin,
            &Assignment::for_api_key(Admin, Some(&[kept, lost])),
            &[],
        );
        let demoted = AccessSet::new(Member, &Assignment::for_person(Member, &[kept]), &[]);

        let capped = key.capped_by(&demoted);
        assert!(!capped.can(Resource::Credential, Action::Write));
        assert_eq!(capped.site_ids(Resource::Host, Action::Write), vec![kept]);
        assert!(capped.site_ids(Resource::Site, Action::Write).is_empty());
    }

    /// A key with no sites keeps its org role, and reaches no site.
    #[test]
    fn a_role_keeps_its_organization_grants_with_no_sites() {
        let access = AccessSet::new(UserOrgPermissions::Owner, &[], &[]);
        assert!(access.can(Resource::Billing, Action::Write));
        assert!(access.can(Resource::NewSite, Action::Write));
        assert!(!access.can(Resource::Site, Action::Read));
        assert!(access.site_ids(Resource::Site, Action::Read).is_empty());
    }
}
