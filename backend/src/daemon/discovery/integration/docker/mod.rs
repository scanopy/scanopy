//! Docker discovery integration.
//!
//! Thin wrapper over the shared container machinery in
//! [`super::container`], selecting [`ContainerRuntime::Docker`]. Two transports:
//! - proxy — HTTP(S) proxy URL (remote or local), `DockerProxy` credential
//! - socket — local Unix socket (`/var/run/docker.sock`), `DockerSocket` credential

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::server::credentials::r#impl::mapping::{
    ContainerProxyQueryCredential, CredentialMapping, CredentialQueryPayload,
    CredentialQueryPayloadDiscriminants, ResolvedCredential,
};
use crate::server::ports::r#impl::base::PortType;

use super::container::{self, CONTAINER_SCAN_TIMEOUT, ContainerRuntime};
use super::{
    Checkpoint, Completeness, DiscoveryIntegration, IntegrationContext, IntegrationFailure,
    ProbeContext, ProbeFailure, ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;

pub struct DockerIntegration;

#[async_trait]
impl DiscoveryIntegration for DockerIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::DockerProxy
    }

    fn estimated_seconds(&self) -> u32 {
        5
    }

    fn timeout(&self) -> Duration {
        CONTAINER_SCAN_TIMEOUT
    }

    /// The container probe retries its connection internally, well past the default budget.
    fn probe_timeout(&self) -> Duration {
        CONTAINER_SCAN_TIMEOUT
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        container::probe_proxy(ctx, ContainerRuntime::Docker).await
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
        checkpoint: &Checkpoint<'_>,
    ) -> Result<Completeness, IntegrationFailure> {
        container::execute(ctx, host_data, checkpoint, ContainerRuntime::Docker).await
    }
}

pub struct DockerSocketIntegration;

#[async_trait]
impl DiscoveryIntegration for DockerSocketIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::DockerSocket
    }

    fn estimated_seconds(&self) -> u32 {
        5
    }

    fn timeout(&self) -> Duration {
        CONTAINER_SCAN_TIMEOUT
    }

    /// The container probe retries its connection internally, well past the default budget.
    fn probe_timeout(&self) -> Duration {
        CONTAINER_SCAN_TIMEOUT
    }

    // No probe_gate_ports — a local Unix socket or named pipe, no TCP port needed.

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        // Explicit socket_path from the credential repoints the socket; None → bollard Docker
        // defaults (DOCKER_HOST or /var/run/docker.sock on Unix, the docker_engine pipe on Windows).
        let socket_path = match ctx.credential {
            CredentialQueryPayload::DockerSocket(c) => c
                .socket_path
                .as_ref()
                .filter(|s| !s.is_blank())
                .map(|s| s.as_str().to_string()),
            _ => None,
        };
        container::probe_socket(ctx, ContainerRuntime::Docker, socket_path).await
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
        checkpoint: &Checkpoint<'_>,
    ) -> Result<Completeness, IntegrationFailure> {
        container::execute(ctx, host_data, checkpoint, ContainerRuntime::Docker).await
    }
}

// ============================================================================
// Docker-specific credential resolution helpers
// ============================================================================

/// Extract all Docker credentials indexed by target IP.
/// Returns credentials for all IPs that have DockerProxy mappings.
pub fn resolve_docker_credentials(
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
) -> HashMap<IpAddr, ResolvedCredential<ContainerProxyQueryCredential>> {
    let mut result = HashMap::new();

    for mapping in credential_mappings {
        for override_entry in &mapping.ip_overrides {
            if let CredentialQueryPayload::DockerProxy(_) = &override_entry.credential {
                match override_entry.credential.resolve_file_paths() {
                    Ok(CredentialQueryPayload::DockerProxy(resolved)) => {
                        result.insert(
                            override_entry.ip,
                            ResolvedCredential {
                                credential: resolved,
                                credential_id: if override_entry.credential_id != Uuid::nil() {
                                    Some(override_entry.credential_id)
                                } else {
                                    None
                                },
                            },
                        );
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!(error = %e, ip = %override_entry.ip, "Failed to resolve Docker credential file paths");
                    }
                }
            }
        }
    }

    result
}
