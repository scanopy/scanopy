use crate::server::auth::access::{AccessSet, Action, Resource};
use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// An event on the live-update stream (`GET /api/v1/live-updates/stream`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(tag = "type", content = "data")]
pub enum LiveUpdate {
    /// Progress or an end state of a discovery session.
    #[schema(title = "Discovery")]
    Discovery(Box<DiscoveryUpdatePayload>),
    /// A site's live entity set changed, so its topology needs refetching.
    #[schema(title = "Topology")]
    Topology(LiveTopologyUpdate),
    /// A daemon checked in with a new version or health, so the daemons list and Home need refetching.
    #[schema(title = "Daemon")]
    Daemon(LiveDaemonUpdate),
}

/// A site whose live topology changed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct LiveTopologyUpdate {
    /// The site whose live entities changed.
    pub site_id: Uuid,
}

/// A daemon whose version or health changed when it checked in.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct LiveDaemonUpdate {
    /// The daemon that checked in.
    pub id: Uuid,
    /// The daemon's site, which decides who receives the update.
    pub site_id: Uuid,
}

impl LiveUpdate {
    pub fn site_id(&self) -> Uuid {
        match self {
            LiveUpdate::Discovery(update) => update.site_id,
            LiveUpdate::Topology(update) => update.site_id,
            LiveUpdate::Daemon(update) => update.site_id,
        }
    }

    /// The resource this event reports on.
    fn resource(&self) -> Resource {
        match self {
            LiveUpdate::Discovery(_) => Resource::Discovery,
            LiveUpdate::Topology(_) => Resource::Topology,
            LiveUpdate::Daemon(_) => Resource::Daemon,
        }
    }

    /// Whether a subscriber should get this event: they must be able to read its resource on its
    /// site, and topology events need the topology view on their plan.
    pub fn is_visible_to(&self, access: &AccessSet, topology_on_plan: bool) -> bool {
        if matches!(self, LiveUpdate::Topology(_)) && !topology_on_plan {
            return false;
        }
        access
            .site_ids(self.resource(), Action::Read)
            .contains(&self.site_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::discovery::r#impl::types::DiscoveryType;
    use crate::server::users::r#impl::permissions::UserOrgPermissions;

    fn viewer_of(site_id: Uuid) -> AccessSet {
        crate::tests::access_on(UserOrgPermissions::Viewer, &[site_id])
    }

    fn discovery(site_id: Uuid) -> LiveUpdate {
        LiveUpdate::Discovery(Box::new(DiscoveryUpdatePayload::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            site_id,
            DiscoveryType::default(),
            None,
        )))
    }

    fn topology(site_id: Uuid) -> LiveUpdate {
        LiveUpdate::Topology(LiveTopologyUpdate { site_id })
    }

    fn daemon(site_id: Uuid) -> LiveUpdate {
        LiveUpdate::Daemon(LiveDaemonUpdate {
            id: Uuid::new_v4(),
            site_id,
        })
    }

    #[test]
    fn subscribers_get_only_their_sites() {
        let mine = Uuid::new_v4();
        let other = Uuid::new_v4();

        for update in [discovery(mine), topology(mine), daemon(mine)] {
            assert!(update.is_visible_to(&viewer_of(mine), true));
        }
        for update in [discovery(other), topology(other), daemon(other)] {
            assert!(!update.is_visible_to(&viewer_of(mine), true));
        }
    }

    #[test]
    fn a_plan_without_topology_gets_discovery_only() {
        let site = Uuid::new_v4();

        assert!(discovery(site).is_visible_to(&viewer_of(site), false));
        assert!(!topology(site).is_visible_to(&viewer_of(site), false));
    }
}
