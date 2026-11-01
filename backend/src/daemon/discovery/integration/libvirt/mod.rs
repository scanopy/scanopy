//! libvirt discovery integration.
//!
//! Logs in to a KVM host over SSH with the credential's SSH login and reads every domain the
//! libvirt connection defines with `virsh`, running or not, each linked to the libvirt service of
//! the host it runs on.
//!
//! A domain's NICs come from its definition, so their MACs are the hypervisor's own assignment:
//! a stopped domain is recorded by them, and a running one joins the host the network scan finds
//! with the same MAC. Addresses come from libvirt's DHCP leases, the host's ARP table and, on a
//! connection with full access to a domain running the QEMU guest agent, the agent. What any
//! hypervisor's guests share (address selection, network identities, recording, reach) is
//! [`super::hypervisor`]; this module reads `virsh`.
//!
//! Layering: [`commands`] is what runs on the host, [`parse`] reads what it printed.

pub mod commands;
pub mod parse;

use std::collections::BTreeSet;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use super::hypervisor::guest::{self, GUEST_AGENT, GuestIdentity, GuestReading};
use super::hypervisor::host::{HypervisorHost, create_hypervisor_host};
use super::hypervisor::identities;
use super::hypervisor::qemu_agent;
use super::hypervisor::reach::{self, HypervisorBridge};
use super::hypervisor::sync::{self, GuestOutcome, GuestSync, sync_guests};
use super::ssh::client::{Session, SessionError};
use super::ssh::connect_trusted;
use super::{
    Checkpoint, CollectionShortfall, Completeness, DiscoveryIntegration, IntegrationContext,
    IntegrationFailure, ProbeContext, ProbeFailure, ProbeSuccess, merge_subnets,
};
use crate::daemon::discovery::integration::controller::ControllerIdentity;
use crate::daemon::discovery::service::ops::HostData;
use crate::daemon::discovery::service::warnings::UnreachedHypervisorGuest;
use crate::server::credentials::r#impl::mapping::{
    CredentialQueryPayload, CredentialQueryPayloadDiscriminants, LibvirtQueryCredential,
};
use crate::server::hosts::r#impl::attributes::{HostHostnameValue, HostOsValue};
use crate::server::hosts::r#impl::virtualization::{
    HostVirtualization, HostVirtualizationDiscriminants, LibvirtVirtualization,
};
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeSource, Attributed};
use crate::server::shared::position::renumber_positions;
use crate::server::shared::types::guest_state::GuestState;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::inference::placeable_subnet;

/// The source of what libvirt reports about the host and its domains in its own words: the host's
/// name and addresses, and the addresses its leases and ARP table hold for a domain.
pub const REPORTED: AttributeSource = AttributeSource::Probe(ClientProbe::Libvirt);

/// How long one `virsh` command in the probe may take.
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
/// How long the whole inventory may take: four `virsh` reads per domain, more with the agent.
const INVENTORY_TIMEOUT: Duration = Duration::from_secs(240);
/// The inventory's output for a large host: a few hundred bytes per domain.
const MAX_INVENTORY_BYTES: usize = 4 * 1024 * 1024;

/// The logged-in session carried from `probe` to `execute`, and whether only libvirt's read-only
/// socket would open.
struct LibvirtProbeHandle {
    session: tokio::sync::Mutex<Option<Session>>,
    read_only: bool,
}

pub struct LibvirtIntegration;

fn credential(c: &CredentialQueryPayload) -> Option<&LibvirtQueryCredential> {
    match c {
        CredentialQueryPayload::Libvirt(l) => Some(l),
        _ => None,
    }
}

#[async_trait]
impl DiscoveryIntegration for LibvirtIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::Libvirt
    }

    fn estimated_seconds(&self) -> u32 {
        10
    }

    fn timeout(&self) -> Duration {
        INVENTORY_TIMEOUT + Duration::from_secs(60)
    }

    /// Log in, then ask `virsh` for the connection. With full access first, since only that lets
    /// the guest agent be read; failing that, libvirt's read-only socket, which every account may
    /// open on a default install. A host with no `virsh` is not a libvirt host.
    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let cred = credential(ctx.credential)
            .ok_or_else(|| ProbeFailure::malformed("Expected a libvirt credential"))?;
        let session = connect_trusted(ctx, &cred.login).await?;

        let mut refusal = String::new();
        for read_only in [false, true] {
            let result = session
                .exec(
                    &commands::probe(&cred.connection_uri, read_only),
                    PROBE_TIMEOUT,
                    4096,
                )
                .await
                .map_err(probe_failure)?;
            match result.exit_code {
                Some(0) => {
                    tracing::info!(ip = %ctx.ip, read_only, "Opened the libvirt connection");
                    // libvirtd answered on its local socket, so its service owns no port. The
                    // login's port goes to the SSH service (`CredentialReach::ThroughLogin`).
                    return Ok(ProbeSuccess {
                        client_probe: Some(ClientProbe::Libvirt),
                        ports: vec![],
                        handle: Some(Box::new(LibvirtProbeHandle {
                            session: tokio::sync::Mutex::new(Some(session)),
                            read_only,
                        })),
                    });
                }
                Some(127) => {
                    session.close().await;
                    return Err(ProbeFailure::not_this_service(
                        "virsh is not installed on this host",
                    ));
                }
                _ => {
                    refusal = String::from_utf8_lossy(&result.stderr_tail)
                        .trim()
                        .to_string()
                }
            }
        }
        session.close().await;
        Err(ProbeFailure::rejected(format!(
            "virsh could not open {}: {refusal}",
            cred.connection_uri
        )))
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
        _checkpoint: &Checkpoint<'_>,
    ) -> Result<Completeness, IntegrationFailure> {
        let cred = credential(ctx.credential).ok_or_else(|| {
            IntegrationFailure::collection_failed("Expected a libvirt credential")
        })?;
        let handle = ctx
            .probe_handle
            .and_then(|h| h.downcast_ref::<LibvirtProbeHandle>())
            .ok_or_else(|| IntegrationFailure::collection_failed("libvirt ran without a probe"))?;
        let session =
            handle.session.lock().await.take().ok_or_else(|| {
                IntegrationFailure::collection_failed("libvirt session already used")
            })?;

        ctx.ops.report_progress(5).await.ok();
        let result = session
            .exec(
                &commands::inventory(&cred.connection_uri, handle.read_only),
                INVENTORY_TIMEOUT,
                MAX_INVENTORY_BYTES,
            )
            .await;
        session.close().await;
        let result = result.map_err(|e| IntegrationFailure::collection_failed(e.to_string()))?;
        if result.timed_out {
            return Err(IntegrationFailure::collection_timed_out(
                "virsh did not finish listing the host's domains in time",
            ));
        }
        let inventory = parse::inventory(&String::from_utf8_lossy(&result.stdout));
        if !inventory.listed {
            return Err(IntegrationFailure::collection_failed(format!(
                "virsh could not list the domains: {}",
                String::from_utf8_lossy(&result.stderr_tail).trim()
            )));
        }
        tracing::info!(
            ip = %ctx.ip,
            domains = inventory.domains.len(),
            read_only = handle.read_only,
            "Fetched libvirt domains"
        );
        ctx.ops.report_progress(20).await.ok();

        let subnets = merge_subnets(ctx.known_subnets, ctx.scanning_subnet, &host_data.subnets);
        let site_id = ctx.ops.site_id().await?;
        let hypervisor_name = inventory
            .hostname
            .clone()
            .unwrap_or_else(|| ctx.ip.to_string());
        if let Some(hostname) = &inventory.hostname {
            host_data.with_hostname(hostname.clone(), REPORTED);
        }

        let (host, ip_addresses) = hypervisor_host(&inventory, ctx.ip, &subnets, site_id);
        let created = create_hypervisor_host(
            ctx,
            &subnets,
            HypervisorHost {
                host,
                ip_addresses,
                client_probe: ClientProbe::Libvirt,
                source: REPORTED,
                guests: HostVirtualizationDiscriminants::Libvirt,
            },
        )
        .await?;
        let (hypervisor_host_id, owner) = match created {
            Some((host_id, service_id)) => (Some(host_id), Some(service_id)),
            None => (None, None),
        };
        ctx.ops.report_progress(30).await.ok();

        let bridges = parse::bridges(&inventory);
        let total = inventory.domains.len();
        let done = AtomicUsize::new(0);
        // Built before the first await, for the same `Send` reason as Proxmox's.
        let syncs: Vec<_> = inventory
            .domains
            .iter()
            .map(|domain| {
                sync_and_report(
                    ctx,
                    domain,
                    &inventory,
                    owner,
                    &bridges,
                    &hypervisor_name,
                    &subnets,
                    site_id,
                    &done,
                    total,
                )
            })
            .collect();
        let GuestSync {
            created,
            unidentifiable,
            short_owners,
            identities_listed,
        } = sync_guests(syncs).await;
        if ctx.cancel.is_cancelled() {
            return Err(IntegrationFailure::cancelled());
        }
        tracing::info!(created, "libvirt domain sync complete");
        if !unidentifiable.is_empty() {
            // No NIC and no address: nothing would keep it one host from scan to scan. A log line
            // rather than a scan warning, since the domain is defined that way on purpose.
            tracing::info!(
                domains = %unidentifiable.join(", "),
                "libvirt domains with no network interface were not recorded"
            );
        }

        // `virsh list --all` lists every domain the connection defines, so when every one was
        // recorded, a stored guest of this host that the run didn't report was undefined.
        let listed_in_full = inventory.complete && short_owners.is_empty();
        if let (Some(credential_id), true) = (ctx.credential_id, listed_in_full) {
            let integration = ctx.credential.into();
            if let Some(host_id) = hypervisor_host_id {
                ctx.ops
                    .record_guests_listed(credential_id, integration, host_id)
                    .await;
            }
            for host_id in identities_listed {
                ctx.ops
                    .record_guests_listed(credential_id, integration, host_id)
                    .await;
            }
        }

        if !inventory.complete {
            let read = inventory
                .domains
                .iter()
                .filter(|d| !d.state.is_empty())
                .count();
            return Ok(Completeness::Partial(CollectionShortfall {
                what: "libvirt domains",
                collected: read,
                expected: inventory.domains.len().max(read + 1),
            }));
        }
        Ok(Completeness::Complete)
    }
}

/// The probe's view of a failed `exec`: the session fell over before `virsh` answered.
fn probe_failure(error: SessionError) -> ProbeFailure {
    match error {
        SessionError::AuthenticationFailed(m) => ProbeFailure::rejected(m),
        SessionError::Malformed(m) => ProbeFailure::malformed(m),
        SessionError::Unreachable(m) => ProbeFailure::unreachable(m),
    }
}

/// Whether `ip` sits on a subnet the site already knows as a container bridge: a `docker0` or
/// `br-*` the runtime's own API reported. 172.17.0.1 is on every Docker host, so recorded on this
/// one it would tie it to all of them. An address on a subnet nobody knows yet is kept, as a
/// libvirt network's `virbr0` is on a first scan; interface names cannot tell a container bridge
/// from any other bridge (#663).
fn on_container_bridge(subnets: &[Subnet], ip: IpAddr) -> bool {
    placeable_subnet(subnets, ip).is_some_and(|s| s.is_container_bridge_subnet())
}

/// The host being scanned: named by its hostname, at the address it was reached at and every
/// address `ip addr` gives it but those on a known container bridge. The libvirt service is added
/// by the matcher.
fn hypervisor_host(
    inventory: &parse::Inventory,
    scanned: IpAddr,
    subnets: &[Subnet],
    site_id: Uuid,
) -> (crate::server::hosts::r#impl::base::Host, Vec<IPAddress>) {
    let host = ControllerIdentity {
        probe: ClientProbe::Libvirt,
        name: None,
        hostname: inventory.hostname.clone(),
        chassis_id: None,
        manufacturer: None,
        model: None,
        serial_number: None,
        firmware_revision: None,
    }
    .into_host(site_id);
    let addresses = parse::interface_addresses(&inventory.ip_addr);
    let iface_of = |ip: IpAddr| {
        addresses
            .iter()
            .find(|(_, a)| *a == ip)
            .map(|(iface, _)| iface.clone())
    };
    let mut seen = BTreeSet::new();
    let mut ip_addresses: Vec<IPAddress> = std::iter::once(scanned)
        .chain(
            addresses
                .iter()
                .map(|(_, ip)| *ip)
                .filter(|ip| !on_container_bridge(subnets, *ip)),
        )
        .filter(|ip| seen.insert(*ip))
        .enumerate()
        .filter_map(|(position, ip)| {
            IPAddress::discovered(
                site_id,
                subnets,
                ip,
                REPORTED,
                None,
                iface_of(ip),
                position as i32,
            )
        })
        .collect();
    renumber_positions(&mut ip_addresses);
    (host, ip_addresses)
}

/// What libvirt reports about one domain: its NICs from its definition, the addresses its leases,
/// the host's ARP table and its guest agent hold, and what the agent reads inside it.
fn read_domain(domain: &parse::Domain, inventory: &parse::Inventory) -> GuestReading {
    let nics = parse::nics(&domain.iflist, &inventory.network_bridges);
    let agent = domain
        .agent
        .as_deref()
        .map(|a| parse::addresses(a, GUEST_AGENT))
        .filter(|a| !a.is_empty());
    let mut runtime = agent.clone().unwrap_or_default();
    runtime.extend(parse::addresses(&domain.lease, REPORTED));
    runtime.extend(parse::addresses(&domain.arp, REPORTED));

    let mut reading = GuestReading {
        // Only what runs inside the guest names its other interfaces.
        reported: agent.clone().unwrap_or_default(),
        interfaces_read: agent.is_some(),
        ..Default::default()
    };
    if let Some(info) = domain.guestinfo.as_deref().filter(|_| agent.is_some()) {
        let (os, hostname) = parse::guest_info(info);
        reading.os = qemu_agent::os(&os).map(|os| Attributed::new(HostOsValue(os), GUEST_AGENT));
        reading.hostname = qemu_agent::hostname(hostname.as_deref())
            .map(|h| Attributed::new(HostHostnameValue(h), GUEST_AGENT));
    }
    reading.addresses = guest::select_addresses(&nics, runtime);
    reading.nics = nics;
    reading
}

/// Sync one domain unless the run was cancelled, then report progress over all `total`.
#[allow(clippy::too_many_arguments)]
async fn sync_and_report(
    ctx: &IntegrationContext<'_>,
    domain: &parse::Domain,
    inventory: &parse::Inventory,
    owner: Option<Uuid>,
    bridges: &[HypervisorBridge],
    hypervisor: &str,
    subnets: &[Subnet],
    site_id: Uuid,
    done: &AtomicUsize,
    total: usize,
) -> GuestOutcome {
    let outcome = if ctx.cancel.is_cancelled() {
        GuestOutcome::Failed {
            owner: hypervisor.to_string(),
        }
    } else {
        sync_domain(
            ctx, domain, inventory, owner, bridges, hypervisor, subnets, site_id,
        )
        .await
    };
    let synced = done.fetch_add(1, Ordering::Relaxed) + 1;
    ctx.ops
        .report_progress(30 + (60 * synced / total.max(1)) as u8)
        .await
        .ok();
    outcome
}

/// Read one domain and record it with its network identities. A running domain at addresses no
/// subnet this scan covers is recorded as unreached, with what would reach it.
#[allow(clippy::too_many_arguments)]
async fn sync_domain(
    ctx: &IntegrationContext<'_>,
    domain: &parse::Domain,
    inventory: &parse::Inventory,
    owner: Option<Uuid>,
    bridges: &[HypervisorBridge],
    hypervisor: &str,
    subnets: &[Subnet],
    site_id: Uuid,
) -> GuestOutcome {
    let label = domain.name.clone().unwrap_or_else(|| domain.uuid.clone());
    let reading = read_domain(domain, inventory);
    let state = GuestState::from_libvirt_state(&domain.state);
    let identity = GuestIdentity {
        virtualization: HostVirtualization::Libvirt(LibvirtVirtualization {
            vm_name: domain.name.clone(),
            vm_id: Some(domain.uuid.clone()),
            state,
        }),
        name: domain.name.clone(),
        reported_by: ClientProbe::Libvirt,
    };
    let Some(record) = guest::guest_host(identity, &reading, owner, subnets, site_id) else {
        return GuestOutcome::Unidentifiable(label);
    };
    let identities =
        identities::network_identities(&reading.nics, &reading.reported, subnets, REPORTED);
    match sync::record_guest(ctx, subnets, &label, record, &identities, site_id).await {
        Ok((host_id, all_identities_recorded)) => {
            if state == GuestState::Running
                && let Some(unreached) = reach::unreached(
                    &reading.nics,
                    &reading.addresses,
                    bridges,
                    ctx.scanned_subnets,
                    hypervisor,
                )
            {
                ctx.ops
                    .record_unreached_guest(UnreachedHypervisorGuest {
                        ip: unreached.address,
                        guest: label,
                        integration: CredentialQueryPayloadDiscriminants::Libvirt,
                        reason: unreached.reason,
                    })
                    .await;
            }
            GuestOutcome::Recorded {
                host_id,
                identities_listed: reading.interfaces_read && all_identities_recorded,
            }
        }
        Err(e) => {
            tracing::warn!(domain = %label, error = %e, "Failed to record libvirt domain");
            GuestOutcome::Failed {
                owner: hypervisor.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Against the inventory recorded from the libvirt lab (`tools/libvirt/`), as the `scanopy`
    //! user (libvirt group) and the `scanopy-ro` user (read-only socket) see it.
    use super::*;
    use crate::daemon::discovery::service::warnings::UnreachedGuestReason;
    use crate::server::shared::storage::traits::Storable;
    use crate::server::subnets::r#impl::base::{
        SubnetBase, SubnetCidr, SubnetCidrValue, SubnetTypeAttributed,
    };
    use crate::server::subnets::r#impl::types::SubnetType;

    const LAB: &str = include_str!("../../../../tests/libvirt/lab_inventory.txt");
    const LAB_READ_ONLY: &str =
        include_str!("../../../../tests/libvirt/lab_inventory_read_only.txt");

    fn domain<'a>(inventory: &'a parse::Inventory, name: &str) -> &'a parse::Domain {
        inventory
            .domains
            .iter()
            .find(|d| d.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("the lab defines {name}"))
    }

    fn subnet(cidr: &str, subnet_type: SubnetType) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: SubnetCidr::new(
                SubnetCidrValue(cidr.parse().unwrap()),
                AttributeSource::Manual,
            ),
            subnet_type: SubnetTypeAttributed::unattributed(subnet_type),
            ..Default::default()
        })
    }

    /// The lab LAN a daemon on it scans.
    fn lan() -> Vec<Subnet> {
        vec![subnet("192.168.4.0/22", SubnetType::Lan)]
    }

    /// Every domain is read, running or not, with the state `virsh` gives it.
    #[test]
    fn the_lab_inventory_reads_every_domain_with_its_state() {
        let inventory = parse::inventory(LAB);
        assert!(inventory.listed && inventory.complete);
        assert_eq!(inventory.domains.len(), 8);
        for d in &inventory.domains {
            let expected = match d.name.as_deref() {
                Some("kvm-paused") => GuestState::Paused,
                Some("kvm-stopped") => GuestState::Stopped,
                _ => GuestState::Running,
            };
            assert_eq!(GuestState::from_libvirt_state(&d.state), expected, "{d:?}");
        }
    }

    /// A NIC on a libvirt network sits on that network's bridge; one on a host bridge, on it.
    #[test]
    fn a_nic_on_a_libvirt_network_takes_the_networks_bridge() {
        let inventory = parse::inventory(LAB);
        let nics = parse::nics(
            &domain(&inventory, "kvm-agent").iflist,
            &inventory.network_bridges,
        );
        let placed: Vec<_> = nics
            .iter()
            .map(|n| (n.mac.as_deref(), n.bridge.as_deref()))
            .collect();
        assert_eq!(
            placed,
            vec![
                (Some("52:54:00:7a:00:05"), Some("br0")),
                (Some("52:54:00:7a:00:06"), Some("virbr0")),
            ]
        );
        assert_ne!(nics[0].key, nics[1].key);
    }

    /// The agent's addresses on the domain's own NICs are its addresses, attributed to the agent.
    /// Loopback and link-local are not, and a NIC's later rows carry its MAC.
    #[test]
    fn a_domain_with_an_agent_holds_its_nics_addresses() {
        let inventory = parse::inventory(LAB);
        let reading = read_domain(domain(&inventory, "kvm-agent"), &inventory);
        let lan: IpAddr = "192.168.7.175".parse().unwrap();
        let on_lan = reading
            .addresses
            .iter()
            .find(|a| a.ip == lan)
            .expect("the agent reports the LAN address");
        assert_eq!(on_lan.source, GUEST_AGENT);
        assert_eq!(on_lan.mac.as_deref(), Some("52:54:00:7a:00:05"));
        assert!(
            reading
                .addresses
                .iter()
                .all(|a| guest::reachable(a.ip) && a.mac.is_some())
        );
        let global_v6 = reading
            .addresses
            .iter()
            .find(|a| a.ip.to_string().starts_with("2001:db8:"))
            .expect("eth0's IPv6 row");
        assert_eq!(global_v6.mac.as_deref(), Some("52:54:00:7a:00:05"));
        assert!(reading.interfaces_read);
        assert!(reading.os.is_some() && reading.hostname.is_some());
    }

    /// With no agent, no lease and no ARP entry, a bridged domain is known by its MAC alone, as
    /// the reporter's VMs are.
    #[test]
    fn a_bridged_domain_without_an_agent_is_known_by_its_mac() {
        let inventory = parse::inventory(LAB);
        for name in ["kvm-noagent", "kvm-nochannel", "kvm-stopped"] {
            let reading = read_domain(domain(&inventory, name), &inventory);
            assert!(reading.addresses.is_empty(), "{name}");
            assert_eq!(reading.nics.len(), 1, "{name}");
            assert!(reading.nics[0].mac.is_some(), "{name}");
            assert!(!reading.interfaces_read && reading.os.is_none(), "{name}");
        }
    }

    /// The macvlan link inside `kvm-docker` holds a LAN address with its own MAC: a network
    /// identity. Its own NIC and Docker's bridges are not.
    #[test]
    fn the_macvlan_link_in_a_guest_is_its_network_identity() {
        let inventory = parse::inventory(LAB);
        let reading = read_domain(domain(&inventory, "kvm-docker"), &inventory);
        let mut subnets = lan();
        subnets.push(subnet("172.17.0.0/16", SubnetType::DockerBridge));
        let found =
            identities::network_identities(&reading.nics, &reading.reported, &subnets, REPORTED);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].interface.as_deref(), Some("mv-lab"));
        assert!(
            found[0]
                .addresses
                .contains(&"192.168.7.178".parse().unwrap())
        );
        assert!(
            reading
                .addresses
                .iter()
                .all(|a| a.ip != "192.168.7.178".parse::<IpAddr>().unwrap())
        );
    }

    /// `br0` holds the host's NIC and address, `virbr0` an address but no NIC, `br-iso` neither.
    #[test]
    fn the_hosts_bridges_read_their_uplink_and_address() {
        let bridges = parse::bridges(&parse::inventory(LAB));
        let of = |name: &str| {
            bridges
                .iter()
                .find(|b| b.iface == name)
                .map(|b| (b.uplinked, b.addressed))
                .unwrap_or_else(|| panic!("{name} is read"))
        };
        assert_eq!(of("br0"), (true, true));
        assert_eq!(of("virbr0"), (false, true));
        assert_eq!(of("br-iso"), (false, false));
    }

    /// The KVM host keeps its LAN and `virbr0` addresses, not its own `docker0`, whose 172.17.0.1
    /// every Docker host on the site also has.
    #[test]
    fn the_kvm_host_records_no_container_bridge_address() {
        // A first scan: nothing knows `virbr0`'s 192.168.122.0/24 yet, and the site holds the
        // Docker bridge another host's runtime reported.
        let mut subnets = lan();
        subnets.push(subnet("172.17.0.0/16", SubnetType::DockerBridge));
        let (_, addresses) = hypervisor_host(
            &parse::inventory(LAB),
            "192.168.7.172".parse().unwrap(),
            &subnets,
            Uuid::new_v4(),
        );
        let ips: Vec<IpAddr> = addresses.iter().map(|a| a.base.ip_address).collect();
        assert!(ips.contains(&"192.168.7.172".parse().unwrap()), "{ips:?}");
        assert!(ips.contains(&"192.168.122.1".parse().unwrap()), "{ips:?}");
        assert!(!ips.contains(&"172.17.0.1".parse().unwrap()), "{ips:?}");
    }

    /// A domain on libvirt's NAT network, at a lease address outside the scan, is reachable only
    /// from the KVM host, through `virbr0`.
    #[test]
    fn a_domain_on_the_nat_network_is_behind_the_hypervisor() {
        let inventory = parse::inventory(LAB);
        let reading = read_domain(domain(&inventory, "kvm-nat"), &inventory);
        let unreached = reach::unreached(
            &reading.nics,
            &reading.addresses,
            &parse::bridges(&inventory),
            &lan(),
            "scanopy-libvirt-host",
        )
        .expect("its lease address is outside the scan");
        assert_eq!(
            unreached.reason,
            UnreachedGuestReason::BehindHypervisor {
                hypervisor: "scanopy-libvirt-host".to_string(),
                bridge: "virbr0".to_string(),
            }
        );
    }

    /// A read-only connection lists the same domains in the same states, and asks the agent for
    /// nothing, so no domain gains an OS or identities from it.
    #[test]
    fn a_read_only_inventory_reads_the_same_domains_without_the_agent() {
        let (full, read_only) = (parse::inventory(LAB), parse::inventory(LAB_READ_ONLY));
        assert!(read_only.listed && read_only.complete);
        let states = |i: &parse::Inventory| {
            let mut s: Vec<_> = i
                .domains
                .iter()
                .map(|d| (d.uuid.clone(), d.state.clone()))
                .collect();
            s.sort();
            s
        };
        assert_eq!(states(&full), states(&read_only));
        for d in &read_only.domains {
            assert!(d.agent.is_none() && d.guestinfo.is_none(), "{d:?}");
            let reading = read_domain(d, &read_only);
            assert!(!reading.interfaces_read && reading.os.is_none(), "{d:?}");
        }
    }
}

#[cfg(test)]
mod lab_tests {
    //! Against the libvirt lab (`tools/libvirt/`), through the same client, script and parser a
    //! scan uses. They submit nothing. `make libvirt-lab-capture` runs `capture`.
    use super::*;
    use crate::daemon::discovery::integration::ssh::client;
    use crate::server::credentials::r#impl::mapping::{ResolvableSecret, SshAuth};

    const HOST: &str = "192.168.7.172";

    async fn inventory_as(user: &str, read_only: bool) -> String {
        let key_path = format!("{}/.ssh/libvirt-lab", std::env::var("HOME").unwrap());
        let key = std::fs::read_to_string(&key_path).expect("the lab key (tools/libvirt)");
        let auth = SshAuth::PrivateKey {
            private_key: ResolvableSecret::Value { value: key },
            passphrase: None,
        };
        let session = client::handshake(HOST.parse().unwrap(), 22)
            .await
            .expect("the lab host answers SSH")
            .authenticate(user, &auth)
            .await
            .expect("the lab accepts the key");
        let result = session
            .exec(
                &commands::inventory("qemu:///system", read_only),
                INVENTORY_TIMEOUT,
                MAX_INVENTORY_BYTES,
            )
            .await
            .expect("the inventory runs");
        session.close().await;
        String::from_utf8_lossy(&result.stdout).into_owned()
    }

    /// Records the inventory as the lab's `scanopy` (libvirt group) and `scanopy-ro` (read-only
    /// socket) users see it, into `src/tests/libvirt/` for the parse tests. Global IPv6 prefixes
    /// become the documentation prefix `2001:db8::/32`, so the lab's ISP prefix stays out of the
    /// repository.
    #[tokio::test]
    #[ignore = "reads the live libvirt lab"]
    async fn capture() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests/libvirt");
        std::fs::create_dir_all(&dir).unwrap();
        // Candidate tokens only; whether one is an address, and a global one, is the parse's call.
        let hex_run = regex::Regex::new(r"[0-9A-Fa-f:]+").unwrap();
        let documentation_prefix = |token: &regex::Captures| {
            let token = &token[0];
            match token.parse::<std::net::Ipv6Addr>() {
                Ok(ip) if ip.segments()[0] & 0xe000 == 0x2000 => {
                    let mut segments = ip.segments();
                    segments[0] = 0x2001;
                    segments[1] = 0x0db8;
                    std::net::Ipv6Addr::from(segments).to_string()
                }
                _ => token.to_string(),
            }
        };
        for (user, read_only, file) in [
            ("scanopy", false, "lab_inventory.txt"),
            ("scanopy-ro", true, "lab_inventory_read_only.txt"),
        ] {
            let output = inventory_as(user, read_only).await;
            let output = hex_run
                .replace_all(&output, documentation_prefix)
                .into_owned();
            std::fs::write(dir.join(file), &output).unwrap();
            let parsed = parse::inventory(&output);
            println!(
                "{file}: {} domains, listed {}, complete {}",
                parsed.domains.len(),
                parsed.listed,
                parsed.complete
            );
            assert!(parsed.complete, "{user}'s inventory reached its end");
        }
    }

    /// Prints what a scan would record from the lab, domain by domain.
    #[tokio::test]
    #[ignore = "reads the live libvirt lab"]
    async fn reads_the_lab_host() {
        let inventory = parse::inventory(&inventory_as("scanopy", false).await);
        println!("bridges {:?}", parse::bridges(&inventory));
        for domain in &inventory.domains {
            let reading = read_domain(domain, &inventory);
            println!(
                "{} {} nics {:?} addresses {:?} os {:?} hostname {:?}",
                domain.name.as_deref().unwrap_or("-"),
                domain.state,
                reading.nics,
                reading
                    .addresses
                    .iter()
                    .map(|a| (a.ip, a.source))
                    .collect::<Vec<_>>(),
                reading.os.as_ref().map(|o| o.value().0.to_string()),
                reading.hostname.as_ref().map(|h| h.value().0.clone()),
            );
        }
        assert!(!inventory.domains.is_empty(), "the lab defines domains");
    }
}
