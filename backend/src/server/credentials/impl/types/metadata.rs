use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::{
    services::r#impl::definitions::{ServiceDefinition, ServiceDefinitionExt},
    shared::{
        concepts::Concept,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
};

use super::{CredentialType, CredentialTypeDiscriminants, SecretValue, default_docker_port};

/// Category grouping for credential types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, IntoStaticStr, ToSchema, PartialEq, Eq)]
pub enum CredentialCategory {
    /// Network monitoring protocols (SNMP, NetFlow, sFlow)
    #[strum(serialize = "Network Monitoring")]
    NetworkMonitoring,
    /// Container and virtualization platforms (Docker, vSphere, ESXi)
    #[strum(serialize = "Container & Virtualization")]
    ContainerVirtualization,
}

/// How a credential is scoped to targets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum ScopeModel {
    /// Network default — try on all hosts with matching open ports
    Broadcast,
    /// Assigned to specific hosts only
    PerHost,
}

/// A credential assigned to a host, optionally limited to specific interfaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct CredentialAssignment {
    pub credential_id: Uuid,
    /// Interface IDs to limit this credential to. None = all host interfaces.
    #[serde(default)]
    #[schema(required)]
    pub interface_ids: Option<Vec<Uuid>>,
}

impl CredentialTypeDiscriminants {
    /// Create a `CredentialType` instance with default field values for this variant.
    /// Used by `generate-fixtures` and anywhere variant iteration is needed.
    pub fn to_credential_type(&self) -> CredentialType {
        match self {
            Self::SnmpV2c => CredentialType::SnmpV2c {
                community: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
            },
            Self::DockerProxy => CredentialType::DockerProxy {
                port: default_docker_port(),
                path: None,
                ssl_cert: None,
                ssl_key: None,
                ssl_chain: None,
            },
        }
    }
}

impl HasId for CredentialTypeDiscriminants {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for CredentialTypeDiscriminants {
    fn color(&self) -> Color {
        // Derive color from associated service's category
        let service = self.to_credential_type().associated_service();
        ServiceDefinition::category(&*service).color()
    }
    fn icon(&self) -> Icon {
        // Fallback icon when the service logo is unavailable
        match self {
            Self::SnmpV2c => Concept::SNMP.icon(),
            Self::DockerProxy => Concept::Virtualization.icon(),
        }
    }
}

impl TypeMetadataProvider for CredentialTypeDiscriminants {
    fn name(&self) -> &'static str {
        match self {
            Self::SnmpV2c => "SNMP v2c",
            Self::DockerProxy => "Docker Proxy",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::SnmpV2c => "SNMPv2c community string for querying network devices",
            Self::DockerProxy => "Docker API proxy credentials. TLS is optional.",
        }
    }

    fn category(&self) -> &'static str {
        self.to_credential_type().credential_category().into()
    }

    fn metadata(&self) -> serde_json::Value {
        let ct = self.to_credential_type();
        let service = ct.associated_service();
        serde_json::json!({
            "fields": ct.field_definitions(),
            "scope_models": ct.scope_models(),
            "associated_service": ServiceDefinition::name(&*service),
            "has_logo": service.has_logo(),
            "logo_url": service.logo_url(),
            "logo_needs_white_background": service.logo_needs_white_background(),
        })
    }
}
