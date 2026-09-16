//! What a controller knows about a device or client it manages.
//!
//! UniFi and HPE Instant On both learn an administrator's deliberate name for every device they
//! adopt and every client they see — the name that is stable when the management IP is a DHCP
//! lease. Before this type each integration mapped that name into `sys_name` and left the host's
//! display `name` empty, so the topology was labelled with an IP (GH #680).
//!
//! This is the one place a controller integration expresses identity. [`ControllerIdentity`] has
//! no `Default` impl on purpose: a new integration cannot `..Default::default()` past the name
//! field, so "we forgot to supply the name" is a compile error rather than a blank label. And
//! because both entry points below route the name through
//! [`HostBase::apply_name`](crate::server::hosts::r#impl::base::HostBase::apply_name), no
//! integration ever writes precedence logic of its own.

use std::net::IpAddr;

use uuid::Uuid;

use crate::daemon::discovery::integration::IntegrationContext;
use crate::daemon::discovery::service::ops::HostData;
use crate::server::hosts::r#impl::attributes::{
    HostChassisIdValue, HostFirmwareRevisionValue, HostHostnameValue, HostManufacturerValue,
    HostModelValue, HostSerialNumberValue, HostSysNameValue,
};
use crate::server::hosts::r#impl::{
    base::{Host, HostBase},
    name::{HostName, HostNameSources},
};
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ip_addresses::r#impl::base::{
    IPAddress, IPAddressBase, MacEvidence, MacEvidenceValue,
};
use crate::server::lldp::canonical_mac;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeSource, Attributed};
use crate::server::shared::types::entities::EntitySource;

/// The identity fields a controller reports for one device or client.
///
/// Every field is `Option` because controllers differ in what they hold, but every field must be
/// written at the construction site — that is what makes the omission visible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerIdentity {
    /// Which controller reported this, so every value it carries can name its own source.
    ///
    /// Without it `enrich` would have to pick one source for all of them, and the two controllers
    /// would be indistinguishable — which is exactly the case the equal-rung rule has to resolve
    /// when a device is adopted by both.
    pub probe: ClientProbe,
    /// The name a person assigned in the controller ("Core Switch", "Meeting Room AP").
    /// `None` only when the controller genuinely holds none.
    pub name: Option<String>,
    /// A hostname the controller observed rather than one a person chose — a DHCP client's
    /// advertised hostname, typically. Ranks below `name`.
    pub hostname: Option<String>,
    /// LLDP chassis ID, canonicalised the same way the SNMP daemon writes it.
    pub chassis_id: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    /// The firmware version the controller reports for the device.
    ///
    /// Controllers hold this as a structured field and it used to be flattened into `sys_descr`
    /// as "UniFi firmware 6.5.59", which is neither a system description nor comparable. It has
    /// its own column now.
    pub firmware_revision: Option<String>,
}

impl ControllerIdentity {
    /// Mint a host for a device or client the controller reports but the sweep never scanned.
    ///
    /// The server deduplicates on IP and MAC, so this merges into the host another discovery
    /// path already found when there is one.
    pub fn into_host(self, network_id: Uuid) -> Host {
        let Self {
            probe,
            name,
            hostname,
            chassis_id,
            manufacturer,
            model,
            serial_number,
            firmware_revision,
        } = self.normalized();

        // Everything the controller reports about a device it manages is `Reported`: a known
        // speaker that is not the subject. The one exception is the name, which a person typed
        // into the controller — `apply_names` records that separately.
        let reported = AttributeSource::Probe(probe);
        let mut host = Host::new(HostBase {
            network_id,
            source: EntitySource::Discovery,
            // The controller's name is also what the device advertises as LLDP sysName, and
            // neighbour resolution matches `interfaces.lldp_sys_name` against this column.
            sys_name: name
                .clone()
                .map(|v| Attributed::new(HostSysNameValue(v), reported)),
            // A client's DHCP hostname is an identifier: its own column, never the name.
            hostname: hostname.map(|v| Attributed::new(HostHostnameValue(v), reported)),
            chassis_id: chassis_id.map(|v| Attributed::new(HostChassisIdValue(v), reported)),
            manufacturer: manufacturer.map(|v| Attributed::new(HostManufacturerValue(v), reported)),
            model: model.map(|v| Attributed::new(HostModelValue(v), reported)),
            serial_number: serial_number
                .map(|v| Attributed::new(HostSerialNumberValue(v), reported)),
            firmware_revision: firmware_revision
                .map(|v| Attributed::new(HostFirmwareRevisionValue(v), reported)),
            ..Default::default()
        });

        if let Some(name) = name {
            host.base.apply_name(HostName::from_controller(name, probe));
        }
        host
    }

    /// Fold this identity into the host currently being scanned.
    ///
    /// Every value carries the controller as its source, so a prior SNMP pass in the same scan
    /// keeps its readings — SNMP asks the device directly, which outranks a controller describing
    /// a device it manages, and that ordering is now stated rather than resting on which
    /// integration happened to run first. The name is the exception: a person typed it into the
    /// controller, so it outranks anything the scan itself could derive and loses only to a name
    /// typed into Scanopy.
    pub fn enrich(&self, host_data: &mut HostData) {
        let Self {
            probe,
            name,
            hostname,
            chassis_id,
            manufacturer,
            model,
            serial_number,
            firmware_revision,
        } = self.clone().normalized();

        let reported = AttributeSource::Probe(probe);

        if let Some(hostname) = hostname {
            host_data.with_hostname(hostname, reported);
        }
        if let Some(name) = name {
            host_data.with_sys_name(name.clone(), reported);
            host_data.apply_name(HostName::from_controller(name, probe));
        }
        if let Some(chassis_id) = chassis_id {
            host_data.with_chassis_id(chassis_id, reported);
        }
        if let Some(manufacturer) = manufacturer {
            host_data.with_manufacturer(manufacturer, reported);
        }
        if let Some(model) = model {
            host_data.with_model(model, reported);
        }
        if let Some(serial_number) = serial_number {
            host_data.with_serial_number(serial_number, reported);
        }
        if let Some(firmware_revision) = firmware_revision {
            host_data.with_firmware_revision(firmware_revision, reported);
        }
    }

    /// Controllers routinely return `""` for a field a user never filled in. An empty string is
    /// absence, not a value, and must not displace something real.
    fn normalized(self) -> Self {
        fn blank_to_none(v: Option<String>) -> Option<String> {
            v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        }
        Self {
            probe: self.probe,
            name: blank_to_none(self.name),
            hostname: blank_to_none(self.hostname),
            chassis_id: blank_to_none(self.chassis_id),
            manufacturer: blank_to_none(self.manufacturer),
            model: blank_to_none(self.model),
            serial_number: blank_to_none(self.serial_number),
            firmware_revision: blank_to_none(self.firmware_revision),
        }
    }
}

/// A client a controller reports — a device on the network it can see but has not adopted.
///
/// Clients are mapped separately from adopted devices because that is all a controller knows
/// about them: an address, a MAC, and whatever name or DHCP hostname it has. They still become
/// hosts. The server deduplicates on IP and MAC, so a client the sweep also found is the same
/// host with a better name, and one it could not reach (a different VLAN, no ARP entry) becomes
/// a host the controller is the only witness for.
pub struct MappedClient {
    pub identity: ControllerIdentity,
    pub ip_address: IPAddress,
    pub ip: IpAddr,
}

impl MappedClient {
    /// A reported client, with its address left for the server to place.
    ///
    /// `None` only when the address is missing or unparseable — there is nothing to report about a
    /// client whose address we cannot read. It used to also return `None` for an address outside
    /// every known subnet, on the grounds that IP-based dedup would mint a duplicate every scan;
    /// that reasoning was sound and the remedy was not, because the subnet list it consulted
    /// carries `0.0.0.0/0` catch-alls that contain every IPv4 address, so nothing was ever skipped
    /// and everything landed on `Internet` instead.
    ///
    /// `subnet_id` is left nil the way `host_id` already is: the server places the address against
    /// the network's authoritative subnet list and infers a range where nothing holds it, which is
    /// the only place that decision can be made consistently across integrations.
    pub fn new(
        identity: ControllerIdentity,
        ip: Option<&str>,
        mac: Option<&str>,
        network_id: Uuid,
    ) -> Option<Self> {
        let ip: IpAddr = ip?.trim().parse().ok()?;
        let mac = mac.and_then(canonical_mac);
        let probe = identity.probe;

        Some(Self {
            identity,
            ip_address: IPAddress::new(IPAddressBase {
                network_id,
                host_id: Uuid::nil(),   // server assigns
                subnet_id: Uuid::nil(), // server places
                ip_address: ip,
                // The controller reporting a device it manages: a known speaker that is not the
                // subject, so weaker than an ARP reply the address itself sent us.
                mac_address: mac
                    .as_deref()
                    .and_then(|m| m.parse().ok())
                    .map(|m| MacEvidence::new(MacEvidenceValue(m), AttributeSource::Probe(probe))),
                name: None,
                position: 0,
            }),
            ip,
        })
    }
}

/// Submit each reported client as a host, and report how many were created.
///
/// Shared rather than per-integration so that a controller integration only has to answer "what
/// does the controller call this thing, and where is it?" — everything downstream of that,
/// including the naming ladder, is decided here.
pub async fn create_client_hosts(
    ctx: &IntegrationContext<'_>,
    clients: Vec<MappedClient>,
) -> usize {
    let Ok(network_id) = ctx.ops.network_id().await else {
        return 0;
    };

    let mut created = 0usize;
    for client in clients {
        if ctx.cancel.is_cancelled() {
            break;
        }
        let MappedClient {
            identity,
            ip_address,
            ip: _,
        } = client;

        let result = ctx
            .ops
            .create_host(
                identity.into_host(network_id),
                vec![ip_address],
                vec![],
                vec![],
                vec![],
                vec![],
                // A controller sees a client's address and name, never its interfaces. Claiming
                // an authoritative empty ifTable here would tear down interfaces an SNMP walk of
                // the same host collected.
                false,
                InterfaceDataComplete::default(),
                ctx.cancel,
            )
            .await;

        match result {
            Ok(_) => created += 1,
            Err(e) => tracing::debug!(error = %e, "Failed to create controller-reported client"),
        }
    }
    created
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(name: Option<&str>, hostname: Option<&str>) -> ControllerIdentity {
        ControllerIdentity {
            probe: ClientProbe::UnifiController,
            name: name.map(str::to_string),
            hostname: hostname.map(str::to_string),
            chassis_id: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            firmware_revision: None,
        }
    }

    #[test]
    fn controller_name_becomes_the_display_name() {
        let host = identity(Some("Core Switch"), None).into_host(Uuid::new_v4());
        assert_eq!(host.base.name.value().as_str(), "Core Switch");
        assert_eq!(
            host.base.name.source(),
            AttributeSource::Authored(ClientProbe::UnifiController)
        );
        // Still mirrored into sys_name, which is what LLDP neighbour resolution matches on.
        assert_eq!(
            crate::server::shared::attribution::text_of(&host.base.sys_name).as_deref(),
            Some("Core Switch")
        );
    }

    /// A client's DHCP hostname is an identifier, so it lands in `hostname` with the controller as
    /// its source and the name stays empty. The display ladder titles the client by it.
    #[test]
    fn a_clients_dhcp_hostname_is_stored_as_its_hostname_not_its_name() {
        let host = identity(None, Some("marys-laptop")).into_host(Uuid::new_v4());
        assert_eq!(host.base.name, HostName::unnamed());
        let hostname = host.base.hostname.as_ref().expect("the hostname is kept");
        assert_eq!(hostname.value().0, "marys-laptop");
        assert_eq!(
            hostname.source(),
            AttributeSource::Probe(ClientProbe::UnifiController)
        );
    }

    #[test]
    fn an_assigned_name_outranks_a_hostname_for_the_same_device() {
        let host = identity(Some("Reception iPad"), Some("ipad-1a2b")).into_host(Uuid::new_v4());
        assert_eq!(host.base.name.value().as_str(), "Reception iPad");
        assert_eq!(
            host.base.name.source(),
            AttributeSource::Authored(ClientProbe::UnifiController)
        );
        assert_eq!(
            crate::server::shared::attribution::text_of(&host.base.hostname).as_deref(),
            Some("ipad-1a2b")
        );
    }

    #[test]
    fn a_blank_controller_name_leaves_the_host_unnamed_rather_than_naming_it_empty() {
        let host = identity(Some("   "), None).into_host(Uuid::new_v4());
        // `Unnamed`, not an empty `Unspecified`: "no name" and "a name we cannot attribute" are
        // different states, and only the latter should outrank anything.
        assert_eq!(host.base.name, HostName::unnamed());
        assert_eq!(host.base.sys_name, None);
    }
}
