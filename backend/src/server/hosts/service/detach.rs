//! Taking a container's addresses off a host that also holds someone else's.
//!
//! A sweep can file a container's address on another host before the runtime reports it: an
//! ipvlan endpoint answers with its parent NIC's MAC, so the sweep keeps its address as a second
//! address on whichever host holds that MAC. The runtime's report names exactly the container's
//! addresses, so a report that matches such a host takes only those addresses (with what is bound
//! to them) to a host of its own, and leaves the rest where they were.
//!
//! A hypervisor guest gives up the same way: its hypervisor's report names every NIC it has, so
//! an address another NIC answered for goes back to that device (`rows_a_guest_sheds`).
use super::*;
use crate::server::credentials::r#impl::types::CredentialAssignment;
use crate::server::ip_addresses::r#impl::base::MacEvidence;
use crate::server::ports::r#impl::base::PortBase;
use crate::server::services::definitions::snmp::Snmp;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::types::metadata::HasId;
use std::collections::BTreeMap;

impl HostMatch {
    /// The rows a container report takes from the host it matched, or `None` when it adopts that
    /// host as before.
    ///
    /// Taken when the report is a container's, the matched host is not a container, and the host
    /// holds an address beside the ones the report names. The report is the runtime's full list
    /// of the container's addresses, so the host's other addresses are another device's: in the
    /// lab, the VM whose second NIC the ipvlan endpoint shares. A matched host holding only the
    /// container's addresses is the container's own record from the sweep, and is adopted.
    pub(crate) fn rows_a_container_takes(
        &self,
        incoming_virtualization: Option<&HostVirtualization>,
        incoming: &[IPAddress],
    ) -> Option<HashSet<Uuid>> {
        if !incoming_virtualization.is_some_and(HostVirtualization::is_container)
            || self
                .host
                .base
                .virtualization_metadata
                .as_ref()
                .is_some_and(HostVirtualization::is_container)
        {
            return None;
        }
        let (named, others): (Vec<&IPAddress>, Vec<&IPAddress>) = self
            .ip_addresses
            .iter()
            .filter(|row| !should_skip_for_matching(row))
            .partition(|row| incoming.iter().any(|i| ip_addresses_share_address(i, row)));
        (!named.is_empty() && !others.is_empty())
            .then(|| named.into_iter().map(|row| row.id).collect())
    }

    /// The rows a guest gives up to the devices they belong to, one set per device, when its
    /// hypervisor's report names the guest's NICs.
    ///
    /// The report's `HypervisorConfig` MACs are every NIC the hypervisor gave the guest, whichever
    /// hypervisor it is. A stored row whose MAC is none of them is some other device's when either
    /// the outside saw that MAC answer for the address (our ARP reply: a VMware guest an earlier
    /// sweep filed here), or the report itself names that MAC as a link inside the guest (a
    /// macvlan link the guest agent lists, which the report carries as an interface). Each such
    /// MAC is one device and gets its rows back as a host of its own.
    ///
    /// Left alone: rows naming an address the report also names (the guest's own address, whose
    /// MAC the report corrects), rows with no MAC, and rows the guest's own collectors filed with
    /// a MAC the report doesn't name (SNMP, SSH, a container runtime's bridge addresses).
    pub(crate) fn rows_a_guest_sheds(
        &self,
        incoming: &[IPAddress],
        incoming_interfaces: &[Interface],
    ) -> Vec<HashSet<Uuid>> {
        let (config_macs, links_inside): (HashSet<MacAddress>, HashSet<MacAddress>) = {
            let (config, other): (Vec<&MacEvidence>, Vec<&MacEvidence>) = incoming
                .iter()
                .filter_map(|i| i.base.mac_address.as_ref())
                .chain(
                    incoming_interfaces
                        .iter()
                        .filter_map(|i| i.base.mac_address.as_ref()),
                )
                .partition(|e| e.source() == AttributeSource::HypervisorConfig);
            (
                config.into_iter().map(|e| e.value().0).collect(),
                other.into_iter().map(|e| e.value().0).collect(),
            )
        };
        let macs_identify_host = self
            .host
            .base
            .virtualization_metadata
            .as_ref()
            .is_none_or(HostVirtualization::macs_identify_host);
        if config_macs.is_empty() || !macs_identify_host {
            return Vec::new();
        }

        let mut by_mac: BTreeMap<String, HashSet<Uuid>> = BTreeMap::new();
        for row in &self.ip_addresses {
            let Some(evidence) = &row.base.mac_address else {
                continue;
            };
            let mac = evidence.value().0;
            let foreign = !config_macs.contains(&mac)
                && (evidence.source() == AttributeSource::ArpReply || links_inside.contains(&mac));
            if foreign
                && !should_skip_for_matching(row)
                && !incoming.iter().any(|i| ip_addresses_share_address(i, row))
            {
                by_mac
                    .entry(evidence.value().0.to_string())
                    .or_default()
                    .insert(row.id);
            }
        }
        by_mac.into_values().collect()
    }
}

impl HostService {
    /// Give each device whose rows `guest` held (`rows_a_guest_sheds`) a host of its own, and move
    /// its rows there with what only they carry.
    ///
    /// The new host is bare: the device's own next payload (a network identity's report, a deep
    /// scan) matches it by address and fills it in. Once the guest holds no SNMP service, the
    /// `Probe(Snmp)` interfaces on it came from another device's walk that landed here, and are
    /// dropped: that device's walk records them on its own host.
    pub(crate) async fn shed_guest_addresses(
        &self,
        guest: &Host,
        devices: &[HashSet<Uuid>],
        authentication: &AuthenticatedEntity,
    ) -> Result<()> {
        if devices.is_empty() {
            return Ok(());
        }
        for rows in devices {
            let device = self
                .create_unlocked(
                    Host::new(HostBase {
                        site_id: guest.base.site_id,
                        source: EntitySource::Discovery,
                        ..Default::default()
                    }),
                    authentication.clone(),
                    None,
                )
                .await?;
            tracing::info!(
                guest_id = %guest.id,
                device_id = %device.id,
                addresses = rows.len(),
                "Moving addresses a guest's NICs do not carry to a host of their own"
            );
            self.detach_addresses(guest, &device, rows, authentication)
                .await?;
        }

        let runs_snmp = self
            .service_service
            .get_all(StorableFilter::<Service>::new_from_host_ids(&[guest.id]).live())
            .await?
            .iter()
            .any(|s| s.base.service_definition.id() == Snmp.id());
        if !runs_snmp {
            for interface in self.interface_service.get_for_host(&guest.id).await? {
                if interface.base.interface_source == AttributeSource::Probe(ClientProbe::Snmp) {
                    self.interface_service
                        .delete(
                            &interface.id,
                            authentication.clone(),
                            DeletionReason::EntityNoLongerReported,
                        )
                        .await?;
                }
            }
        }
        Ok(())
    }

    /// Move the `rows` of `from` to `to`, with what only they carry.
    ///
    /// - The address rows move with their ids.
    /// - A service on `from` bound only to moved rows moves, and its ports with it. A port another
    ///   service on `from` still binds stays, and `to` gets its own row for it.
    /// - A service bound to moved rows and others stays, without the bindings to moved rows.
    /// - An interface on `from` linked to a moved row stays and loses the link: the NIC is the
    ///   one the two share.
    /// - Credential assignments naming moved rows move those rows to `to`.
    pub(crate) async fn detach_addresses(
        &self,
        from: &Host,
        to: &Host,
        rows: &HashSet<Uuid>,
        authentication: &AuthenticatedEntity,
    ) -> Result<()> {
        let guard = self
            .storage()
            .session_lock(LockKey::Host(from.id), DEFAULT_LOCK_TIMEOUT)
            .await?;

        for mut row in self.ip_address_service.get_for_host(&from.id).await? {
            if rows.contains(&row.id) {
                row.base.host_id = to.id;
                self.ip_address_service
                    .update(
                        &mut row,
                        authentication.clone(),
                        UpdateReason::EntityAddressesDetached {
                            from_host_id: from.id,
                        },
                    )
                    .await?;
            }
        }

        for mut interface in self.interface_service.get_for_host(&from.id).await? {
            if interface
                .base
                .ip_address_id
                .is_some_and(|id| rows.contains(&id))
            {
                interface.base.ip_address_id = None;
                self.interface_service
                    .update(
                        &mut interface,
                        authentication.clone(),
                        UpdateReason::EntityAddressesDetached {
                            from_host_id: from.id,
                        },
                    )
                    .await?;
            }
        }

        let binds_moved_row = |binding: &Binding| match binding.base.binding_type {
            BindingType::IPAddress { ip_address_id } => rows.contains(&ip_address_id),
            BindingType::Port { ip_address_id, .. } => {
                ip_address_id.is_some_and(|id| rows.contains(&id))
            }
        };
        let services = self
            .service_service
            .get_all(StorableFilter::<Service>::new_from_host_ids(&[from.id]).live())
            .await?;
        let (moving, staying): (Vec<Service>, Vec<Service>) = services.into_iter().partition(|s| {
            !s.base.bindings.is_empty() && s.base.bindings.iter().all(binds_moved_row)
        });

        let ports_still_bound: HashSet<Uuid> = staying
            .iter()
            .flat_map(|s| &s.base.bindings)
            .filter(|b| !binds_moved_row(b))
            .filter_map(|b| match b.base.binding_type {
                BindingType::Port { port_id, .. } => Some(port_id),
                BindingType::IPAddress { .. } => None,
            })
            .collect();
        let from_ports = self.port_service.get_for_host(&from.id).await?;
        let mut port_id_map: HashMap<Uuid, Uuid> = HashMap::new();
        for port_id in moving
            .iter()
            .flat_map(|s| &s.base.bindings)
            .filter_map(|b| match b.base.binding_type {
                BindingType::Port { port_id, .. } => Some(port_id),
                BindingType::IPAddress { .. } => None,
            })
            .collect::<HashSet<Uuid>>()
        {
            let Some(port) = from_ports.iter().find(|p| p.id == port_id) else {
                continue;
            };
            let new_id = if ports_still_bound.contains(&port_id) {
                self.port_service
                    .create(
                        Port::new(PortBase::new(to.id, to.base.site_id, port.base.port_type))
                            .with_source(port.base.port_source),
                        authentication.clone(),
                    )
                    .await?
                    .id
            } else {
                let mut moved = port.with_host(to.id, to.base.site_id);
                self.port_service
                    .update(
                        &mut moved,
                        authentication.clone(),
                        UpdateReason::EntityAddressesDetached {
                            from_host_id: from.id,
                        },
                    )
                    .await?;
                port_id
            };
            port_id_map.insert(port_id, new_id);
        }

        for mut service in moving {
            service.base.host_id = to.id;
            for binding in &mut service.base.bindings {
                if let BindingType::Port { port_id, .. } = &mut binding.base.binding_type
                    && let Some(&new_id) = port_id_map.get(port_id)
                {
                    *port_id = new_id;
                }
            }
            self.service_service
                .update(
                    &mut service,
                    authentication.clone(),
                    UpdateReason::EntityAddressesDetached {
                        from_host_id: from.id,
                    },
                )
                .await?;
        }
        for mut service in staying {
            let before = service.base.bindings.len();
            service.base.bindings.retain(|b| !binds_moved_row(b));
            if service.base.bindings.len() != before {
                self.service_service
                    .update(
                        &mut service,
                        authentication.clone(),
                        UpdateReason::EntityAddressesDetached {
                            from_host_id: from.id,
                        },
                    )
                    .await?;
            }
        }

        let assignments = self
            .credential_service
            .get_credential_assignments_for_host(&from.id)
            .await?;
        let mut kept: Vec<CredentialAssignment> = Vec::new();
        let mut taken: Vec<CredentialAssignment> = Vec::new();
        for assignment in &assignments {
            let Some(ids) = &assignment.ip_address_ids else {
                kept.push(assignment.clone());
                continue;
            };
            let (moved, remaining): (Vec<Uuid>, Vec<Uuid>) =
                ids.iter().partition(|id| rows.contains(id));
            if !moved.is_empty() {
                taken.push(CredentialAssignment {
                    credential_id: assignment.credential_id,
                    ip_address_ids: Some(moved),
                });
            }
            if !remaining.is_empty() {
                kept.push(CredentialAssignment {
                    credential_id: assignment.credential_id,
                    ip_address_ids: Some(remaining),
                });
            }
        }
        if !taken.is_empty() {
            self.credential_service
                .set_host_credentials(&from.id, &kept)
                .await?;
            self.credential_service
                .set_host_credentials(&to.id, &taken)
                .await?;
        }

        guard.release().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::virtualization::{
        ContainerHostVirtualization, ContainerNetworkType, HostVirtualizationDiscriminants,
    };
    use crate::server::ip_addresses::r#impl::base::{IPAddressBase, MacEvidence, MacEvidenceValue};

    fn row(host_id: Uuid, subnet_id: Uuid, ip: &str) -> IPAddress {
        IPAddress::new(IPAddressBase {
            host_id,
            subnet_id,
            ip_address: ip.parse().unwrap(),
            ..Default::default()
        })
    }

    fn container() -> HostVirtualization {
        HostVirtualization::Docker(ContainerHostVirtualization {
            state: Default::default(),
            container_name: Some("ipvlan-test".to_string()),
            container_id: Some("aec2a3e9".to_string()),
            compose_project: None,
            network_type: ContainerNetworkType::IpVlan,
        })
    }

    fn matched(virtualization: Option<HostVirtualization>, rows: Vec<IPAddress>) -> HostMatch {
        let mut host = Host::new(HostBase::default());
        host.base.virtualization_metadata = virtualization;
        HostMatch {
            host,
            matched_by: HostMatchedBy::Mac,
            ip_addresses: rows,
            same_device: vec![],
        }
    }

    /// The lab: the sweep kept the ipvlan endpoint's .231 beside the VM's .63 behind one MAC. The
    /// runtime's report for .231 takes .231 and leaves .63.
    #[test]
    fn a_container_report_takes_only_its_own_addresses() {
        let (host, lan) = (Uuid::new_v4(), Uuid::new_v4());
        let vm_address = row(host, lan, "192.168.4.63");
        let container_address = row(host, lan, "192.168.7.231");
        let m = matched(None, vec![vm_address, container_address.clone()]);
        let incoming = [row(Uuid::nil(), lan, "192.168.7.231")];

        assert_eq!(
            m.rows_a_container_takes(Some(&container()), &incoming),
            Some(HashSet::from([container_address.id]))
        );
    }

    /// The same holds when the host the report matched is a guest of any hypervisor: a container
    /// is never that guest.
    #[test]
    fn a_guest_holding_a_container_address_gives_it_up() {
        for kind in HostVirtualizationDiscriminants::hypervisor_guests() {
            let (host, lan) = (Uuid::new_v4(), Uuid::new_v4());
            let m = matched(
                Some(kind.guest("docker", "103")),
                vec![
                    row(host, lan, "192.168.4.63"),
                    row(host, lan, "192.168.7.231"),
                ],
            );
            let incoming = [row(Uuid::nil(), lan, "192.168.7.231")];
            assert!(
                m.rows_a_container_takes(Some(&container()), &incoming)
                    .is_some(),
                "{kind:?}"
            );
        }
    }

    /// A host holding only the container's addresses is the container's record from the sweep.
    #[test]
    fn a_host_holding_only_the_containers_addresses_is_adopted() {
        let (host, lan) = (Uuid::new_v4(), Uuid::new_v4());
        let m = matched(None, vec![row(host, lan, "192.168.7.231")]);
        let incoming = [row(Uuid::nil(), lan, "192.168.7.231")];
        assert_eq!(
            m.rows_a_container_takes(Some(&container()), &incoming),
            None
        );
    }

    /// A container's own host, whatever else it was given, is adopted: its report is a rescan.
    #[test]
    fn a_container_host_is_adopted() {
        let (host, lan) = (Uuid::new_v4(), Uuid::new_v4());
        let m = matched(
            Some(container()),
            vec![
                row(host, lan, "192.168.4.63"),
                row(host, lan, "192.168.7.231"),
            ],
        );
        let incoming = [row(Uuid::nil(), lan, "192.168.7.231")];
        assert_eq!(
            m.rows_a_container_takes(Some(&container()), &incoming),
            None
        );
    }

    fn with_mac(mut row: IPAddress, mac: &str, source: AttributeSource) -> IPAddress {
        row.base.mac_address = Some(MacEvidence::new(
            MacEvidenceValue(mac.parse().unwrap()),
            source,
        ));
        row
    }

    fn snmp_test() -> HostVirtualization {
        HostVirtualizationDiscriminants::Proxmox.guest("snmp-test", "107")
    }

    const ETH0: &str = "02:e9:65:cc:7f:7b";

    /// snmp-test as the dev DB held it on 2026-10-22, receiving its hypervisor's report: eth0's
    /// addresses under the config MAC, and an interface per macvlan link under the link's MAC.
    /// Three other devices' rows come off it, one host each: the VMware guest at 6.1 and the two
    /// sim links. The guest's own addresses, the `docker0` row its own SNMP read filed, and a
    /// container's bridge address its runtime reported stay. The same under every hypervisor.
    #[test]
    fn a_guest_report_sheds_each_other_devices_rows_to_its_own_set() {
        for kind in HostVirtualizationDiscriminants::hypervisor_guests() {
            sheds_each_other_devices_rows(kind.guest("snmp-test", "107"));
        }
    }

    fn sheds_each_other_devices_rows(guest: HostVirtualization) {
        let (vm, lan, v6, bridge) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let config = AttributeSource::HypervisorConfig;
        let agent = AttributeSource::GuestAgent;
        let arp = AttributeSource::ArpReply;
        let own_421 = with_mac(row(vm, lan, "192.168.4.21"), ETH0, arp);
        let own_596 = with_mac(row(vm, lan, "192.168.5.96"), ETH0, config);
        let own_v6 = with_mac(
            row(vm, v6, "fd0b:d38d:98f6:1:e9:65ff:fecc:7f7b"),
            ETH0,
            config,
        );
        let vmware = with_mac(row(vm, lan, "192.168.6.1"), "00:0c:29:87:81:5d", arp);
        let link_230 = with_mac(row(vm, lan, "192.168.7.230"), "ca:ef:69:ca:e3:4c", arp);
        let link_230_v6 = with_mac(
            row(vm, v6, "fd0b:d38d:98f6:1:c8ef:69ff:feca:e34c"),
            "ca:ef:69:ca:e3:4c",
            agent,
        );
        let link_231 = with_mac(row(vm, lan, "192.168.7.231"), "42:d3:d4:77:c4:0f", arp);
        let link_231_v6 = with_mac(
            row(vm, v6, "fd0b:d38d:98f6:1:40d3:d4ff:fe77:c40f"),
            "42:d3:d4:77:c4:0f",
            agent,
        );
        let docker0 = with_mac(
            row(vm, bridge, "172.17.0.1"),
            "02:42:ac:11:00:01",
            AttributeSource::Probe(ClientProbe::Snmp),
        );
        let container_bridge = with_mac(
            row(vm, bridge, "172.17.0.2"),
            "f6:af:f2:ff:a5:ff",
            AttributeSource::Probe(ClientProbe::Docker),
        );
        let m = matched(
            Some(guest),
            vec![
                own_421,
                own_596,
                own_v6,
                vmware.clone(),
                link_230.clone(),
                link_230_v6.clone(),
                link_231.clone(),
                link_231_v6.clone(),
                docker0,
                container_bridge,
            ],
        );
        let link = |mac: &str| {
            use crate::server::interfaces::r#impl::base::InterfaceBase;
            Interface::new(InterfaceBase {
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(mac.parse().unwrap()),
                    agent,
                )),
                interface_source: agent,
                ..Default::default()
            })
        };
        let links = [link("ca:ef:69:ca:e3:4c"), link("42:d3:d4:77:c4:0f")];
        let report = [
            with_mac(row(Uuid::nil(), lan, "192.168.4.21"), ETH0, config),
            with_mac(row(Uuid::nil(), lan, "192.168.5.96"), ETH0, config),
            with_mac(
                row(Uuid::nil(), v6, "fd0b:d38d:98f6:1:e9:65ff:fecc:7f7b"),
                ETH0,
                config,
            ),
        ];

        let mut shed = m.rows_a_guest_sheds(&report, &links);
        shed.sort_by_key(|set| set.len());
        let expected = [
            HashSet::from([vmware.id]),
            HashSet::from([link_230.id, link_230_v6.id]),
            HashSet::from([link_231.id, link_231_v6.id]),
        ];
        assert_eq!(shed.len(), expected.len());
        for set in &expected {
            assert!(shed.contains(set), "missing {set:?}");
        }
    }

    /// A payload naming no config MAC (a sweep, an SNMP read of the guest) says nothing about the
    /// guest's NICs, so nothing is shed; nor from a host whose MACs are its runtime's.
    #[test]
    fn nothing_is_shed_without_a_config_mac_or_from_an_ipvlan_host() {
        let (vm, lan) = (Uuid::new_v4(), Uuid::new_v4());
        let rows = vec![
            with_mac(
                row(vm, lan, "192.168.4.21"),
                ETH0,
                AttributeSource::HypervisorConfig,
            ),
            with_mac(
                row(vm, lan, "192.168.6.1"),
                "00:0c:29:87:81:5d",
                AttributeSource::ArpReply,
            ),
        ];
        let sweep = [with_mac(
            row(Uuid::nil(), lan, "192.168.4.21"),
            ETH0,
            AttributeSource::ArpReply,
        )];
        assert!(
            matched(Some(snmp_test()), rows.clone())
                .rows_a_guest_sheds(&sweep, &[])
                .is_empty()
        );

        let report = [with_mac(
            row(Uuid::nil(), lan, "192.168.4.21"),
            ETH0,
            AttributeSource::HypervisorConfig,
        )];
        assert!(
            matched(Some(container()), rows)
                .rows_a_guest_sheds(&report, &[])
                .is_empty()
        );
    }

    /// A guest with no address still names its NIC on an interface row, which is enough.
    #[test]
    fn a_config_mac_on_an_interface_row_names_the_guests_nic() {
        use crate::server::interfaces::r#impl::base::InterfaceBase;
        let (vm, lan) = (Uuid::new_v4(), Uuid::new_v4());
        let stray = with_mac(
            row(vm, lan, "192.168.6.1"),
            "00:0c:29:87:81:5d",
            AttributeSource::ArpReply,
        );
        let m = matched(Some(snmp_test()), vec![stray.clone()]);
        let nic = Interface::new(InterfaceBase {
            mac_address: Some(MacEvidence::new(
                MacEvidenceValue(ETH0.parse().unwrap()),
                AttributeSource::HypervisorConfig,
            )),
            ..Default::default()
        });
        assert_eq!(
            m.rows_a_guest_sheds(&[], &[nic]),
            vec![HashSet::from([stray.id])]
        );
    }

    /// Only a container's report names a full address list; any other payload adopts as before.
    #[test]
    fn a_payload_that_is_not_a_container_adopts() {
        let (host, lan) = (Uuid::new_v4(), Uuid::new_v4());
        let m = matched(
            None,
            vec![
                row(host, lan, "192.168.4.63"),
                row(host, lan, "192.168.7.231"),
            ],
        );
        let incoming = [row(Uuid::nil(), lan, "192.168.7.231")];
        assert_eq!(m.rows_a_container_takes(None, &incoming), None);
    }
}
