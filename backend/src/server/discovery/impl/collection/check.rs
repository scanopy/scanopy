//! Checking what a producer submitted against what it declares collecting.

use std::collections::HashSet;

use strum::IntoEnumIterator;

use crate::server::{
    credentials::r#impl::types::CredentialIntegration,
    hosts::r#impl::{
        base::{Host, HostBase},
        virtualization::{HostVirtualizationDiscriminants, HostVirtualizationState},
    },
    interfaces::r#impl::base::{Interface, InterfaceBase},
    ip_addresses::r#impl::base::IPAddress,
    ports::r#impl::base::Port,
    services::r#impl::{
        base::Service, patterns::ClientProbe, virtualization::ServiceVirtualization,
    },
    shared::attribution::{AttributeSource, AttributeValue, Attributed},
    subnets::r#impl::base::Subnet,
};

use super::{CollectedData, Producer};

/// What a guest host of virtualization `kind` counts as collected, and under whose source: the
/// integration that declares reporting it ([`CredentialIntegration::host_virtualizations`]),
/// through the source its own `collection` lists for that category. A kind no integration
/// declares gets `Unspecified`, so a producer that starts to report it is flagged until one
/// declares it. `None` for a network identity: an address the host presents from an interface of
/// its own, whose addresses already count.
fn guest_collection(
    kind: HostVirtualizationDiscriminants,
) -> Option<(CollectedData, AttributeSource)> {
    let category = match kind.state() {
        HostVirtualizationState::Virtualized => CollectedData::VirtualMachines,
        HostVirtualizationState::Containerized => CollectedData::Containers,
        HostVirtualizationState::NetworkIdentity | HostVirtualizationState::BareMetal => {
            return None;
        }
    };
    let source = CredentialIntegration::iter()
        .find(|i| i.host_virtualizations().contains(&kind))
        .and_then(|i| {
            i.collection()
                .iter()
                .find(|(_, categories)| categories.contains(&category))
                .map(|(source, _)| *source)
        })
        .unwrap_or(AttributeSource::Unspecified);
    Some((category, source))
}

/// The `(category, source)` pairs a host submission carries, derived from which of its fields
/// hold data and the source each one records.
///
/// The host and interface fields are destructured in full, so a new field does not compile until
/// it is placed in a category or deliberately left out. Services carry no source of their own, so
/// they count through their ports and, for containers, their virtualization.
pub fn present_in(
    host: &Host,
    ip_addresses: &[IPAddress],
    ports: &[Port],
    services: &[Service],
    interfaces: &[Interface],
    subnets: &[Subnet],
) -> HashSet<(CollectedData, AttributeSource)> {
    let mut present = HashSet::new();
    let mut mark = |category: CollectedData, source: AttributeSource| {
        present.insert((category, source));
    };
    fn source_of<T: AttributeValue>(slot: &Option<Attributed<T>>) -> Option<AttributeSource> {
        slot.as_ref().map(Attributed::source)
    }

    let HostBase {
        hostname,
        os,
        sys_descr,
        sys_object_id,
        sys_location,
        sys_contact,
        sys_name,
        manufacturer,
        model,
        serial_number,
        asset_tag,
        firmware_revision,
        software_revision,
        matched_device_type,
        virtualization_metadata,
        // Identifiers and bookkeeping rather than collected data. The chassis id names the device
        // for neighbor matching; the management URL is where to reach it.
        name: _,
        site_id: _,
        description: _,
        source: _,
        virtualization_service_id: _,
        virtualization_interface_id: _,
        tags: _,
        management_url: _,
        chassis_id: _,
        credential_assignments: _,
    } = &host.base;

    if let Some(source) = source_of(hostname) {
        mark(CollectedData::Hostnames, source);
    }
    if let Some(source) = source_of(os) {
        mark(CollectedData::OperatingSystem, source);
    }
    for source in [
        source_of(sys_descr),
        source_of(sys_object_id),
        source_of(sys_location),
        source_of(sys_contact),
        source_of(sys_name),
        source_of(manufacturer),
        source_of(model),
        source_of(serial_number),
        source_of(asset_tag),
        source_of(firmware_revision),
        source_of(software_revision),
        source_of(matched_device_type),
    ]
    .into_iter()
    .flatten()
    {
        mark(CollectedData::DeviceDetails, source);
    }

    for ip_address in ip_addresses {
        let source = ip_address.base.ip_address_source;
        mark(CollectedData::Addresses, source);
        // An address another device reported seeing is a client of that device.
        if source == AttributeSource::ForwardingTable {
            mark(CollectedData::ConnectedClients, source);
        }
        if let Some(mac) = source_of(&ip_address.base.mac_address) {
            mark(CollectedData::Addresses, mac);
        }
    }

    for port in ports {
        mark(CollectedData::PortsAndServices, port.base.port_source);
    }

    for subnet in subnets {
        mark(CollectedData::Subnets, subnet.base.cidr.source());
    }

    if let Some(virtualization) = virtualization_metadata
        && let Some((category, source)) =
            guest_collection(HostVirtualizationDiscriminants::from(virtualization))
    {
        mark(category, source);
    }

    for service in services {
        if let Some(virtualization) = &service.base.virtualization_metadata {
            let probe = match virtualization {
                ServiceVirtualization::Docker(_) => ClientProbe::Docker,
                ServiceVirtualization::Podman(_) => ClientProbe::Podman,
            };
            mark(CollectedData::Containers, AttributeSource::Probe(probe));
        }
    }

    for interface in interfaces {
        let InterfaceBase {
            interface_source,
            mac_address,
            neighbor_candidates,
            fdb_macs,
            native_vlan_id,
            vlan_ids,
            vlan_membership_source,
            // The rest of the interface row, counted under the row's own source.
            host_id: _,
            site_id: _,
            if_index: _,
            if_descr: _,
            if_name: _,
            if_alias: _,
            if_type: _,
            speed_bps: _,
            admin_status: _,
            oper_status: _,
            ip_address_id: _,
            ip_configured: _,
        } = &interface.base;
        mark(CollectedData::Interfaces, *interface_source);
        if let Some(mac) = source_of(mac_address) {
            mark(CollectedData::Interfaces, mac);
        }
        for candidate in neighbor_candidates {
            mark(CollectedData::Neighbors, candidate.evidence_source);
        }
        if let Some(fdb) = fdb_macs
            && !fdb.is_blank()
        {
            mark(CollectedData::ForwardingTables, fdb.source());
        }
        if native_vlan_id.is_some() || vlan_ids.as_ref().is_some_and(|v| !v.is_empty()) {
            mark(CollectedData::Vlans, *vlan_membership_source);
        }
    }

    present
}

/// The first pair present in `after` but not in `before` that `producer` does not declare, or
/// `None` when everything it added is declared. Pairs are compared in a fixed order, so the same
/// submission always reports the same pair.
///
/// A pair already present in `before` is never flagged, so a producer that adds more of a pair
/// another producer already wrote goes unchecked for that pair. Checking each integration's
/// mapping on its own, from an empty host, covers that case in tests.
pub fn undeclared_collection(
    producer: Producer,
    before: &HashSet<(CollectedData, AttributeSource)>,
    after: &HashSet<(CollectedData, AttributeSource)>,
) -> Option<(CollectedData, AttributeSource)> {
    let mut added: Vec<(CollectedData, AttributeSource)> =
        after.difference(before).copied().collect();
    added.sort_by_key(|(category, source)| (*category, source.to_string()));
    added
        .into_iter()
        .find(|(category, source)| !producer.declares(*category, *source))
}

/// The first pair a host submitted by `producer` carries that the producer does not declare.
///
/// The host is new, so everything on it counts as collected, except subnets: those only place the
/// addresses and are the site's own, so a submission does not take them.
pub fn undeclared_in_submitted_host(
    producer: impl Into<Producer>,
    host: &Host,
    ip_addresses: &[IPAddress],
    ports: &[Port],
    services: &[Service],
    interfaces: &[Interface],
) -> Option<(CollectedData, AttributeSource)> {
    undeclared_collection(
        producer.into(),
        &HashSet::new(),
        &present_in(host, ip_addresses, ports, services, interfaces, &[]),
    )
}

/// Fail a debug assertion, and warn in a release build, for a pair `producer` collected without
/// declaring it. Release builds keep the data: the gap is in the declaration, and in the security
/// page built from it, not in what was collected.
pub fn report_undeclared_collection(
    producer: Producer,
    undeclared: Option<(CollectedData, AttributeSource)>,
) {
    if let Some((category, source)) = undeclared {
        tracing::warn!(
            producer = producer.id(),
            category = ?category,
            source = %source,
            "A producer submitted data from a source it does not declare collecting"
        );
        debug_assert!(
            false,
            "{producer:?} collected {category:?} from {source}; declare it in its collection()"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{
        credentials::r#impl::types::CredentialIntegration, ports::r#impl::base::PortType,
    };

    fn pairs(
        pairs: &[(CollectedData, AttributeSource)],
    ) -> HashSet<(CollectedData, AttributeSource)> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn a_pair_already_present_is_not_charged_to_the_producer() {
        let producer = Producer::Integration(CredentialIntegration::Gnmi);
        let reverse_dns = (CollectedData::Hostnames, AttributeSource::ReverseDns);
        let before = pairs(&[reverse_dns]);
        let after = pairs(&[
            reverse_dns,
            (
                CollectedData::Interfaces,
                AttributeSource::Probe(ClientProbe::Gnmi),
            ),
        ]);
        assert_eq!(undeclared_collection(producer, &before, &after), None);
    }

    #[test]
    fn a_pair_added_without_a_declaration_is_flagged() {
        let producer = Producer::Integration(CredentialIntegration::Gnmi);
        let snmp_interfaces = (
            CollectedData::Interfaces,
            AttributeSource::Probe(ClientProbe::Snmp),
        );
        let after = pairs(&[snmp_interfaces]);
        assert_eq!(
            undeclared_collection(producer, &HashSet::new(), &after),
            Some(snmp_interfaces)
        );
    }

    /// A current producer must say where every row it writes came from, so an unattributed one is
    /// flagged even in a category the producer collects.
    #[test]
    fn an_unattributed_row_is_flagged_for_every_producer() {
        let unattributed = (CollectedData::Interfaces, AttributeSource::Unspecified);
        for producer in [
            Producer::CredentialFree,
            Producer::Integration(CredentialIntegration::Snmp),
        ] {
            assert_eq!(
                undeclared_collection(producer, &HashSet::new(), &pairs(&[unattributed])),
                Some(unattributed),
                "{producer:?}"
            );
        }
    }

    /// A derived reading rests on data the producer collected, which is checked on its own.
    #[test]
    fn a_derived_source_is_never_charged() {
        let derived = (
            CollectedData::OperatingSystem,
            AttributeSource::ServiceMatch,
        );
        assert_eq!(
            undeclared_collection(
                Producer::CredentialFree,
                &HashSet::new(),
                &pairs(&[derived])
            ),
            None
        );
    }

    /// A credential-free producer cannot claim a reading only an integration makes.
    #[test]
    fn credential_free_discovery_cannot_write_a_credentialed_source() {
        let snmp = (
            CollectedData::DeviceDetails,
            AttributeSource::Probe(ClientProbe::Snmp),
        );
        assert_eq!(
            undeclared_collection(Producer::CredentialFree, &HashSet::new(), &pairs(&[snmp])),
            Some(snmp)
        );
    }

    /// A default host carries no data, so a default field value never charges a pair to every
    /// producer that submits a host.
    #[test]
    fn an_empty_host_carries_nothing() {
        let host = Host::new(HostBase::default());
        assert!(present_in(&host, &[], &[], &[], &[], &[]).is_empty());
    }

    /// A daemon predating row sources sends rows without the keys. They read as unattributed,
    /// which ingest heals on the next attributed report, rather than failing the submission.
    #[test]
    fn a_payload_predating_row_sources_reads_as_unattributed() {
        use crate::server::interface_neighbors::r#impl::base::InterfaceNeighborEvidence;
        use crate::server::interfaces::r#impl::base::{FdbMacs, FdbMacsValue};
        use serde::de::DeserializeOwned;

        fn without<T: serde::Serialize + DeserializeOwned>(value: &T, keys: &[&str]) -> T {
            let mut json = serde_json::to_value(value).unwrap();
            for key in keys {
                json.as_object_mut().unwrap().remove(*key);
            }
            serde_json::from_value(json).unwrap()
        }
        let snmp = AttributeSource::Probe(ClientProbe::Snmp);

        let port = Port::new_hostless(PortType::Ssh).with_source(AttributeSource::PortScan);
        assert_eq!(
            without(&port, &["port_source"]).base.port_source,
            AttributeSource::Unspecified
        );

        let mut ip = IPAddress::default();
        ip.base.ip_address_source = AttributeSource::ArpReply;
        assert_eq!(
            without(&ip, &["ip_address_source"]).base.ip_address_source,
            AttributeSource::Unspecified
        );

        let mut interface = Interface::default();
        interface.base.interface_source = snmp;
        interface.base.vlan_membership_source = snmp;
        interface.base.fdb_macs = Some(FdbMacs::new(
            FdbMacsValue(vec!["00:1a:2b:00:10:00".to_string()]),
            snmp,
        ));
        let old = without(
            &interface,
            &[
                "interface_source",
                "vlan_membership_source",
                "fdb_macs_source",
            ],
        );
        assert_eq!(old.base.interface_source, AttributeSource::Unspecified);
        assert_eq!(
            old.base.vlan_membership_source,
            AttributeSource::Unspecified
        );
        assert_eq!(
            old.base.fdb_macs.map(|fdb| fdb.source()),
            Some(AttributeSource::Unspecified)
        );

        let evidence = InterfaceNeighborEvidence {
            evidence_source: snmp,
            ..Default::default()
        };
        assert_eq!(
            without(&evidence, &["evidence_source"]).evidence_source,
            AttributeSource::Unspecified
        );
    }

    /// A port counts under its own source, so the check sees which method found it.
    #[test]
    fn a_port_counts_under_its_source() {
        let host = Host::new(HostBase::default());
        let port = Port::new_hostless(PortType::Ssh).with_source(AttributeSource::PortScan);
        assert_eq!(
            present_in(&host, &[], &[port], &[], &[], &[]),
            pairs(&[(CollectedData::PortsAndServices, AttributeSource::PortScan)])
        );
    }
}
