//! Docker discovery integration.
//!
//! Thin wrapper over the shared container machinery in
//! [`super::container`], selecting [`ContainerRuntime::Docker`]. Two transports:
//! - proxy — HTTP(S) proxy URL (remote or local), `DockerProxy` credential
//! - socket — local Unix socket (`/var/run/docker.sock`), `DockerSocket` credential
//!
//! Also holds the Docker-specific localhost-proxy resolution helpers used by the
//! pre-Docker-phase subnet discovery (AppConfig fallback).

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

use anyhow::{Error, Result};
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
    InterfaceViewScope, ProbeContext, ProbeFailure, ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;
use crate::daemon::shared::config::ConfigStore;

pub struct DockerIntegration;

#[async_trait]
impl DiscoveryIntegration for DockerIntegration {
    /// Container runtimes report networks and containers, never the host's ports.
    fn interface_view_scope(&self) -> InterfaceViewScope {
        InterfaceViewScope::NoInterfaces
    }

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

    fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
        match credential {
            CredentialQueryPayload::DockerProxy(docker) => vec![PortType::new_tcp(docker.port)],
            _ => vec![],
        }
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
    /// Container runtimes report networks and containers, never the host's ports.
    fn interface_view_scope(&self) -> InterfaceViewScope {
        InterfaceViewScope::NoInterfaces
    }

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

    // No probe_gate_ports — Unix socket, no TCP port needed.

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        // Explicit socket_path from the credential repoints the socket; None → bollard Docker
        // defaults (DOCKER_HOST / /var/run/docker.sock).
        let socket_path = match ctx.credential {
            CredentialQueryPayload::DockerSocket(c) => c.socket_path.clone(),
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

/// Resolve the Docker proxy configuration for localhost from credential mappings.
///
/// Checks credential_mappings for DockerProxy targeting localhost only.
/// Remote Docker credentials are handled in deep_scan_host() during network scanning.
/// Falls back to AppConfig if no credential found.
pub async fn resolve_docker_proxy(
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
    config_store: &ConfigStore,
) -> Result<(
    Result<Option<String>, Error>,
    Result<Option<(String, String, String)>, Error>,
    Vec<tempfile::NamedTempFile>,
    Option<Uuid>,
    Option<u16>,
)> {
    for mapping in credential_mappings {
        let docker_match = mapping.ip_overrides.iter().find(|o| {
            o.is_localhost() && matches!(o.credential, CredentialQueryPayload::DockerProxy(_))
        });

        let (docker_cred, cred_id, override_ip) = if let Some(override_entry) = docker_match {
            let cred = match &override_entry.credential {
                CredentialQueryPayload::DockerProxy(d) => d,
                _ => unreachable!(),
            };
            let id = override_entry.credential_id;
            (
                Some(cred),
                if id != Uuid::nil() { Some(id) } else { None },
                Some(override_entry.ip),
            )
        } else if let Some(CredentialQueryPayload::DockerProxy(d)) =
            mapping.default_credential.as_ref()
        {
            (Some(d), None, None)
        } else {
            (None, None, None)
        };

        if let Some(docker_cred) = docker_cred {
            let label = "Docker proxy connection";

            // Build proxy URL
            let proxy_path = docker_cred
                .path
                .as_deref()
                .unwrap_or("")
                .trim_start_matches('/');
            let has_ssl = docker_cred.ssl_cert.is_some()
                && docker_cred.ssl_key.is_some()
                && docker_cred.ssl_chain.is_some();
            let partial_ssl = !has_ssl
                && (docker_cred.ssl_cert.is_some()
                    || docker_cred.ssl_key.is_some()
                    || docker_cred.ssl_chain.is_some());
            if partial_ssl {
                tracing::warn!(
                    "Partial Docker proxy SSL config: all of ssl_cert, ssl_key, and ssl_chain \
                     must be provided for TLS. Falling back to HTTP."
                );
            }
            let scheme = if has_ssl { "https" } else { "http" };
            let host = match override_ip {
                Some(IpAddr::V6(v6)) => format!("[{}]", v6),
                Some(ip) => ip.to_string(),
                None => "127.0.0.1".to_string(),
            };
            let proxy_url = if proxy_path.is_empty() {
                format!("{}://{}:{}", scheme, host, docker_cred.port)
            } else {
                format!("{}://{}:{}/{}", scheme, host, docker_cred.port, proxy_path)
            };

            // Resolve SSL to filesystem paths (inline values get written to temp files)
            let mut temp_handles = Vec::new();
            let ssl_info = if let (Some(cert_rv), Some(key_rv), Some(chain_rv)) = (
                &docker_cred.ssl_cert,
                &docker_cred.ssl_key,
                &docker_cred.ssl_chain,
            ) {
                let (cert_path, cert_handle) = cert_rv.resolve_to_path("ssl_cert", label)?;
                let (key_path, key_handle) = key_rv.resolve_to_path("ssl_key", label)?;
                let (chain_path, chain_handle) = chain_rv.resolve_to_path("ssl_chain", label)?;
                temp_handles.extend(cert_handle);
                temp_handles.extend(key_handle);
                temp_handles.extend(chain_handle);
                Ok(Some((
                    cert_path.to_string_lossy().into_owned(),
                    key_path.to_string_lossy().into_owned(),
                    chain_path.to_string_lossy().into_owned(),
                )))
            } else {
                Ok(None)
            };

            tracing::info!(
                proxy_url = %proxy_url,
                has_ssl = has_ssl,
                credential_id = ?cred_id,
                "Resolved Docker proxy from credential"
            );

            return Ok((
                Ok(Some(proxy_url)),
                ssl_info,
                temp_handles,
                cred_id,
                Some(docker_cred.port),
            ));
        }
    }

    // Fall back to AppConfig with deprecation warning (no credential_id)
    tracing::debug!("No Docker proxy credential in mappings, falling back to AppConfig");
    let docker_proxy = config_store.get_docker_proxy().await;
    let docker_proxy_ssl_info = config_store.get_docker_proxy_ssl_info().await;

    Ok((docker_proxy, docker_proxy_ssl_info, Vec::new(), None, None))
}

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
