use serde::{Deserialize, Serialize};
use std::hash::Hash;
use strum_macros::{EnumDiscriminants, EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;
use validator::Validate;

use crate::server::shared::{
    concepts::Concept,
    types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
    },
};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    IntoStaticStr,
    EnumDiscriminants,
    VariantNames,
    ToSchema,
)]
#[strum_discriminants(derive(IntoStaticStr, EnumIter))]
#[schema(title = "ServiceVirtualization")]
#[serde(tag = "type", content = "details")]
pub enum ServiceVirtualization {
    #[schema(title = "Docker")]
    Docker(DockerVirtualization),
    #[schema(title = "Podman")]
    Podman(PodmanVirtualization),
}

#[derive(Debug, Clone, Serialize, Validate, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct DockerVirtualization {
    /// Container name as reported by Docker.
    pub container_name: Option<String>,
    /// Docker container ID.
    pub container_id: Option<String>,
    /// Compose project the container belongs to, when it was started by Compose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compose_project: Option<String>,
}

#[derive(Debug, Clone, Serialize, Validate, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct PodmanVirtualization {
    /// Container name as reported by Podman.
    pub container_name: Option<String>,
    /// Podman container ID.
    pub container_id: Option<String>,
    /// Compose project the container belongs to, when it was started by Compose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compose_project: Option<String>,
}

impl ServiceVirtualization {
    // The runtime service that owns this container is `Service::virtualization_service_id`, a
    // real foreign key, rather than a field inside each payload — see the note on
    // `HostVirtualization`.

    /// Container id for any container-runtime variant (Docker, Podman, …).
    pub fn container_id(&self) -> Option<&str> {
        match self {
            Self::Docker(d) => d.container_id.as_deref(),
            Self::Podman(p) => p.container_id.as_deref(),
        }
    }

    /// Container name for any container-runtime variant (Docker, Podman, …).
    pub fn container_name(&self) -> Option<&str> {
        match self {
            Self::Docker(d) => d.container_name.as_deref(),
            Self::Podman(p) => p.container_name.as_deref(),
        }
    }

    /// Compose/pod project for any container-runtime variant (Docker, Podman, …).
    pub fn compose_project(&self) -> Option<&str> {
        match self {
            Self::Docker(d) => d.compose_project.as_deref(),
            Self::Podman(p) => p.compose_project.as_deref(),
        }
    }

    /// Service-definition id of this container runtime (e.g. `"Docker"`, `"Podman"`).
    /// Used as the logo for a deployment-unit (Stack) grouping container. The variant
    /// discriminant matches the runtime's `ServiceDefinition` id by construction.
    pub fn runtime_service_definition_id(&self) -> &'static str {
        self.into()
    }
}

impl HasId for ServiceVirtualizationDiscriminants {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for ServiceVirtualizationDiscriminants {
    fn color(&self) -> Color {
        Concept::Containerization.color()
    }
    fn icon(&self) -> Icon {
        Concept::Containerization.icon()
    }
}

impl TypeMetadataProvider for ServiceVirtualizationDiscriminants {
    fn name(&self) -> &'static str {
        match self {
            Self::Docker => "Docker",
            Self::Podman => "Podman",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Docker => "Runs in a Docker container",
            Self::Podman => "Runs in a Podman container",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_virtualization_serde_round_trip_with_compose_project() {
        let virt = ServiceVirtualization::Docker(DockerVirtualization {
            container_name: Some("plex".to_string()),
            container_id: Some("abc123".to_string()),
            compose_project: Some("media-stack".to_string()),
        });

        let json = serde_json::to_string(&virt).unwrap();
        let deserialized: ServiceVirtualization = serde_json::from_str(&json).unwrap();
        assert_eq!(virt, deserialized);

        // Verify compose_project is present in serialized output
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            value["details"]["compose_project"],
            serde_json::json!("media-stack")
        );
    }

    #[test]
    fn docker_virtualization_serde_round_trip_without_compose_project() {
        let virt = ServiceVirtualization::Docker(DockerVirtualization {
            container_name: Some("nginx".to_string()),
            container_id: Some("def456".to_string()),
            compose_project: None,
        });

        let json = serde_json::to_string(&virt).unwrap();
        let deserialized: ServiceVirtualization = serde_json::from_str(&json).unwrap();
        assert_eq!(virt, deserialized);

        // Verify compose_project is omitted when None
        assert!(!json.contains("compose_project"));
    }

    #[test]
    fn docker_virtualization_backward_compat_missing_compose_project() {
        // Simulate old JSONB data without compose_project
        let old_json = r#"{
            "type": "Docker",
            "details": {
                "container_name": "redis",
                "container_id": "old123",
                "service_id": "00000000-0000-0000-0000-000000000000"
            }
        }"#;

        let deserialized: ServiceVirtualization = serde_json::from_str(old_json).unwrap();
        match deserialized {
            ServiceVirtualization::Docker(d) => {
                assert_eq!(d.container_name, Some("redis".to_string()));
                assert_eq!(d.compose_project, None);
            }
            other => panic!("expected Docker variant, got {other:?}"),
        }
    }
}
