use super::*;
use crate::server::hosts::r#impl::attributes::{
    HostChassisIdValue, HostHostnameValue, HostModelValue, HostSysNameValue,
};
use crate::server::hosts::r#impl::name_ladder::HostNameRung;
use crate::server::services::r#impl::patterns::ClientProbe;

fn controller_name(name: &str) -> HostName {
    HostName::from_controller(name.to_string(), ClientProbe::UnifiController)
}

/// A host carrying nothing but the rungs under test, so a fall-through cannot be masked by a
/// leftover value from a fuller fixture.
fn nameless_host() -> Host {
    let mut host = crate::server::shared::types::examples::host();
    host.base.name = HostName::unnamed();
    host.base.hostname = None;
    host
}

fn probed<V>(value: V) -> Attributed<V>
where
    V: crate::server::shared::attribution::AttributeValue,
{
    Attributed::new(value, AttributeSource::Probe(ClientProbe::Snmp))
}

/// The ladder descends only as far as it has to.
///
/// Written as one walk down rather than a case per rung: what matters is the *ordering* between
/// them — that a sysName never displaces a hostname, and an address never displaces either —
/// and an assertion per rung in isolation would pass even if the `or_else` chain were shuffled.
///
/// Each step also asserts the rung. The editor tells a person which piece of evidence named the
/// host, so a rung that disagreed with the value would explain the title wrongly.
#[test]
fn display_name_stops_at_the_highest_rung_the_host_carries() {
    let addresses = [crate::server::shared::types::examples::ip_address()];
    let mut host = nameless_host();
    let titled = |value: &str, rung| Some((value.to_string(), rung));

    // Nothing at all: absence, not `Some("")`. This is what every caller's fallback hangs on —
    // a blank title would be read as a name the host actually has.
    assert_eq!(host.display_name(&addresses[..0]), None);
    assert_eq!(host.resolved_name(&addresses[..0]), None);

    // The bottom rung, reached only because the four above are empty.
    assert_eq!(
        host.resolved_name(&addresses),
        titled("192.168.1.100", HostNameRung::Address)
    );

    host.base.chassis_id = Some(probed(HostChassisIdValue("00:1a:2b:3c:4d:5e".to_string())));
    assert_eq!(
        host.resolved_name(&addresses),
        titled("00:1a:2b:3c:4d:5e", HostNameRung::ChassisId)
    );

    host.base.sys_name = Some(probed(HostSysNameValue("core-sw-01".to_string())));
    assert_eq!(
        host.resolved_name(&addresses),
        titled("core-sw-01", HostNameRung::SysName)
    );

    host.base.hostname = Some(Attributed::new(
        HostHostnameValue("switch.lan".to_string()),
        AttributeSource::ReverseDns,
    ));
    assert_eq!(
        host.resolved_name(&addresses),
        titled("switch.lan", HostNameRung::Hostname)
    );

    host.base.name = HostName::manual("Core Switch".to_string());
    assert_eq!(
        host.resolved_name(&addresses),
        titled("Core Switch", HostNameRung::Name)
    );
    assert_eq!(
        host.display_name(&addresses),
        Some("Core Switch".to_string())
    );

    // A person clearing the name hands the title back to the evidence below it.
    assert!(host.base.clear_name());
    assert_eq!(host.base.name.source(), AttributeSource::Unspecified);
    assert_eq!(
        host.resolved_name(&addresses),
        titled("switch.lan", HostNameRung::Hostname)
    );
}

/// A rung holding whitespace is not a rung.
///
/// SNMP agents and controllers return `" "` and `""` for fields they don't populate, and a
/// host titled with a space is indistinguishable on screen from one titled with nothing —
/// except that it silently outranks the real evidence below it.
#[test]
fn display_name_treats_a_blank_rung_as_absent() {
    let addresses = [crate::server::shared::types::examples::ip_address()];
    let mut host = nameless_host();
    host.base.hostname = Some(Attributed::new(
        HostHostnameValue("   ".to_string()),
        AttributeSource::ReverseDns,
    ));
    host.base.sys_name = Some(probed(HostSysNameValue(String::new())));
    host.base.chassis_id = Some(probed(HostChassisIdValue("  ".to_string())));

    assert_eq!(
        host.resolved_name(&addresses),
        Some(("192.168.1.100".to_string(), HostNameRung::Address))
    );
}

/// `apply_name`'s return value is what `upsert_host` uses to decide whether the host actually
/// changed, and an Updated event (and a topology rebuild) rides on that. A re-sync that
/// reports the same name must be silent, not a no-op write that still looks like a change.
#[test]
fn reapplying_an_unchanged_name_reports_no_change() {
    let mut base = HostBase::default();
    assert!(base.apply_name(controller_name("Core Switch")));
    assert!(!base.apply_name(controller_name("Core Switch")));
    assert!(base.apply_name(controller_name("Core Switch 2")));
}

/// The same value arriving from a *better* source is still a change worth recording: the name
/// reads the same, but the host is now protected from the rungs in between.
#[test]
fn the_same_name_from_a_higher_rung_is_recorded() {
    let mut base = HostBase::default();
    base.apply_name(HostName::from_service("switch.lan".to_string()));
    assert!(base.apply_name(controller_name("switch.lan")));
    assert_eq!(
        base.name.source(),
        AttributeSource::Authored(ClientProbe::UnifiController)
    );
}

/// What a person typed into Scanopy survives every subsequent scan. This used to be the
/// `is_none()` gate — first writer wins, whoever they were — and is now `Manual` outranking
/// everything discovery can produce, which is what makes a refreshable `model` safe.
#[test]
fn a_manually_entered_value_is_never_displaced() {
    let mut existing = HostBase {
        model: Some(Attributed::new(
            HostModelValue("typed-by-a-person".to_string()),
            AttributeSource::Manual,
        )),
        ..Default::default()
    };
    let incoming = HostBase {
        model: Some(Attributed::new(
            HostModelValue("read-over-snmp".to_string()),
            AttributeSource::Probe(ClientProbe::Snmp),
        )),
        serial_number: Some(Attributed::new(
            crate::server::hosts::r#impl::attributes::HostSerialNumberValue(
                "FOC1234X5YZ".to_string(),
            ),
            AttributeSource::Probe(ClientProbe::Snmp),
        )),
        ..Default::default()
    };

    assert!(existing.apply_attributes_from(&incoming));

    assert_eq!(
        attribution::text_of(&existing.model).as_deref(),
        Some("typed-by-a-person")
    );
    assert_eq!(
        attribution::text_of(&existing.serial_number).as_deref(),
        Some("FOC1234X5YZ")
    );
}

/// The behaviour the `is_none()` gate could not express: a value already present is displaced
/// when a better source reads it. Under first-write-wins the model below stayed "Cisco Switch"
/// for the life of the host, whatever SNMP later said.
#[test]
fn a_weak_value_is_displaced_by_a_stronger_source() {
    let mut existing = HostBase {
        model: Some(Attributed::new(
            HostModelValue("Cisco Switch".to_string()),
            AttributeSource::Probe(ClientProbe::UnifiController),
        )),
        ..Default::default()
    };
    let incoming = HostBase {
        model: Some(Attributed::new(
            HostModelValue("WS-C2960X-48FPD-L".to_string()),
            AttributeSource::Probe(ClientProbe::Snmp),
        )),
        ..Default::default()
    };

    assert!(existing.apply_attributes_from(&incoming));
    assert_eq!(
        attribution::text_of(&existing.model).as_deref(),
        Some("WS-C2960X-48FPD-L")
    );
}

/// The ordering this item exists to establish, on the field that prompted it. ENTITY-MIB is
/// Track 2's reader, but its rung is decided here: a device answering SNMP outranks a
/// controller describing a device it manages, so a firmware revision from the MIB displaces
/// one a controller reported rather than losing to whichever probe finished first.
#[test]
fn firmware_from_the_device_displaces_firmware_from_a_controller() {
    use crate::server::hosts::r#impl::attributes::HostFirmwareRevisionValue;

    let mut existing = HostBase {
        firmware_revision: Some(Attributed::new(
            HostFirmwareRevisionValue("6.5.59".to_string()),
            AttributeSource::Probe(ClientProbe::UnifiController),
        )),
        ..Default::default()
    };
    let incoming = HostBase {
        firmware_revision: Some(Attributed::new(
            HostFirmwareRevisionValue("17.03.01".to_string()),
            AttributeSource::Probe(ClientProbe::Snmp),
        )),
        ..Default::default()
    };

    assert!(existing.apply_attributes_from(&incoming));
    assert_eq!(
        attribution::text_of(&existing.firmware_revision).as_deref(),
        Some("17.03.01")
    );
}

/// `upsert_host` publishes an Updated event and triggers a topology rebuild off this return
/// value, so a scan that learns nothing new must report no change.
#[test]
fn learning_nothing_new_reports_no_change() {
    let snmp = AttributeSource::Probe(ClientProbe::Snmp);
    let model = |v: &str| Some(Attributed::new(HostModelValue(v.to_string()), snmp));
    let mut existing = HostBase {
        model: model("WS-C2960X"),
        ..Default::default()
    };
    let incoming = HostBase {
        model: model("WS-C2960X"),
        ..Default::default()
    };

    assert!(!existing.apply_attributes_from(&incoming));
    assert!(!existing.apply_attributes_from(&HostBase::default()));
}

fn proxmox_guest(owner: Uuid, vm_id: Option<&str>, vm_name: &str) -> HostBase {
    use crate::server::hosts::r#impl::virtualization::{
        HostVirtualization, ProxmoxGuestType, ProxmoxVirtualization,
    };
    HostBase {
        virtualization_service_id: Some(owner),
        virtualization_metadata: Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
            vm_name: Some(vm_name.to_string()),
            vm_id: vm_id.map(str::to_string),
            guest_type: vm_id.map(|_| ProxmoxGuestType::Qemu),
        })),
        ..Default::default()
    }
}

/// An integration links a guest it finds already on file, a renamed guest's metadata follows,
/// a migrated guest follows its new node, and a hypervisor someone assigned by hand stays.
#[test]
fn virtualization_fills_follows_migration_and_keeps_hand_assignments() {
    let node_a = Uuid::new_v4();
    let node_b = Uuid::new_v4();

    // A plain scan found the VM first; Proxmox then names its node.
    let mut existing = HostBase::default();
    assert!(existing.fill_virtualization_from(&proxmox_guest(node_a, Some("100"), "gitlab")));
    assert_eq!(existing.virtualization_service_id, Some(node_a));

    // Same node, guest renamed in Proxmox: the metadata refreshes.
    let renamed = proxmox_guest(node_a, Some("100"), "gitlab-2");
    assert!(existing.fill_virtualization_from(&renamed));
    assert_eq!(
        existing.virtualization_metadata,
        renamed.virtualization_metadata
    );
    assert!(!existing.fill_virtualization_from(&renamed));

    // Live-migrated to node B: same VMID, so the link moves with it.
    assert!(existing.fill_virtualization_from(&proxmox_guest(node_b, Some("100"), "gitlab-2")));
    assert_eq!(existing.virtualization_service_id, Some(node_b));

    // A different guest id on another node is a different guest, not a migration.
    assert!(!existing.fill_virtualization_from(&proxmox_guest(node_a, Some("200"), "x")));
    assert_eq!(existing.virtualization_service_id, Some(node_b));

    // A report that names no owner (any non-Proxmox scan) changes nothing.
    assert!(!existing.fill_virtualization_from(&HostBase::default()));

    // Assigned by hand: the UI writes no guest id, so a report naming another node leaves the
    // owner and its metadata alone.
    let mut by_hand = proxmox_guest(node_a, None, "nas");
    let before = by_hand.clone();
    assert!(!by_hand.fill_virtualization_from(&proxmox_guest(node_b, Some("300"), "nas")));
    assert_eq!(
        by_hand.virtualization_service_id,
        before.virtualization_service_id
    );
    assert_eq!(
        by_hand.virtualization_metadata,
        before.virtualization_metadata
    );
}

fn docker_container_host(owner: Uuid, container_id: Option<&str>) -> HostBase {
    use crate::server::hosts::r#impl::virtualization::{
        ContainerHostVirtualization, ContainerNetworkType,
    };
    HostBase {
        virtualization_service_id: Some(owner),
        virtualization_metadata: Some(HostVirtualization::Docker(ContainerHostVirtualization {
            container_name: Some("pihole".to_string()),
            container_id: container_id.map(str::to_string),
            compose_project: None,
            network_type: ContainerNetworkType::MacVlan,
        })),
        ..Default::default()
    }
}

/// A container host follows its container ID to another runtime service, and a container ID
/// that happens to equal a Proxmox VMID is a different guest.
#[test]
fn container_host_follows_its_container_id() {
    let runtime_a = Uuid::new_v4();
    let runtime_b = Uuid::new_v4();

    let mut existing = docker_container_host(runtime_a, Some("100"));
    assert!(existing.fill_virtualization_from(&docker_container_host(runtime_b, Some("100"))));
    assert_eq!(existing.virtualization_service_id, Some(runtime_b));

    assert!(!existing.fill_virtualization_from(&docker_container_host(runtime_a, Some("200"))));
    assert!(!existing.fill_virtualization_from(&proxmox_guest(runtime_a, Some("100"), "x")));
    assert_eq!(existing.virtualization_service_id, Some(runtime_b));
}

/// The stored JSON of a container host reads back as the same host, and a reader that meets a
/// runtime it does not know reads the host as not virtualized instead of rejecting the row.
#[test]
fn virtualization_metadata_round_trips_and_reads_unknown_variants_as_absent() {
    let host = docker_container_host(Uuid::new_v4(), Some("4f1c2a"));
    let json = serde_json::to_value(&host).unwrap();
    let read: HostBase = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(read.virtualization_metadata, host.virtualization_metadata);

    let mut future = json;
    future["virtualization_metadata"] = serde_json::json!({
        "type": "SomeFutureRuntime",
        "details": { "container_id": "4f1c2a" }
    });
    let read: HostBase = serde_json::from_value(future).unwrap();
    assert_eq!(read.virtualization_metadata, None);
    assert_eq!(
        read.virtualization_service_id,
        host.virtualization_service_id
    );
}

fn snmp_asset_tag(tag: &str) -> Option<HostAssetTagAttributed> {
    Some(Attributed::new(
        HostAssetTagValue(tag.to_string()),
        AttributeSource::Probe(ClientProbe::Snmp),
    ))
}

/// The edit modal sends the stored tag back on every save. Resending what a scan read must not
/// restamp it `Manual`, or the next relabel on the device would never land.
#[test]
fn resending_the_stored_asset_tag_keeps_its_source() {
    let mut base = HostBase {
        asset_tag: snmp_asset_tag("IT-00412"),
        ..Default::default()
    };

    assert!(!base.apply_requested_asset_tag(Some(" IT-00412 ".to_string())));
    assert!(!base.apply_requested_asset_tag(None));
    assert_eq!(
        base.asset_tag.as_ref().map(|t| t.source()),
        Some(AttributeSource::Probe(ClientProbe::Snmp))
    );
}

/// A typed tag outranks the device's own, so a later scan reading the old label leaves it.
#[test]
fn a_typed_asset_tag_survives_the_next_scan() {
    let mut base = HostBase {
        asset_tag: snmp_asset_tag("IT-00412"),
        ..Default::default()
    };

    assert!(base.apply_requested_asset_tag(Some("IT-09001".to_string())));
    let rescan = HostBase {
        asset_tag: snmp_asset_tag("IT-00412"),
        ..Default::default()
    };
    base.apply_attributes_from(&rescan);

    assert_eq!(
        attribution::text_of(&base.asset_tag).as_deref(),
        Some("IT-09001")
    );
}

/// A blank field is a person clearing the tag, and clearing hands it back to discovery.
#[test]
fn a_blank_asset_tag_clears_it_and_the_next_scan_refills_it() {
    let mut base = HostBase {
        asset_tag: Some(Attributed::new(
            HostAssetTagValue("IT-09001".to_string()),
            AttributeSource::Manual,
        )),
        ..Default::default()
    };

    assert!(base.apply_requested_asset_tag(Some("  ".to_string())));
    assert_eq!(base.asset_tag, None);

    let rescan = HostBase {
        asset_tag: snmp_asset_tag("IT-00412"),
        ..Default::default()
    };
    assert!(base.apply_attributes_from(&rescan));
    assert_eq!(
        attribution::text_of(&base.asset_tag).as_deref(),
        Some("IT-00412")
    );
}
