//! Host rescans: one-shot scans of known hosts' addresses, run by the daemon that last found each
//! host. Hosts that share a daemon share one rescan, which targets all of their addresses.

use std::net::IpAddr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Json;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::permissions::{Authorized, Can, Write};
use crate::server::config::AppState;
use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use crate::server::daemons::r#impl::base::Daemon;
use crate::server::daemons::r#impl::version::{
    last_without_routed_rescan, last_without_targeted_rescan, resolves_routed_rescan,
    supports_targeted_rescan,
};
use crate::server::discovery::r#impl::base::{Discovery, DiscoveryBase};
use crate::server::discovery::r#impl::scan_settings::{RescanSettings, ScanSettings};
use crate::server::discovery::r#impl::types::{DiscoveryType, RunType};
use crate::server::hosts::r#impl::base::Host;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::ports::r#impl::base::PortType;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiJson, ApiResponse, ApiResult,
};
use crate::server::shared::validation::validate_read_access;

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(rescan_host))
        .routes(routes!(bulk_rescan_hosts))
}

/// Rescan host
///
/// Starts a one-shot scan of this host's addresses and nothing else, answering
/// "is this host still there, and is its data current?" without sweeping the
/// whole subnet.
///
/// The scan runs on the daemon that last discovered this host, which is evidence
/// it can reach the address. Where the daemon has an interface with a MAC on the
/// address's subnet it ARPs the target, which sees a live host even when every
/// port is firewalled. Every other address, on a MAC-less interface (a
/// point-to-point tunnel) or on a subnet the daemon reaches through a router, is
/// probed by TCP. An address that doesn't answer comes back as not found. A
/// loopback address is reached locally and is excluded from the target set.
///
/// Daemons at or below `last_without_routed_rescan` only rescan addresses on
/// subnets they have an interface on, and the request is refused when the host
/// has none.
///
/// Returns the session, which streams progress over `/api/v1/live-updates/stream`
/// like any other scan. A `Queued` phase means the daemon is busy; it will start
/// when the running scan finishes.
#[utoipa::path(
    post,
    path = "/{id}/rescan",
    tag = Host::ENTITY_NAME_PLURAL,
    params(("id" = uuid::Uuid, Path, description = "Host ID")),
    responses(
        (status = 200, description = "Rescan session started", body = ApiResponse<DiscoveryUpdatePayload>),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
        (status = 400, description = "Host cannot be rescanned (never scanned, daemon gone, or daemon too old)", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn rescan_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Discovery, Write>>,
    Path(host_id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<DiscoveryUpdatePayload>>> {
    let host = state
        .services
        .host_service
        .get_by_id(&host_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(host_id))?;

    let mut sessions = rescan(&state, auth, vec![host]).await?;
    let session = sessions
        .pop()
        .ok_or_else(|| ApiError::internal_error("A rescan of one host started no session"))?;
    Ok(Json(ApiResponse::success(session)))
}

/// Rescan hosts
///
/// Rescans several hosts at once. Hosts last found by the same daemon share one
/// rescan, which scans all of their addresses in one session; hosts found by
/// different daemons get one rescan per daemon. Every host is checked as in
/// `POST /{id}/rescan` before any rescan starts, and the request is refused if
/// any of them can't be rescanned.
///
/// Returns one session per daemon.
#[utoipa::path(
    post,
    path = "/bulk-rescan",
    tag = Host::ENTITY_NAME_PLURAL,
    request_body(content = Vec<Uuid>, description = "IDs of the hosts to rescan"),
    responses(
        (status = 200, description = "Rescan sessions started, one per daemon", body = ApiResponse<Vec<DiscoveryUpdatePayload>>),
        (status = 404, description = "A host was not found", body = ApiErrorResponse),
        (status = 400, description = "A host cannot be rescanned; the message names it", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_rescan_hosts(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Discovery, Write>>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<Vec<DiscoveryUpdatePayload>>>> {
    if ids.is_empty() {
        return Err(ApiError::bulk_empty());
    }
    let hosts = state
        .services
        .host_service
        .get_all(StorableFilter::<Host>::new_from_entity_ids(&ids))
        .await?;
    if let Some(missing) = ids.iter().find(|id| !hosts.iter().any(|h| h.id == **id)) {
        return Err(ApiError::entity_not_found::<Host>(missing));
    }

    let sessions = rescan(&state, auth, hosts).await?;
    Ok(Json(ApiResponse::success(sessions)))
}

/// Checks every host, then starts one rescan per daemon. Nothing starts unless every host
/// can be rescanned.
async fn rescan(
    state: &Arc<AppState>,
    auth: Authorized<Can<Discovery, Write>>,
    hosts: Vec<Host>,
) -> Result<Vec<DiscoveryUpdatePayload>, ApiError> {
    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let several = hosts.len() > 1;

    let mut targets = Vec::with_capacity(hosts.len());
    for host in hosts {
        validate_read_access(Some(host.base.site_id), None, &site_ids, organization_id)?;
        host.require_confirmed()
            .map_err(|e| ApiError::bad_request(&e.to_string()))?;
        let (name, target) = rescan_target(state, &host).await?;
        match target {
            Ok((daemon, ips)) => targets.push((
                RescanHost {
                    id: host.id,
                    name,
                    ips,
                },
                daemon,
            )),
            // Several hosts: name the one refused, since the message says "this host".
            Err(reason) if several => {
                return Err(ApiError::bad_request(&format!(
                    "Can't rescan \"{name}\": {reason}"
                )));
            }
            Err(reason) => return Err(ApiError::bad_request(&reason)),
        }
    }

    // The ports already recorded on these hosts. `get_for_hosts` applies the live
    // filter; `get_for_host` does not, and would resurrect historically-closed
    // ports into the scan list.
    let host_ids: Vec<Uuid> = targets.iter().map(|(host, _)| host.id).collect();
    let mut ports_by_host = state
        .services
        .port_service
        .get_for_hosts(&host_ids, None)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    let mut planned: Vec<(Daemon, Vec<RescanHost>)> = Vec::new();
    for (host, daemon) in targets {
        match planned.iter_mut().find(|(d, _)| d.id == daemon.id) {
            Some((_, group)) => group.push(host),
            None => planned.push((daemon, vec![host])),
        }
    }

    let entity = auth.into_entity();
    let mut sessions = Vec::with_capacity(planned.len());
    for (daemon, group) in planned {
        let mut ports: Vec<PortType> = Vec::new();
        for host in &group {
            for port in ports_by_host.remove(&host.id).unwrap_or_default() {
                if !ports.contains(&port.base.port_type) {
                    ports.push(port.base.port_type);
                }
            }
        }
        let discovery = rescan_discovery(state, &daemon, &group, ports).await;
        sessions.push(start_rescan(state, daemon, discovery, entity.clone()).await?);
    }
    Ok(sessions)
}

/// One daemon's rescan of the hosts it last found, with their scannable addresses.
async fn rescan_discovery(
    state: &Arc<AppState>,
    daemon: &Daemon,
    group: &[RescanHost],
    ports: Vec<PortType>,
) -> Discovery {
    // Inherit the daemon's configured scan settings so a deliberately lowered
    // scan rate — or raw-socket probing the operator turned on — carries over.
    // The conversion drops the full-scan fields, which a rescan must not have.
    let settings = RescanSettings::from(
        &daemon_scan_settings(state, daemon.id)
            .await
            .unwrap_or_default(),
    );
    let ips: Vec<IpAddr> = group
        .iter()
        .flat_map(|host| host.ips.iter().copied())
        .collect();
    let names: Vec<String> = group.iter().map(|host| host.name.clone()).collect();

    Discovery::new(DiscoveryBase {
        run_type: RunType::AdHoc { last_run: None },
        discovery_type: DiscoveryType::Rescan {
            host_id: daemon.base.host_id,
            target_host_ids: group.iter().map(|host| host.id).collect(),
            ips: ips.clone(),
            ports,
            settings,
        },
        // The host names and addresses are only known here, and the historical
        // record inherits this name.
        name: rescan_name(&names, &ips),
        daemon_id: daemon.id,
        site_id: daemon.base.site_id,
        tags: Vec::new(),
    })
}

/// "Rescan of web-01 (10.0.0.5)" for one host, so the record carries the address the scan
/// aimed at; "Rescan of web-01, web-02 and 3 more" for several, where the addresses would not fit.
fn rescan_name(names: &[String], ips: &[IpAddr]) -> String {
    match names {
        [one] => format!(
            "Rescan of {} ({})",
            one,
            ips.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        [a, b] => format!("Rescan of {a} and {b}"),
        [a, b, c] => format!("Rescan of {a}, {b} and {c}"),
        [a, b, rest @ ..] => format!("Rescan of {a}, {b} and {} more", rest.len()),
        [] => "Rescan".to_string(),
    }
}

/// Creates the rescan, wakes its daemon if it is on standby, and starts the session.
async fn start_rescan(
    state: &Arc<AppState>,
    mut daemon: Daemon,
    discovery: Discovery,
    entity: AuthenticatedEntity,
) -> Result<DiscoveryUpdatePayload, ApiError> {
    let discovery = state
        .services
        .discovery_service
        .create_discovery(discovery, AuthenticatedEntity::System)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // start_session itself doesn't clear standby, and a standby daemon never picks up the work.
    state
        .services
        .daemon_service
        .wake_from_standby(&mut daemon)
        .await?;

    state
        .services
        .discovery_service
        .start_session(discovery, entity)
        .await
}

/// A host going into a rescan: what to call it in the rescan's name, and its addresses the
/// daemon can reach.
struct RescanHost {
    id: Uuid,
    name: String,
    ips: Vec<IpAddr>,
}

/// What the host list titles this host, and either the daemon that will rescan it with the
/// addresses it can reach, or why the host can't be rescanned, worded for the one host.
async fn rescan_target(
    state: &Arc<AppState>,
    host: &Host,
) -> Result<(String, Result<(Daemon, Vec<IpAddr>), String>), ApiError> {
    let ip_addresses = state
        .services
        .ip_address_service
        .get_for_host(&host.id)
        .await?;
    // The name ladder the host list titles rows with: a host known only by its address is
    // named by it, where `host.base.name` is empty.
    let name = host
        .display_name(&ip_addresses)
        .unwrap_or_else(|| "unnamed host".to_string());
    if ip_addresses.is_empty() {
        return Ok((
            name,
            Err("This host has no IP addresses to scan.".to_string()),
        ));
    }

    let daemon = match resolve_rescan_daemon(state, host).await? {
        Ok(daemon) => daemon,
        Err(reason) => return Ok((name, Err(reason))),
    };

    // A daemon that only rescans its interfaced subnets fails the whole rescan when no target
    // is on one, so it is sent just those.
    let routes = resolves_routed_rescan(daemon.base.version.as_ref());
    let interfaced = if routes {
        None
    } else {
        Some(
            state
                .services
                .daemon_service
                .get_interfaced_subnet_ids(&daemon.id)
                .await,
        )
    };
    let targets = scannable_addresses(&ip_addresses, interfaced.as_deref());

    if targets.is_empty() {
        let reason = if routes {
            "This host has only loopback addresses, which aren't scanned.".to_string()
        } else {
            format!(
                "Daemon \"{}\" has no interface on a subnet holding any of this host's \
                 addresses. Upgrade it to a version newer than {} to rescan addresses it \
                 reaches through a router.",
                daemon.base.name,
                last_without_routed_rescan()
            )
        };
        return Ok((name, Err(reason)));
    }

    Ok((name, Ok((daemon, targets))))
}

/// The addresses a rescan targets: every one but loopback, which is reached locally and dropped
/// from every scan. Keeping it out also keeps 127.0.0.1 out of the frozen rescan name.
/// `interfaced`, for a daemon that can't scan through a router, narrows them to the addresses
/// on those subnets.
fn scannable_addresses(ip_addresses: &[IPAddress], interfaced: Option<&[Uuid]>) -> Vec<IpAddr> {
    ip_addresses
        .iter()
        .filter(|ip| !ip.base.ip_address.is_loopback())
        .filter(|ip| interfaced.is_none_or(|subnets| subnets.contains(&ip.base.subnet_id)))
        .map(|ip| ip.base.ip_address)
        .collect()
}

/// The scan settings on this daemon's own discovery configuration, if it has one.
async fn daemon_scan_settings(state: &Arc<AppState>, daemon_id: Uuid) -> Option<ScanSettings> {
    let filter = StorableFilter::new_from_uuid_column("daemon_id", &daemon_id).live_configs();
    let discoveries = state
        .services
        .discovery_service
        .get_all(filter)
        .await
        .ok()?;
    discoveries
        .into_iter()
        .find_map(|d| match d.base.discovery_type {
            DiscoveryType::Unified { scan_settings, .. } => Some(scan_settings),
            _ => None,
        })
}

/// The daemon that last discovered this host, validated as usable for a rescan. The inner `Err`
/// is why it isn't.
///
/// Provenance beats inference here: the daemon that produced this host's data
/// demonstrably reached the address, which is a stronger signal than any
/// subnet-membership calculation over the interfaced-subnet junction.
async fn resolve_rescan_daemon(
    state: &Arc<AppState>,
    host: &Host,
) -> Result<Result<Daemon, String>, ApiError> {
    let Some(discovery_id) = host.last_discovery_id else {
        return Ok(Err(
            "This host hasn't been scanned yet, so there is no daemon known to \
                       reach it. Run a discovery first."
                .to_string(),
        ));
    };

    // The FK is ON DELETE SET NULL, so a pruned historical row lands above rather
    // than here; this covers a row that vanished between reads.
    let Some(last_scan) = state
        .services
        .discovery_service
        .get_by_id(&discovery_id)
        .await?
    else {
        return Ok(Err(
            "The scan that last found this host is no longer on record, so its \
                       daemon can't be identified. Run a discovery first."
                .to_string(),
        ));
    };

    let Some(daemon) = state
        .services
        .daemon_service
        .get_by_id(&last_scan.base.daemon_id)
        .await?
    else {
        return Ok(Err(
            "The daemon that last scanned this host no longer exists. Run a \
                       discovery from another daemon to refresh it."
                .to_string(),
        ));
    };

    if daemon.base.site_id != host.base.site_id {
        return Ok(Err(format!(
            "Daemon \"{}\" has moved to a different site and can no longer reach this host.",
            daemon.base.name
        )));
    }

    if !supports_targeted_rescan(daemon.base.version.as_ref()) {
        return Ok(Err(match daemon.base.version {
            None => format!(
                "Daemon \"{}\" has not connected to the server yet, so its version is unknown. \
                 Wait for it to check in, then try again.",
                daemon.base.name
            ),
            Some(_) => format!(
                "Daemon \"{}\" does not support rescans. Upgrade it to a version \
                 newer than {}.",
                daemon.base.name,
                last_without_targeted_rescan()
            ),
        }));
    }

    Ok(Ok(daemon))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::ip_addresses::r#impl::base::IPAddressBase;

    fn address(ip: &str, subnet_id: Uuid) -> IPAddress {
        IPAddress::new(IPAddressBase {
            subnet_id,
            ip_address: ip.parse().unwrap(),
            ..Default::default()
        })
    }

    /// A host on a VLAN the daemon reaches through a router: none of its subnets is interfaced,
    /// and every address is still a target.
    #[test]
    fn a_host_with_only_routed_addresses_is_rescanned() {
        let vlan = Uuid::new_v4();
        let ips = [address("10.20.0.5", vlan), address("10.20.0.6", vlan)];

        assert_eq!(
            scannable_addresses(&ips, None),
            vec![ips[0].base.ip_address, ips[1].base.ip_address]
        );
    }

    /// A daemon that can't scan through a router is sent only the addresses on its interfaced
    /// subnets, so a routed-only host leaves it nothing and the request is refused.
    #[test]
    fn an_old_daemon_keeps_only_interfaced_addresses() {
        let lan = Uuid::new_v4();
        let vlan = Uuid::new_v4();
        let ips = [address("192.168.1.5", lan), address("10.20.0.5", vlan)];

        assert_eq!(
            scannable_addresses(&ips, Some(&[lan])),
            vec![ips[0].base.ip_address]
        );
        assert!(scannable_addresses(&ips[1..], Some(&[lan])).is_empty());
    }

    #[test]
    fn loopback_is_never_a_target() {
        let subnet = Uuid::new_v4();
        let ips = [address("127.0.0.1", subnet), address("10.20.0.5", subnet)];

        assert_eq!(
            scannable_addresses(&ips, None),
            vec![ips[1].base.ip_address]
        );
        assert!(scannable_addresses(&ips[..1], Some(&[subnet])).is_empty());
    }
}
