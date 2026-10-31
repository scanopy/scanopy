//! Whether a container or VM is running, as its runtime or hypervisor reported it at the last scan.

use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;

use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};

/// A guest's run state. Stored rows from before the state was recorded read as `Running`: Docker
/// and Podman discovery listed running containers only, so every stored container was running.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    IntoStaticStr,
    EnumIter,
    VariantNames,
    ToSchema,
)]
pub enum GuestState {
    #[default]
    Running,
    Paused,
    Restarting,
    Stopped,
}

impl GuestState {
    /// A Docker or Podman container's `State`, as `docker ps -a` reports it.
    pub fn from_container_state(state: &str) -> Self {
        match state {
            "running" => Self::Running,
            "paused" => Self::Paused,
            "restarting" => Self::Restarting,
            // created, exited, dead, removing
            _ => Self::Stopped,
        }
    }

    /// A libvirt domain's state, as `virsh domstate` prints it. A suspended-to-memory
    /// (`pmsuspended`) domain is paused; one shut off, crashed or dying is stopped.
    pub fn from_libvirt_state(state: &str) -> Self {
        match state.trim() {
            "running" | "idle" | "in shutdown" => Self::Running,
            "paused" | "pmsuspended" => Self::Paused,
            _ => Self::Stopped,
        }
    }

    /// A Proxmox guest's `status`, as `/cluster/resources` reports it.
    pub fn from_proxmox_status(status: Option<&str>) -> Self {
        match status {
            Some("running") => Self::Running,
            Some("paused" | "suspended") => Self::Paused,
            _ => Self::Stopped,
        }
    }
}

impl HasId for GuestState {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for GuestState {
    fn color(&self) -> Color {
        match self {
            Self::Running => Color::Green,
            Self::Paused => Color::Yellow,
            Self::Restarting => Color::Blue,
            Self::Stopped => Color::Gray,
        }
    }
    fn icon(&self) -> Icon {
        match self {
            Self::Running => Icon::CirclePlay,
            Self::Paused => Icon::CirclePause,
            Self::Restarting => Icon::RefreshCw,
            Self::Stopped => Icon::CircleStop,
        }
    }
}

impl TypeMetadataProvider for GuestState {
    fn name(&self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Paused => "Paused",
            Self::Restarting => "Restarting",
            Self::Stopped => "Stopped",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Running => "Running at the last scan",
            Self::Paused => "Paused or suspended at the last scan",
            Self::Restarting => "Restarting at the last scan",
            Self::Stopped => "Stopped at the last scan",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_container_that_is_not_running_paused_or_restarting_is_stopped() {
        for state in ["created", "exited", "dead", "removing"] {
            assert_eq!(GuestState::from_container_state(state), GuestState::Stopped);
        }
        assert_eq!(
            GuestState::from_container_state("paused"),
            GuestState::Paused
        );
    }
}
