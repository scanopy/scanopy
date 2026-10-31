//! What a principal may do: per resource type, per action, per site.
//!
//! A [`Resource`] is one thing an admin thinks about, so one per tab; `E::resource()` maps every
//! entity onto one. [`Grants`] are (resource, action) pairs, and a role is a source of them. A
//! principal holds one organization role, which answers for org-keyed resources, and any number of
//! [`Assignment`]s, each a [`SiteRole`] at a scope, which answer for site-keyed resources. An
//! [`AccessSet`] holds the resulting grants and is all the evaluator reads.

use std::collections::{BTreeMap, BTreeSet};

use strum::{Display, EnumIter, EnumString, IntoEnumIterator};
use uuid::Uuid;

use crate::server::users::r#impl::permissions::UserOrgPermissions;

/// What a role grants access to. Child entities share their parent's resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display, EnumIter)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display, EnumIter)]
pub enum Action {
    Read,
    Write,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumIter, EnumString)]
pub enum SiteRole {
    Admin,
    Member,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentScope {
    /// Every site in the organization, including sites created later.
    AllSites,
    /// One site.
    Site(Uuid),
}

/// A site role at a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
                Self::for_api_key(role, site_ids)
            }
        }
    }

    /// What `permissions` plus `site_ids` mean on an API key: its role on the listed sites, and no
    /// others, whatever the role.
    pub fn for_api_key(role: UserOrgPermissions, site_ids: &[Uuid]) -> Vec<Self> {
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccessSet {
    organization: Grants,
    sites: BTreeMap<Uuid, Grants>,
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
        for assignment in assignments {
            let grants = assignment.role.grants();
            let covered = match assignment.scope {
                AssignmentScope::AllSites => organization_site_ids,
                AssignmentScope::Site(ref site_id) => std::slice::from_ref(site_id),
            };
            for site_id in covered {
                let held = sites.entry(*site_id).or_default();
                *held = held.union(&grants);
            }
        }
        Self::from_grants(role.grants(), sites)
    }

    pub fn from_grants(organization: Grants, sites: BTreeMap<Uuid, Grants>) -> Self {
        Self {
            organization,
            sites,
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
                .filter(|_| self.next() % 2 == 0)
                .collect()
        }
    }

    const SEED: u64 = 0x9E37_79B9_7F4A_7C15;
    const ROUNDS: usize = 500;

    /// Read, Read plus Write, all three, or Write alone: the cells the role comparison can show.
    #[test]
    fn every_role_cell_is_none_view_edit_or_full() {
        for resource in Resource::iter() {
            for role in UserOrgPermissions::iter() {
                let actions = resource.actions(role);
                assert!(
                    [NONE, R, W, RW, RWD].contains(&actions),
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
