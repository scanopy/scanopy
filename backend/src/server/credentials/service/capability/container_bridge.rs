//! Which daemon is sent a credential override for a container's bridge address.
//!
//! A container on a Docker or Podman bridge has no address on the LAN. Its bridge address is an
//! IP row on the runtime host, so a host assignment covering it already becomes an override, but
//! only a daemon on that host can route to it. Two separate questions follow:
//!
//! - **Reachability** ([`retain_reachable_overrides`]): a daemon on another machine can
//!   never reach the address, so it is not sent it, silently. No upgrade would change that. The
//!   same holds for another host's loopback, which on this daemon would be its own machine.
//! - **Capability** ([`TargetsContainerBridgeWithCredentials`]): the runtime host's own daemon
//!   reaches it, but only from the release that tries credentials at its bridge containers. An
//!   older one is not sent it, and the scan says which addresses an upgrade would add.
//!
//! One rule here has nothing to do with bridges: [`retain_overrides_scanned_here`] leaves an
//! address another discovery on the site scans to that discovery's daemon.

use super::DaemonCapability;
use crate::server::credentials::r#impl::capability::{CapabilityGap, ContainerBridgeGap};
use crate::server::credentials::service::TypedCredentialMapping;
use crate::server::daemons::r#impl::version::last_without_bridge_credentials;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::subnets::r#impl::base::Subnet;
use semver::Version;
use std::collections::{BTreeMap, HashSet};
use std::net::IpAddr;
use uuid::Uuid;

/// Where the daemon a dispatch is built for runs, and which of the site's subnets other scans
/// cover.
#[derive(Debug, Clone, Default)]
pub struct DaemonPlacement {
    /// The host the daemon runs on.
    pub daemon_host_id: Option<Uuid>,
    /// The site's Docker and Podman bridge subnets.
    pub bridge_subnet_ids: HashSet<Uuid>,
    /// Subnets another discovery on the site scans and this daemon's does not.
    pub subnets_scanned_elsewhere: HashSet<Uuid>,
}

impl DaemonPlacement {
    pub fn new(
        daemon_host_id: Option<Uuid>,
        site_subnets: &[Subnet],
        subnets_scanned_elsewhere: HashSet<Uuid>,
    ) -> Self {
        Self {
            daemon_host_id,
            subnets_scanned_elsewhere,
            bridge_subnet_ids: site_subnets
                .iter()
                .filter(|s| s.is_container_bridge_subnet())
                .map(|s| s.id)
                .collect(),
        }
    }
}

/// The site's bridge addresses, each with the host that holds it, and the daemon's host.
pub(crate) struct ContainerBridgeContext {
    daemon_host_id: Option<Uuid>,
    bridge_addresses: HashSet<(Uuid, IpAddr)>,
}

impl ContainerBridgeContext {
    pub(crate) fn new(placement: &DaemonPlacement, ip_addresses: &[IPAddress]) -> Self {
        Self {
            daemon_host_id: placement.daemon_host_id,
            bridge_addresses: ip_addresses
                .iter()
                .filter(|i| placement.bridge_subnet_ids.contains(&i.base.subnet_id))
                .map(|i| (i.base.host_id, i.base.ip_address))
                .collect(),
        }
    }

    /// The host holding `ip` on a bridge, for an override naming `host_id`. Bridge ranges repeat
    /// on every Docker host, so the address alone does not say whether it is a bridge one.
    fn is_bridge(&self, host_id: Option<Uuid>, ip: IpAddr) -> Option<Uuid> {
        host_id.filter(|h| self.bridge_addresses.contains(&(*h, ip)))
    }
}

/// Drop every host-assignment override at an address only another host can reach: its bridge
/// address, or its loopback. A daemon reaching `127.0.0.1` reaches its own machine, so another
/// host's loopback assignment (a Docker or Podman socket, or a proxy on that host) would be tried
/// against this daemon's own socket. The seeded socket credentials are one per organization and
/// earn a loopback assignment on every daemon host, so each daemon keeps exactly its own.
///
/// Overrides without a host (discovery `integration_targets`, including a daemon-host target)
/// name an address the user chose for this daemon and are left alone, and so is everything when
/// the daemon's host is unknown.
pub(crate) fn retain_reachable_overrides(
    ctx: &ContainerBridgeContext,
    mappings: &mut BTreeMap<Uuid, TypedCredentialMapping>,
) {
    let Some(daemon_host_id) = ctx.daemon_host_id else {
        return;
    };
    for typed in mappings.values_mut() {
        typed.mapping.ip_overrides.retain(|o| {
            let other_host = o.host_id.is_some_and(|h| h != daemon_host_id);
            !(other_host && (o.ip.is_loopback() || ctx.is_bridge(o.host_id, o.ip).is_some()))
        });
    }
}

/// Drop every host-assignment override at an address in a subnet another discovery on the site
/// scans and this daemon's does not. That discovery's daemon tries the credential there, and this
/// one would only report the address as outside its scan. An address no discovery scans is kept,
/// so the daemon still reports it: that warning tells the user the credential sits outside every
/// scan.
///
/// The daemon's own host keeps every address. Subnets are keyed by CIDR, so another daemon's
/// interfaced range can be this host's container bridge, which only this daemon reaches.
pub(crate) fn retain_overrides_scanned_here(
    placement: &DaemonPlacement,
    ip_addresses: &[IPAddress],
    mappings: &mut BTreeMap<Uuid, TypedCredentialMapping>,
) {
    if placement.subnets_scanned_elsewhere.is_empty() {
        return;
    }
    let scanned_elsewhere: HashSet<(Uuid, IpAddr)> = ip_addresses
        .iter()
        .filter(|i| Some(i.base.host_id) != placement.daemon_host_id)
        .filter(|i| {
            placement
                .subnets_scanned_elsewhere
                .contains(&i.base.subnet_id)
        })
        .map(|i| (i.base.host_id, i.base.ip_address))
        .collect();
    for typed in mappings.values_mut() {
        typed.mapping.ip_overrides.retain(|o| {
            !o.host_id
                .is_some_and(|h| scanned_elsewhere.contains(&(h, o.ip)))
        });
    }
}

/// Trying a credential at a container's bridge address on the daemon's own host.
pub(crate) struct TargetsContainerBridgeWithCredentials;

impl From<ContainerBridgeGap> for CapabilityGap {
    fn from(gap: ContainerBridgeGap) -> Self {
        CapabilityGap::TargetsContainerBridgeWithCredentials(gap)
    }
}

impl DaemonCapability for TargetsContainerBridgeWithCredentials {
    type Context = ContainerBridgeContext;
    type Gap = ContainerBridgeGap;

    fn gap(ctx: &Self::Context, credential: &TypedCredentialMapping) -> Option<Self::Gap> {
        let addresses: Vec<IpAddr> = credential
            .mapping
            .ip_overrides
            .iter()
            .filter(|o| ctx.is_bridge(o.host_id, o.ip).is_some())
            .map(|o| o.ip)
            .collect();
        (!addresses.is_empty()).then_some(ContainerBridgeGap { addresses })
    }

    fn last_without(_: &Self::Gap) -> Version {
        last_without_bridge_credentials()
    }

    fn withhold(gap: &Self::Gap, credential: &mut TypedCredentialMapping) {
        credential
            .mapping
            .ip_overrides
            .retain(|o| !gap.addresses.contains(&o.ip));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::types::warnings::DiscoveryWarning;
    use crate::server::credentials::r#impl::{
        base::{Credential, CredentialBase},
        types::{CredentialAssignment, CredentialTypeDiscriminants},
    };
    use crate::server::credentials::service::apply_host_assignment_standalone;
    use crate::server::credentials::service::capability::retain_capable;
    use crate::server::ip_addresses::r#impl::base::IPAddressBase;

    const LAN: &str = "10.0.20.20";
    const BRIDGE: &str = "172.17.0.3";

    struct Fixture {
        runtime_host: Uuid,
        bridge_subnet: Uuid,
        bridge_ip_id: Uuid,
        credential: Credential,
        ip_addresses: Vec<IPAddress>,
    }

    fn ip(id: Uuid, host_id: Uuid, subnet_id: Uuid, address: &str) -> IPAddress {
        IPAddress {
            id,
            base: IPAddressBase {
                host_id,
                subnet_id,
                ip_address: address.parse().unwrap(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn fixture() -> Fixture {
        let runtime_host = Uuid::new_v4();
        let bridge_subnet = Uuid::new_v4();
        let bridge_ip_id = Uuid::new_v4();
        Fixture {
            runtime_host,
            bridge_subnet,
            bridge_ip_id,
            credential: Credential {
                id: Uuid::new_v4(),
                base: CredentialBase {
                    credential_type: CredentialTypeDiscriminants::SshScript.to_credential_type(),
                    ..Default::default()
                },
                ..Default::default()
            },
            ip_addresses: vec![
                ip(Uuid::new_v4(), runtime_host, Uuid::new_v4(), LAN),
                ip(bridge_ip_id, runtime_host, bridge_subnet, BRIDGE),
            ],
        }
    }

    fn placement(f: &Fixture, daemon_host_id: Uuid) -> DaemonPlacement {
        DaemonPlacement {
            daemon_host_id: Some(daemon_host_id),
            bridge_subnet_ids: HashSet::from([f.bridge_subnet]),
            ..Default::default()
        }
    }

    fn version(tries: bool) -> Version {
        let last = last_without_bridge_credentials();
        if tries {
            Version::new(last.major, last.minor, last.patch + 1)
        } else {
            last
        }
    }

    /// Reachability then the capability, as `build_all_credential_mappings` runs them: the
    /// overrides a host assignment leaves for one daemon, and the warnings raised.
    fn dispatched(
        f: &Fixture,
        ip_address_ids: Option<Vec<Uuid>>,
        daemon_host_id: Uuid,
        tries: bool,
    ) -> (Vec<IpAddr>, Vec<DiscoveryWarning>) {
        let mut map = BTreeMap::new();
        apply_host_assignment_standalone(
            &mut map,
            f.runtime_host,
            &CredentialAssignment {
                credential_id: f.credential.id,
                ip_address_ids,
            },
            &f.credential,
            &f.ip_addresses,
        );
        let ctx = ContainerBridgeContext::new(&placement(f, daemon_host_id), &f.ip_addresses);
        retain_reachable_overrides(&ctx, &mut map);
        let warnings = retain_capable::<TargetsContainerBridgeWithCredentials>(
            &ctx,
            &mut map,
            Some(&version(tries)),
        );
        let mut ips: Vec<IpAddr> = map
            .values()
            .flat_map(|t| t.mapping.ip_overrides.iter().map(|o| o.ip))
            .collect();
        ips.sort();
        (ips, warnings)
    }

    fn addr(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    /// Pins the expansion the filters work on: a host-wide assignment covers the bridge address
    /// of a container counted on its host, as it did before bridge addresses were tried at all.
    #[test]
    fn host_wide_assignment_expands_to_bridge_addresses() {
        let f = fixture();
        let mut map = BTreeMap::new();
        apply_host_assignment_standalone(
            &mut map,
            f.runtime_host,
            &CredentialAssignment {
                credential_id: f.credential.id,
                ip_address_ids: None,
            },
            &f.credential,
            &f.ip_addresses,
        );
        let ips: HashSet<IpAddr> = map
            .values()
            .flat_map(|t| t.mapping.ip_overrides.iter().map(|o| o.ip))
            .collect();
        assert!(ips.contains(&addr(BRIDGE)));
    }

    #[test]
    fn the_runtime_hosts_daemon_gets_bridge_overrides_host_wide_and_scoped() {
        let f = fixture();
        let (ips, warnings) = dispatched(&f, None, f.runtime_host, true);
        assert_eq!(ips, vec![addr(LAN), addr(BRIDGE)]);
        assert!(warnings.is_empty());
        let (ips, _) = dispatched(&f, Some(vec![f.bridge_ip_id]), f.runtime_host, true);
        assert_eq!(ips, vec![addr(BRIDGE)]);
    }

    /// A daemon elsewhere could never reach the container, so it is told nothing, whatever its
    /// version.
    #[test]
    fn another_daemon_gets_no_bridge_overrides_and_no_warning() {
        let f = fixture();
        for tries in [true, false] {
            let (ips, warnings) = dispatched(&f, None, Uuid::new_v4(), tries);
            assert_eq!(ips, vec![addr(LAN)]);
            assert!(warnings.is_empty());
            let (ips, warnings) = dispatched(&f, Some(vec![f.bridge_ip_id]), Uuid::new_v4(), tries);
            assert!(ips.is_empty());
            assert!(warnings.is_empty());
        }
    }

    /// The runtime host's own daemon, when too old, keeps the LAN address and is told which bridge
    /// addresses an upgrade would add, whether the assignment is scoped or host-wide.
    #[test]
    fn an_old_runtime_host_daemon_is_told_which_bridge_addresses_it_missed() {
        let f = fixture();
        for scope in [None, Some(vec![f.bridge_ip_id])] {
            let host_wide = scope.is_none();
            let (ips, warnings) = dispatched(&f, scope, f.runtime_host, false);
            assert_eq!(ips, if host_wide { vec![addr(LAN)] } else { vec![] });
            match warnings.as_slice() {
                [
                    DiscoveryWarning::CredentialNeedsNewerDaemon {
                        credential_id: Some(id),
                        gap: CapabilityGap::TargetsContainerBridgeWithCredentials(gap),
                        last_unsupported_daemon_version,
                        ..
                    },
                ] => {
                    assert_eq!(*id, f.credential.id);
                    assert_eq!(gap.addresses, vec![addr(BRIDGE)]);
                    assert_eq!(*last_unsupported_daemon_version, version(false));
                }
                other => panic!("expected one bridge gap, got {other:?}"),
            }
        }
    }

    /// Bridge ranges repeat on every Docker host, so the same address on another host's bridge
    /// must not decide whether this host's override is a bridge one.
    #[test]
    fn the_same_bridge_address_on_another_host_does_not_filter_a_lan_override() {
        let mut f = fixture();
        f.ip_addresses
            .push(ip(Uuid::new_v4(), Uuid::new_v4(), f.bridge_subnet, LAN));
        let (ips, warnings) = dispatched(&f, None, Uuid::new_v4(), true);
        assert_eq!(ips, vec![addr(LAN)]);
        assert!(warnings.is_empty());
    }

    /// The seeded Docker socket credential is one per organization: every daemon that probes it
    /// successfully earns an assignment on its own host's loopback. Each daemon must be sent only
    /// its own, plus any daemon-host target its discovery names, or it tries another host's socket
    /// against its own machine.
    #[test]
    fn a_shared_socket_credential_reaches_each_daemon_only_at_its_own_loopback() {
        use crate::server::credentials::r#impl::mapping::IntegrationTarget;
        use crate::server::credentials::service::apply_integration_target_standalone;

        let socket = Credential {
            id: Uuid::new_v4(),
            base: CredentialBase {
                credential_type: CredentialTypeDiscriminants::DockerSocket.to_credential_type(),
                ..Default::default()
            },
            ..Default::default()
        };
        let (host_a, host_b) = (Uuid::new_v4(), Uuid::new_v4());
        let (lo_a, lo_b) = (Uuid::new_v4(), Uuid::new_v4());
        let ip_addresses = vec![
            ip(lo_a, host_a, Uuid::new_v4(), "127.0.0.1"),
            ip(lo_b, host_b, Uuid::new_v4(), "127.0.0.1"),
        ];

        let sent_to = |daemon_host: Uuid, with_target: bool| {
            let mut map = BTreeMap::new();
            for (host, lo) in [(host_a, lo_a), (host_b, lo_b)] {
                apply_host_assignment_standalone(
                    &mut map,
                    host,
                    &CredentialAssignment {
                        credential_id: socket.id,
                        ip_address_ids: Some(vec![lo]),
                    },
                    &socket,
                    &ip_addresses,
                );
            }
            // The order `build_all_credential_mappings` runs them in: reachability, then targets.
            let placement = DaemonPlacement {
                daemon_host_id: Some(daemon_host),
                ..Default::default()
            };
            retain_reachable_overrides(
                &ContainerBridgeContext::new(&placement, &ip_addresses),
                &mut map,
            );
            if with_target {
                apply_integration_target_standalone(
                    &mut map,
                    &IntegrationTarget::DaemonHost {
                        credential_id: socket.id,
                    },
                    &socket.base.credential_type,
                );
            }
            map.values()
                .flat_map(|t| t.mapping.ip_overrides.iter().map(|o| o.host_id))
                .collect::<Vec<_>>()
        };

        assert_eq!(sent_to(host_a, false), vec![Some(host_a)]);
        assert_eq!(sent_to(host_b, false), vec![Some(host_b)]);
        // A new daemon whose host has no assignment yet still gets its daemon-host target, though
        // other hosts hold the same credential at the same 127.0.0.1.
        assert_eq!(sent_to(Uuid::new_v4(), true), vec![None]);
    }

    /// A host-wide assignment on a host with one address in another daemon's subnet and one in a
    /// subnet nobody scans: this daemon keeps only the second, which it then reports as outside
    /// its scan.
    #[test]
    fn an_address_another_discovery_scans_is_left_to_it() {
        let f = fixture();
        let (theirs, nobodys) = (Uuid::new_v4(), Uuid::new_v4());
        let ip_addresses = vec![
            ip(Uuid::new_v4(), f.runtime_host, theirs, "10.0.20.20"),
            ip(Uuid::new_v4(), f.runtime_host, nobodys, "10.0.30.30"),
        ];
        let placement = DaemonPlacement {
            daemon_host_id: Some(Uuid::new_v4()),
            subnets_scanned_elsewhere: HashSet::from([theirs]),
            ..Default::default()
        };

        let kept = |placement: &DaemonPlacement| {
            let mut map = BTreeMap::new();
            apply_host_assignment_standalone(
                &mut map,
                f.runtime_host,
                &CredentialAssignment {
                    credential_id: f.credential.id,
                    ip_address_ids: None,
                },
                &f.credential,
                &ip_addresses,
            );
            retain_overrides_scanned_here(placement, &ip_addresses, &mut map);
            map.values()
                .flat_map(|t| t.mapping.ip_overrides.iter().map(|o| o.ip))
                .collect::<Vec<IpAddr>>()
        };

        assert_eq!(kept(&placement), vec![addr("10.0.30.30")]);
        // The host's own daemon keeps both: the range another daemon interfaces may be a bridge
        // on this host.
        let own = DaemonPlacement {
            daemon_host_id: Some(f.runtime_host),
            ..placement
        };
        assert_eq!(kept(&own), vec![addr("10.0.20.20"), addr("10.0.30.30")]);
    }
}
