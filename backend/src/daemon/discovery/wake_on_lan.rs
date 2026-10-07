//! The Wake-on-LAN step: wake hosts that sleep between scans before the sweep looks for them.
//!
//! Runs once per session, between the daemon-host phase and the network phase, because the ARP/ICMP
//! sweep only finds hosts that are already awake. The server sends one `IpOverride` per address of
//! each assigned host, carrying the MAC it holds for that address. The packet goes to the directed
//! broadcast of the address's subnet unless the credential names another destination, and then the
//! daemon pauses for the credential's wait so the hosts can boot.
//!
//! This step does not decide whether a host woke. The sweep does, with the same liveness checks it
//! runs on every host: every target is recorded here as not woken, and the Wake-on-LAN integration
//! (`integration::wake_on_lan`) marks the ones the scan then finds.

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use mac_address::MacAddress;
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::discovery::service::warnings::{
    AttemptOutcome, CredentialIssue, CredentialIssueReason,
};
use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
    WakeOnLanQueryCredential,
};
use crate::server::credentials::r#impl::run_results::WakeOnLanResult;
use crate::server::subnets::r#impl::base::Subnet;

/// Packets per target. UDP broadcast is unacknowledged and a switch can drop one on a port that is
/// renegotiating link speed as the NIC drops into its low-power state.
const PACKETS_PER_TARGET: u32 = 3;
const PACKET_INTERVAL: Duration = Duration::from_secs(1);
/// How often the boot wait re-reports progress, so the server's stall detector does not end the
/// session while hosts boot.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// One address to wake, with what to wake it with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeTarget {
    pub ip: IpAddr,
    pub mac: Option<MacAddress>,
    pub credential: WakeOnLanQueryCredential,
    pub credential_id: Uuid,
}

/// The addresses Wake-on-LAN credentials are assigned to. A WoL credential targets hosts only, so
/// only overrides are read; a default (site-wide) WoL mapping would be a server bug.
pub fn wake_targets(mappings: &[CredentialMapping<CredentialQueryPayload>]) -> Vec<WakeTarget> {
    let mut seen = HashSet::new();
    mappings
        .iter()
        .flat_map(|m| m.ip_overrides.iter())
        .filter_map(|o| match &o.credential {
            CredentialQueryPayload::WakeOnLan(c) => Some(WakeTarget {
                ip: o.ip,
                mac: o.mac_address,
                credential: c.clone(),
                credential_id: o.credential_id,
            }),
            _ => None,
        })
        .filter(|t| seen.insert((t.ip, t.credential_id)))
        .collect()
}

/// The AMD Magic Packet: six `0xFF` bytes, then the target MAC sixteen times, then the SecureOn
/// password if the NIC requires one.
pub fn magic_packet(mac: MacAddress, secure_on: Option<MacAddress>) -> Vec<u8> {
    let mut packet = Vec::with_capacity(108);
    packet.extend_from_slice(&[0xFF; 6]);
    for _ in 0..16 {
        packet.extend_from_slice(&mac.bytes());
    }
    if let Some(password) = secure_on {
        packet.extend_from_slice(&password.bytes());
    }
    packet
}

/// Where to send the packet for `ip`: the credential's own destination if it names one, else the
/// directed broadcast of the known subnet holding `ip`, else the limited broadcast.
///
/// A directed broadcast reaches the target's segment from anywhere a router forwards it, and on
/// the daemon's own segment it is delivered as an L2 broadcast like `255.255.255.255`. The limited
/// broadcast is the fallback for an address in no known subnet: it only reaches the daemon's own
/// segment, which is the one case it can work in.
pub fn destination(ip: IpAddr, broadcast_address: Option<IpAddr>, subnets: &[Subnet]) -> IpAddr {
    if let Some(addr) = broadcast_address {
        return addr;
    }
    let IpAddr::V4(v4) = ip else {
        // IPv6 has no broadcast; a WoL relay address is the way to reach a v6-only host.
        return IpAddr::V4(Ipv4Addr::BROADCAST);
    };
    subnets
        .iter()
        .filter_map(|s| match s.base.cidr.value().0 {
            cidr::IpCidr::V4(c) if c.contains(&v4) => Some(c),
            _ => None,
        })
        // The most specific subnet wins: a /24 inside a /16 is where the host actually sits.
        .max_by_key(|c| c.network_length())
        .filter(|c| c.network_length() < 31)
        .map(|c| IpAddr::V4(c.last_address()))
        .unwrap_or(IpAddr::V4(Ipv4Addr::BROADCAST))
}

/// Send the packets, record every target as not yet woken, and wait for the hosts to boot.
///
/// Never fails the session: a host that will not wake is a finding about that host, and the sweep
/// still has every other host to find.
pub async fn wake(
    ops: &DiscoveryOps,
    mappings: &[CredentialMapping<CredentialQueryPayload>],
    subnets: &[Subnet],
    cancel: &CancellationToken,
) {
    let targets = wake_targets(mappings);
    if targets.is_empty() {
        return;
    }
    tracing::info!(targets = targets.len(), "Waking hosts before the sweep");

    let mut issues = Vec::new();
    let mut sent = Vec::new();
    for target in &targets {
        match send(target, subnets, cancel).await {
            Ok(()) => sent.push(target),
            Err(issue) => issues.push(issue),
        }
    }

    // Every target gets a result. The Wake-on-LAN integration flips the ones the scan finds.
    for target in &targets {
        let result = WakeOnLanResult {
            ip: target.ip,
            woke: false,
        };
        ops.record_wake_on_lan(target.credential_id, result).await;
    }
    if !cancel.is_cancelled() {
        ops.record_credential_issues(&issues).await;
    }

    let wait = sent
        .iter()
        .map(|t| Duration::from_secs(u64::from(t.credential.wait_seconds)))
        .max()
        .unwrap_or_default();
    boot_wait(wait, ops, cancel).await;
}

async fn send(
    target: &WakeTarget,
    subnets: &[Subnet],
    cancel: &CancellationToken,
) -> Result<(), CredentialIssue> {
    let Some(mac) = target.mac else {
        return Err(issue(
            target,
            AttemptOutcome::Malformed,
            "no MAC address is known for this address; scan the host once while it is awake so its MAC is recorded".to_string(),
        ));
    };
    let secure_on = match &target.credential.secure_on_password {
        None => None,
        Some(secret) => {
            let parsed = secret
                .resolve("secure_on_password", "Wake-on-LAN")
                .map_err(|e| e.to_string())
                .and_then(|v| {
                    v.expose_secret()
                        .trim()
                        .parse::<MacAddress>()
                        .map_err(|e| e.to_string())
                });
            match parsed {
                Ok(password) => Some(password),
                Err(e) => {
                    return Err(issue(
                        target,
                        AttemptOutcome::Malformed,
                        format!("could not read the SecureOn password: {e}"),
                    ));
                }
            }
        }
    };

    let dest = SocketAddr::new(
        destination(target.ip, target.credential.broadcast_address, subnets),
        target.credential.port,
    );
    let packet = magic_packet(mac, secure_on);
    let result: std::io::Result<()> = async {
        let bind: SocketAddr = if dest.is_ipv4() {
            (Ipv4Addr::UNSPECIFIED, 0).into()
        } else {
            (std::net::Ipv6Addr::UNSPECIFIED, 0).into()
        };
        let socket = UdpSocket::bind(bind).await?;
        socket.set_broadcast(true)?;
        for i in 0..PACKETS_PER_TARGET {
            if cancel.is_cancelled() {
                break;
            }
            if i > 0 {
                tokio::time::sleep(PACKET_INTERVAL).await;
            }
            socket.send_to(&packet, dest).await?;
        }
        Ok(())
    }
    .await;

    match result {
        Ok(()) => {
            tracing::info!(ip = %target.ip, %mac, %dest, "Sent Wake-on-LAN packets");
            Ok(())
        }
        Err(e) => Err(issue(
            target,
            AttemptOutcome::Unreachable,
            format!("could not send the magic packet to {dest}: {e}"),
        )),
    }
}

/// Pause for `wait`, re-reporting progress so a long boot wait does not trip the stall detector.
async fn boot_wait(wait: Duration, ops: &DiscoveryOps, cancel: &CancellationToken) {
    let deadline = tokio::time::Instant::now() + wait;
    while !cancel.is_cancelled() {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }
        let step = (deadline - now).min(HEARTBEAT_INTERVAL);
        tokio::select! {
            _ = tokio::time::sleep(step) => {}
            _ = cancel.cancelled() => break,
        }
        let _ = ops.heartbeat().await;
    }
}

fn issue(target: &WakeTarget, outcome: AttemptOutcome, message: String) -> CredentialIssue {
    CredentialIssue {
        integration: CredentialQueryPayloadDiscriminants::WakeOnLan,
        ip: target.ip,
        reason: CredentialIssueReason::Attempted { outcome, message },
        credential_id: Some(target.credential_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mac(s: &str) -> MacAddress {
        s.parse().unwrap()
    }

    #[test]
    fn magic_packet_is_sync_stream_then_sixteen_macs() {
        let target = mac("01:23:45:67:89:ab");
        let packet = magic_packet(target, None);
        assert_eq!(packet.len(), 102);
        assert!(packet[..6].iter().all(|b| *b == 0xFF));
        for chunk in packet[6..].chunks(6) {
            assert_eq!(chunk, target.bytes());
        }
    }

    #[test]
    fn secure_on_password_is_appended() {
        let password = mac("de:ad:be:ef:00:01");
        let packet = magic_packet(mac("01:23:45:67:89:ab"), Some(password));
        assert_eq!(packet.len(), 108);
        assert_eq!(&packet[102..], password.bytes());
    }

    fn subnet(cidr: &str) -> Subnet {
        use crate::server::shared::attribution::AttributeSource;
        use crate::server::subnets::r#impl::base::{SubnetBase, SubnetCidr, SubnetCidrValue};
        Subnet {
            base: SubnetBase {
                cidr: SubnetCidr::new(
                    SubnetCidrValue(cidr.parse().unwrap()),
                    AttributeSource::DaemonSelfReport,
                ),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn destination_prefers_override_then_most_specific_subnet_then_limited_broadcast() {
        let ip: IpAddr = "192.168.10.20".parse().unwrap();
        let subnets = [subnet("192.168.0.0/16"), subnet("192.168.10.0/24")];
        let relay: IpAddr = "10.0.0.1".parse().unwrap();

        assert_eq!(destination(ip, Some(relay), &subnets), relay);
        assert_eq!(
            destination(ip, None, &subnets),
            "192.168.10.255".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            destination("172.16.0.5".parse().unwrap(), None, &subnets),
            IpAddr::V4(Ipv4Addr::BROADCAST)
        );
    }
}
