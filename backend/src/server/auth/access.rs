//! What a principal may do: per resource type, per action, per site.
//!
//! A [`Resource`] is one thing an admin thinks about, so one per tab; `E::resource()` maps every
//! entity onto one. A role is the set of (resource, actions) in [`Resource::actions`]. An
//! [`Assignment`] gives a principal a role at a scope, and an [`AccessSet`] answers for all of a
//! principal's assignments at once.

use strum::{Display, EnumIter};
use uuid::Uuid;

use crate::server::users::r#impl::permissions::UserOrgPermissions;

/// What a role grants access to. Child entities share their parent's resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumIter)]
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
}

/// What a role may do to a resource. `Write` is create and update, plus operations such as
/// starting a scan or merging hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum Action {
    Read,
    Write,
    Delete,
}

/// What a resource's rows belong to, which decides what a site-scoped assignment reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keying {
    /// A site: an assignment reaches the rows on the sites it covers.
    Site,
    /// The organization: `Read` comes from any assignment, and `Write` and `Delete` need one that
    /// isn't limited to a site, so a site-scoped person can't change what other sites share.
    Organization,
}

const NONE: &[Action] = &[];
const R: &[Action] = &[Action::Read];
const RW: &[Action] = &[Action::Read, Action::Write];
const RWD: &[Action] = &[Action::Read, Action::Write, Action::Delete];

impl Resource {
    pub fn keying(self) -> Keying {
        match self {
            Resource::Site
            | Resource::Host
            | Resource::Service
            | Resource::Subnet
            | Resource::Vlan
            | Resource::Dependency
            | Resource::Topology
            | Resource::Share
            | Resource::Discovery
            | Resource::Daemon
            | Resource::DaemonApiKey => Keying::Site,
            Resource::Credential
            | Resource::Tag
            | Resource::Member
            | Resource::UserApiKey
            | Resource::Organization
            | Resource::Billing
            | Resource::License => Keying::Organization,
        }
    }

    /// The role table. Viewer sees the network, Member keeps it documented, Admin decides how the
    /// org runs, Owner answers for the org.
    pub fn actions(self, role: UserOrgPermissions) -> &'static [Action] {
        use UserOrgPermissions::{Admin, Member, Owner, Viewer};
        match self {
            Resource::Host
            | Resource::Service
            | Resource::Subnet
            | Resource::Vlan
            | Resource::Dependency
            | Resource::Topology
            | Resource::Share
            | Resource::Discovery => match role {
                Viewer => R,
                Member | Admin | Owner => RWD,
            },
            Resource::Tag => match role {
                Viewer => R,
                Member => RW,
                Admin | Owner => RWD,
            },
            Resource::Site | Resource::Daemon | Resource::Credential => match role {
                Viewer | Member => R,
                Admin | Owner => RWD,
            },
            Resource::DaemonApiKey => match role {
                Viewer => NONE,
                Member => R,
                Admin | Owner => RWD,
            },
            Resource::Member => match role {
                Viewer | Member => NONE,
                Admin | Owner => RWD,
            },
            Resource::UserApiKey => match role {
                Viewer | Member | Admin | Owner => RWD,
            },
            Resource::Organization => match role {
                Viewer | Member | Admin => R,
                Owner => RWD,
            },
            Resource::Billing | Resource::License => match role {
                Viewer | Member | Admin => NONE,
                Owner => RW,
            },
        }
    }
}

/// Where a role holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentScope {
    /// Every site in the organization, including sites created later.
    AllSites,
    /// One site.
    Site(Uuid),
    /// Organization-keyed resources, and no site. `users.permissions` and
    /// `user_api_keys.permissions` hold at this level whatever the site list says, which neither
    /// of the other scopes can state.
    Organization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assignment {
    pub role: UserOrgPermissions,
    pub scope: AssignmentScope,
}

impl Assignment {
    /// Whether this assignment grants `action` on `resource` somewhere.
    fn allows(&self, resource: Resource, action: Action) -> bool {
        let limited_to_a_site = matches!(self.scope, AssignmentScope::Site(_));
        let org_keyed_change =
            resource.keying() == Keying::Organization && action != Action::Read;
        resource.actions(self.role).contains(&action) && !(limited_to_a_site && org_keyed_change)
    }
}

/// What a principal can do, from its assignments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccessSet {
    assignments: Vec<Assignment>,
    /// Every site in the organization: what an all-sites assignment covers.
    organization_site_ids: Vec<Uuid>,
}

impl AccessSet {
    /// A user's access from `users.permissions` and `site_ids`: every site in the organization
    /// for an Owner or Admin, and the rows of `user_site_access` for anyone else.
    pub fn for_user(role: UserOrgPermissions, site_ids: Vec<Uuid>) -> Self {
        match role {
            UserOrgPermissions::Owner | UserOrgPermissions::Admin => Self {
                assignments: vec![Assignment {
                    role,
                    scope: AssignmentScope::AllSites,
                }],
                organization_site_ids: site_ids,
            },
            UserOrgPermissions::Member | UserOrgPermissions::Viewer => {
                Self::for_listed_sites(role, &site_ids)
            }
        }
    }

    /// A key's access from `user_api_keys.permissions` and the rows of
    /// `user_api_key_site_access`.
    pub fn for_api_key(role: UserOrgPermissions, site_ids: Vec<Uuid>) -> Self {
        Self::for_listed_sites(role, &site_ids)
    }

    fn for_listed_sites(role: UserOrgPermissions, site_ids: &[Uuid]) -> Self {
        let scopes = std::iter::once(AssignmentScope::Organization)
            .chain(site_ids.iter().copied().map(AssignmentScope::Site));
        Self {
            assignments: scopes.map(|scope| Assignment { role, scope }).collect(),
            organization_site_ids: Vec::new(),
        }
    }

    #[cfg(test)]
    pub fn from_assignments(assignments: Vec<Assignment>, organization_site_ids: Vec<Uuid>) -> Self {
        Self {
            assignments,
            organization_site_ids,
        }
    }

    /// Whether the principal holds `action` on `resource` anywhere.
    pub fn can(&self, resource: Resource, action: Action) -> bool {
        self.assignments
            .iter()
            .any(|assignment| assignment.allows(resource, action))
    }

    /// The sites where `action` on `resource` is allowed.
    pub fn site_ids(&self, resource: Resource, action: Action) -> Vec<Uuid> {
        let mut site_ids: Vec<Uuid> = self
            .assignments
            .iter()
            .filter(|assignment| assignment.allows(resource, action))
            .flat_map(|assignment| match assignment.scope {
                AssignmentScope::AllSites => self.organization_site_ids.clone(),
                AssignmentScope::Site(site_id) => vec![site_id],
                AssignmentScope::Organization => Vec::new(),
            })
            .collect();
        site_ids.sort();
        site_ids.dedup();
        site_ids
    }
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
    use strum::IntoEnumIterator;

    fn on_site(role: UserOrgPermissions, site_id: Uuid) -> AccessSet {
        AccessSet::from_assignments(
            vec![Assignment {
                role,
                scope: AssignmentScope::Site(site_id),
            }],
            vec![],
        )
    }

    /// Read, Read plus Write, and all three: the only cells the role comparison can show.
    #[test]
    fn every_role_cell_is_none_view_edit_or_full() {
        for resource in Resource::iter() {
            for role in UserOrgPermissions::iter() {
                let actions = resource.actions(role);
                assert!(
                    [NONE, R, RW, RWD].contains(&actions),
                    "{resource} {role}: {actions:?}"
                );
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

    #[test]
    fn a_site_assignment_reaches_only_its_site() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let access = on_site(UserOrgPermissions::Owner, a);
        for resource in Resource::iter().filter(|r| r.keying() == Keying::Site) {
            assert_eq!(access.site_ids(resource, Action::Read), vec![a]);
            assert!(!access.site_ids(resource, Action::Read).contains(&b));
        }
    }

    #[test]
    fn a_site_assignment_reads_but_cannot_change_org_keyed_resources() {
        let access = on_site(UserOrgPermissions::Owner, Uuid::new_v4());
        for resource in Resource::iter().filter(|r| r.keying() == Keying::Organization) {
            assert!(access.can(resource, Action::Read), "{resource}");
            assert!(!access.can(resource, Action::Write), "{resource}");
            assert!(!access.can(resource, Action::Delete), "{resource}");
        }
    }

    #[test]
    fn an_all_sites_assignment_changes_org_keyed_resources_and_covers_every_site() {
        let sites = vec![Uuid::new_v4(), Uuid::new_v4()];
        let access = AccessSet::for_user(UserOrgPermissions::Admin, sites.clone());
        assert!(access.can(Resource::Credential, Action::Write));
        let mut expected = sites;
        expected.sort();
        assert_eq!(access.site_ids(Resource::Host, Action::Delete), expected);
    }

    #[test]
    fn different_roles_on_different_sites_answer_per_site() {
        let (viewed, edited) = (Uuid::new_v4(), Uuid::new_v4());
        let scoped = |role, site_id| Assignment {
            role,
            scope: AssignmentScope::Site(site_id),
        };
        let access = AccessSet::from_assignments(
            vec![
                scoped(UserOrgPermissions::Viewer, viewed),
                scoped(UserOrgPermissions::Member, edited),
            ],
            vec![],
        );
        assert_eq!(access.site_ids(Resource::Host, Action::Write), vec![edited]);
        assert_eq!(access.site_ids(Resource::Host, Action::Read).len(), 2);
    }

    #[test]
    fn a_listed_role_keeps_its_org_level_actions_with_no_sites() {
        let access = AccessSet::for_api_key(UserOrgPermissions::Owner, vec![]);
        assert!(access.can(Resource::Billing, Action::Write));
        assert!(access.can(Resource::Site, Action::Write));
        assert!(access.site_ids(Resource::Site, Action::Read).is_empty());
    }
}
