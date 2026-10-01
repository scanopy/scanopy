//! Podman discovery integration.
//!
//! Thin wrapper over the shared container machinery in [`super::container`],
//! selecting [`ContainerRuntime::Podman`]. Podman exposes a Docker-compatible
//! REST API, so the same scanner and transports serve it. Two transports:
//! - proxy — HTTP(S) proxy URL (remote or local), `PodmanProxy` credential
//! - socket — local Unix socket (`/run/podman/podman.sock` or rootless
//!   `$XDG_RUNTIME_DIR/podman/podman.sock`), `PodmanSocket` credential

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

use crate::server::credentials::r#impl::mapping::{
    CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
};
use crate::server::ports::r#impl::base::PortType;

use super::container::{self, CONTAINER_SCAN_TIMEOUT, ContainerRuntime};
use super::{
    Checkpoint, Completeness, DiscoveryIntegration, IntegrationContext, IntegrationFailure,
    InterfaceViewScope, ProbeContext, ProbeFailure, ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;

/// Resolve the Podman API socket when the credential names none, returning the first that exists.
///
/// Honors `CONTAINER_HOST` (Podman's `DOCKER_HOST` analog) first. Then, on Unix, the rootful
/// (`/run/podman/podman.sock`) and rootless (`$XDG_RUNTIME_DIR/podman/podman.sock`) sockets; on
/// Windows, Podman machine's own pipe (`\\.\pipe\podman-machine-default`). Not `docker_engine`:
/// Podman only takes that pipe when Docker is absent, and probing it as Podman could reach Docker.
/// Returns `None` if nothing is present (the probe then fails cleanly).
pub fn resolve_podman_socket_path() -> Option<String> {
    if let Ok(host) = std::env::var("CONTAINER_HOST") {
        let path = host
            .strip_prefix("unix://")
            .or_else(|| host.strip_prefix("npipe://"));
        if let Some(path) = path
            && std::path::Path::new(path).exists()
        {
            return Some(path.to_string());
        }
    }

    let candidates: Vec<String> = if cfg!(windows) {
        vec![r"\\.\pipe\podman-machine-default".to_string()]
    } else {
        let mut c = vec!["/run/podman/podman.sock".to_string()];
        if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
            c.push(format!("{}/podman/podman.sock", xdg.trim_end_matches('/')));
        }
        c
    };
    candidates
        .into_iter()
        .find(|p| std::path::Path::new(p).exists())
}

pub struct PodmanIntegration;

#[async_trait]
impl DiscoveryIntegration for PodmanIntegration {
    /// Container runtimes report networks and containers, never the host's ports.
    fn interface_view_scope(&self) -> InterfaceViewScope {
        InterfaceViewScope::NoInterfaces
    }

    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::PodmanProxy
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

    fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
        match credential {
            CredentialQueryPayload::PodmanProxy(podman) => vec![PortType::new_tcp(podman.port)],
            _ => vec![],
        }
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        container::probe_proxy(ctx, ContainerRuntime::Podman).await
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
        checkpoint: &Checkpoint<'_>,
    ) -> Result<Completeness, IntegrationFailure> {
        container::execute(ctx, host_data, checkpoint, ContainerRuntime::Podman).await
    }
}

pub struct PodmanSocketIntegration;

#[async_trait]
impl DiscoveryIntegration for PodmanSocketIntegration {
    /// Container runtimes report networks and containers, never the host's ports.
    fn interface_view_scope(&self) -> InterfaceViewScope {
        InterfaceViewScope::NoInterfaces
    }

    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::PodmanSocket
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
        // Explicit socket_path from the credential wins; otherwise auto-detect the rootful /
        // rootless Podman socket.
        let socket_path = match ctx.credential {
            CredentialQueryPayload::PodmanSocket(c) => c
                .socket_path
                .as_ref()
                .filter(|s| !s.is_blank())
                .map(|s| s.as_str().to_string()),
            _ => None,
        }
        .or_else(resolve_podman_socket_path);
        container::probe_socket(ctx, ContainerRuntime::Podman, socket_path).await
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
        checkpoint: &Checkpoint<'_>,
    ) -> Result<Completeness, IntegrationFailure> {
        container::execute(ctx, host_data, checkpoint, ContainerRuntime::Podman).await
    }
}
