//! The host naming ladder, end to end through the real discovery write path.
//!
//! GH #680: a UniFi switch imported under a name the administrator chose, and a rescan that both
//! keeps that name fresh and leaves a hand-typed one alone. The interesting behaviour is not in
//! any single function — it is what survives two consecutive `discover_host` calls with an
//! interleaved user edit, which is exactly what `upsert_host` used to get wrong by having no
//! `name` merge arm at all.

use crate::server::hosts::r#impl::attributes::{HostHostnameAttributed, HostHostnameValue};
use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
use crate::server::shared::attribution::{AttributeSource, Attributed};
use std::net::{IpAddr, Ipv4Addr};

use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::hosts::r#impl::api::{HostResponse, UpdateHostRequest};
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::r#impl::name::{HostName, HostNameSources, host_name_from_parts};
use crate::server::hosts::r#impl::name_ladder::HostNameRung;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};

/// A name a person assigned in a controller, which is what the old `Integration` rung meant. Named
/// once so these tests read the same as they did before the ladder was generalised.
const CONTROLLER: AttributeSource = AttributeSource::Authored(ClientProbe::UnifiController);

/// A name a person assigned in a UniFi controller.
fn controller_name(name: String) -> HostName {
    HostName::from_controller(name, ClientProbe::UnifiController)
}
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
use crate::server::networks::r#impl::{Network, NetworkBase};
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::{Storable, Storage};
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
use crate::server::subnets::r#impl::types::SubnetType;

use super::{organization, test_services};

const LAN_CIDR: &str = "192.168.1.0/24";
const DEVICE_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20));
/// The device's chassis MAC. Present because it is what makes a second submission resolve to the
/// same host: the daemon mints a fresh pending subnet id every scan, so IP+subnet does not match
/// across scans and the MAC is the stable anchor.
const DEVICE_MAC: &str = "aa:bb:cc:00:00:20";

macro_rules! harness {
    ($services:ident, $network_id:ident, $container:ident) => {
        let (storage, $services, $container) = test_services().await;

        let org = organization();
        storage.organizations.create(&org).await.unwrap();

        let network = $services
            .network_service
            .create(
                Network::new(NetworkBase::new(org.id)),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap();
        let $network_id = network.id;
    };
}

/// One controller-reported device, as the daemon submits it: an address on a known subnet and a
/// name carrying the rung it came from.
fn submission(network_id: Uuid, name: HostName, hostname: Option<&str>) -> Submission {
    submission_at(network_id, DEVICE_IP, name, hostname)
}

/// The same, at an explicit address — for the case where a host's DHCP lease moves.
fn submission_at(
    network_id: Uuid,
    device_ip: IpAddr,
    name: HostName,
    hostname: Option<&str>,
) -> Submission {
    let mut host = Host::new(HostBase {
        network_id,
        source: EntitySource::Discovery,
        hostname: hostname.and_then(reverse_dns),
        ..Default::default()
    });
    host.base.apply_name(name);

    let subnet = Subnet::new(SubnetBase {
        name: "lan".to_string(),
        network_id,
        cidr: SubnetCidr::new(
            SubnetCidrValue(LAN_CIDR.parse().unwrap()),
            AttributeSource::DaemonSelfReport,
        ),
        subnet_type: SubnetType::Lan,
        source: EntitySource::Discovery,
        ..Default::default()
    });

    let ip = IPAddress::new(IPAddressBase {
        network_id,
        host_id: host.id,
        subnet_id: subnet.id,
        ip_address: device_ip,
        mac_address: DEVICE_MAC
            .parse()
            .ok()
            .map(|m| MacEvidence::new(MacEvidenceValue(m), AttributeSource::ArpReply)),
        name: None,
        position: 0,
    });

    Submission {
        host,
        ip_address: ip,
        subnet,
    }
}

/// A hostname as a scan's PTR lookup records it.
fn reverse_dns(hostname: &str) -> Option<HostHostnameAttributed> {
    hostname_from(hostname, AttributeSource::ReverseDns)
}

fn hostname_from(hostname: &str, source: AttributeSource) -> Option<HostHostnameAttributed> {
    Some(Attributed::new(
        HostHostnameValue(hostname.to_string()),
        source,
    ))
}

struct Submission {
    host: Host,
    ip_address: IPAddress,
    subnet: Subnet,
}

async fn submit(services: &ServiceFactory, s: Submission) -> HostResponse {
    services
        .host_service
        .discover_host(
            s.host,
            vec![s.ip_address],
            vec![],
            vec![],
            vec![],
            vec![s.subnet],
            true,
            InterfaceDataComplete::default(),
            None,
            AuthenticatedEntity::System,
            None,
        )
        .await
        .expect("a discovery submission must persist")
}

/// Rename the host the way the edit modal does: the whole object, every field present.
async fn save_from_ui(
    services: &ServiceFactory,
    existing: &HostResponse,
    name: &str,
    hidden: bool,
) -> HostResponse {
    services
        .host_service
        .update_from_request(
            UpdateHostRequest {
                id: existing.id,
                name: name.to_string(),
                hostname: existing.hostname.clone(),
                description: existing.description.clone(),
                virtualization_metadata: None,
                virtualization_service_id: None,
                hidden,
                tags: vec![],
                expected_updated_at: None,
                ip_addresses: None,
                ports: None,
                services: None,
                interfaces: None,
                credential_assignments: None,
            },
            AuthenticatedEntity::System,
        )
        .await
        .expect("the update must succeed")
}

/// The reported bug: the controller holds the name, the host displays its DHCP address.
#[tokio::test]
async fn a_controller_name_replaces_an_address_title() {
    harness!(services, network_id, _container);

    let scanned = submit(&services, submission(network_id, HostName::unnamed(), None)).await;
    assert_eq!(
        scanned.display_name.as_deref(),
        Some(DEVICE_IP.to_string().as_str())
    );
    assert_eq!(scanned.display_name_rung, Some(HostNameRung::Address));

    let synced = submit(
        &services,
        submission(network_id, controller_name("Core Switch".to_string()), None),
    )
    .await;

    assert_eq!(synced.id, scanned.id, "the same host, matched on its IP");
    assert_eq!(synced.name, "Core Switch");
    assert_eq!(synced.name_source, CONTROLLER);
}

/// "Changing a device's name in the controller updates the Scanopy host on the next sync."
/// Equal rank has to win for this, which is the one direction a first-write-wins merge cannot go.
#[tokio::test]
async fn a_controller_rename_propagates_on_the_next_sync() {
    harness!(services, network_id, _container);

    submit(
        &services,
        submission(
            network_id,
            controller_name("Floor 1 Switch".to_string()),
            None,
        ),
    )
    .await;

    let renamed = submit(
        &services,
        submission(
            network_id,
            controller_name("Floor 2 Switch".to_string()),
            None,
        ),
    )
    .await;

    assert_eq!(renamed.name, "Floor 2 Switch");
}

/// "A host whose name was set by hand in Scanopy keeps that name across repeated discoveries."
#[tokio::test]
async fn a_hand_typed_name_survives_repeated_discovery() {
    harness!(services, network_id, _container);

    let discovered = submit(
        &services,
        submission(network_id, controller_name("Core Switch".to_string()), None),
    )
    .await;

    let typed = save_from_ui(&services, &discovered, "Rack 3 Top Switch", false).await;
    assert_eq!(typed.name_source, AttributeSource::Manual);

    let resynced = submit(
        &services,
        submission(
            network_id,
            controller_name("Core Switch Renamed Upstream".to_string()),
            Some("switch.lan"),
        ),
    )
    .await;

    assert_eq!(
        resynced.name, "Rack 3 Top Switch",
        "a later sync must not overwrite a name a person typed"
    );
    assert_eq!(resynced.name_source, AttributeSource::Manual);
}

/// The edit modal PUTs every field, so "the user saved the host" cannot be read as "the user
/// named the host" — otherwise toggling `hidden` once would freeze the name for good.
#[tokio::test]
async fn saving_an_unrelated_field_does_not_freeze_a_derived_name() {
    harness!(services, network_id, _container);

    let discovered = submit(
        &services,
        submission(network_id, HostName::from_service("SSH".to_string()), None),
    )
    .await;

    let hidden = save_from_ui(&services, &discovered, &discovered.name, true).await;
    assert!(hidden.hidden);
    assert_eq!(
        hidden.name_source,
        AttributeSource::ServiceMatch,
        "an unchanged name is not a user assertion about the name"
    );

    let synced = submit(
        &services,
        submission(
            network_id,
            controller_name("Meeting Room AP".to_string()),
            None,
        ),
    )
    .await;
    assert_eq!(synced.name, "Meeting Room AP");
}

/// `Manual` means "a person typed this into Scanopy", which nothing on a daemon can know. A
/// payload claiming it is refused, so the claim cannot lock the name against future syncs.
///
/// The refused claim lands at `Unspecified` rather than at the rung below `Manual`. The old
/// naming ladder demoted by rank, which meant picking the next rung down and asserting it; a
/// payload that lied about its provenance has told us nothing believable about where the name came
/// from, and `Unspecified` is what that means. The name itself survives either way, and — as the
/// second half of this test shows — the next real sync can still rename the host, which is the
/// behaviour the guard exists for.
#[tokio::test]
async fn a_daemon_cannot_claim_a_name_was_typed_by_a_person() {
    harness!(services, network_id, _container);

    let mut forged = submission(network_id, controller_name("Impostor".to_string()), None);
    forged.host.base.name = HostName::manual("Impostor".to_string());

    let created = submit(&services, forged).await;
    assert_eq!(created.name, "Impostor", "the name itself is kept");
    assert_eq!(created.name_source, AttributeSource::Unspecified);

    let resynced = submit(
        &services,
        submission(network_id, controller_name("Real Name".to_string()), None),
    )
    .await;
    assert_eq!(
        resynced.name, "Real Name",
        "a forged Manual claim must not make a name permanent"
    );
}

/// A controller's name still titles a host whose scan reported a hostname: a vouched name ranks
/// above every identifier. The hostname is kept in its own column, with its own source.
#[tokio::test]
async fn a_hostname_does_not_displace_a_controller_name() {
    harness!(services, network_id, _container);

    submit(
        &services,
        submission(
            network_id,
            controller_name("Meeting Room AP".to_string()),
            None,
        ),
    )
    .await;

    let rescanned = submit(
        &services,
        submission(network_id, HostName::unnamed(), Some("unifi-a1b2c3.lan")),
    )
    .await;

    assert_eq!(rescanned.name, "Meeting Room AP");
    assert_eq!(rescanned.display_name.as_deref(), Some("Meeting Room AP"));
    assert_eq!(rescanned.hostname.as_deref(), Some("unifi-a1b2c3.lan"));
    assert_eq!(rescanned.hostname_source, AttributeSource::ReverseDns);
}

/// Daemons up to v0.17.14 repeat the hostname as the name (`ReverseDns`) and send the address as
/// an `OwnAddress` name. Neither copy is stored: the identifier is already in its own column, and
/// the display ladder titles the host by it.
#[tokio::test]
async fn an_old_daemons_identifier_copies_are_not_stored_as_names() {
    harness!(services, network_id, _container);

    let by_address = submit(
        &services,
        submission(
            network_id,
            host_name_from_parts(DEVICE_IP.to_string(), AttributeSource::OwnAddress),
            None,
        ),
    )
    .await;
    assert_eq!(by_address.name, "");
    assert_eq!(by_address.display_name_rung, Some(HostNameRung::Address));

    let by_hostname = submit(
        &services,
        submission(
            network_id,
            host_name_from_parts("nas.lan".to_string(), AttributeSource::ReverseDns),
            Some("nas.lan"),
        ),
    )
    .await;
    assert_eq!(by_hostname.id, by_address.id);
    assert_eq!(by_hostname.name, "");
    assert_eq!(by_hostname.display_name.as_deref(), Some("nas.lan"));
    assert_eq!(by_hostname.display_name_rung, Some(HostNameRung::Hostname));
}

/// The OS hostname the daemon reads on its own host outranks a PTR record a DNS server holds for
/// its address, and a later PTR reading never takes the column back.
#[tokio::test]
async fn the_daemons_own_hostname_outranks_reverse_dns() {
    harness!(services, network_id, _container);

    submit(
        &services,
        submission(network_id, HostName::unnamed(), Some("nas.lan")),
    )
    .await;

    let mut self_report = submission(network_id, HostName::unnamed(), None);
    self_report.host.base.hostname = hostname_from("nas", AttributeSource::DaemonSelfReport);
    let reported = submit(&services, self_report).await;
    assert_eq!(reported.hostname.as_deref(), Some("nas"));
    assert_eq!(reported.hostname_source, AttributeSource::DaemonSelfReport);

    let rescanned = submit(
        &services,
        submission(network_id, HostName::unnamed(), Some("nas.lan")),
    )
    .await;
    assert_eq!(rescanned.hostname.as_deref(), Some("nas"));
    assert_eq!(rescanned.hostname_source, AttributeSource::DaemonSelfReport);
}

/// A daemon's provisioning name is a placeholder: it titles the host until the daemon reports its
/// own hostname, which then titles the host instead.
#[tokio::test]
async fn a_provisioning_placeholder_yields_to_the_self_reported_hostname() {
    harness!(services, network_id, _container);

    let provisioned = submit(
        &services,
        submission(
            network_id,
            HostName::unattributed("office-daemon".to_string()),
            None,
        ),
    )
    .await;
    assert_eq!(provisioned.display_name.as_deref(), Some("office-daemon"));

    let mut self_report = submission(network_id, HostName::unnamed(), None);
    self_report.host.base.hostname = hostname_from("nas", AttributeSource::DaemonSelfReport);
    let reported = submit(&services, self_report).await;
    assert_eq!(reported.display_name.as_deref(), Some("nas"));
    assert_eq!(reported.display_name_rung, Some(HostNameRung::Hostname));
}

/// A host matched by its MAC after moving to another subnet is titled by its new address. Nothing
/// about the address is stored in `name`: reconciliation re-homes the address row, and the ladder
/// reads the row.
///
/// A lease that changes *within* one subnet is left alone by reconciliation, which cannot tell it
/// from a second address on the same MAC (`interface_moved` in `hosts/service/create.rs`), so such
/// a host keeps its stored address and is titled by it.
#[tokio::test]
async fn a_host_that_moves_subnet_is_titled_by_its_new_address() {
    harness!(services, network_id, _container);

    let first = submit(
        &services,
        submission_at(network_id, DEVICE_IP, HostName::unnamed(), None),
    )
    .await;
    assert_eq!(first.display_name.as_deref(), Some("192.168.1.20"));

    let moved_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 2, 21));
    let mut relocated = submission_at(network_id, moved_ip, HostName::unnamed(), None);
    relocated.subnet.base.cidr = SubnetCidr::new(
        SubnetCidrValue("192.168.2.0/24".parse().unwrap()),
        AttributeSource::DaemonSelfReport,
    );
    let moved = submit(&services, relocated).await;

    assert_eq!(moved.id, first.id, "the same host, matched on its MAC");
    assert_eq!(
        moved.display_name.as_deref(),
        Some("192.168.2.21"),
        "addresses (ip, last_seen_at, position): {:?}",
        moved
            .ip_addresses
            .iter()
            .map(|ip| (ip.base.ip_address, ip.last_seen_at, ip.base.position))
            .collect::<Vec<_>>()
    );
}

/// Clearing the name in the editor hands naming back to discovery: the host is titled by the next
/// rung at once, and the next sync can name it again. The applier reads a blank candidate as "no
/// name to offer", so without an explicit clear the typed name used to survive the save.
#[tokio::test]
async fn clearing_a_typed_name_hands_naming_back_to_discovery() {
    harness!(services, network_id, _container);

    let discovered = submit(
        &services,
        submission(network_id, HostName::unnamed(), Some("switch.lan")),
    )
    .await;
    let typed = save_from_ui(&services, &discovered, "Rack 3 Top Switch", false).await;
    assert_eq!(typed.name_source, AttributeSource::Manual);

    let cleared = save_from_ui(&services, &typed, "", false).await;
    assert_eq!(cleared.name, "");
    assert_eq!(cleared.name_source, AttributeSource::Unspecified);
    assert_eq!(cleared.display_name.as_deref(), Some("switch.lan"));
    assert_eq!(cleared.display_name_rung, Some(HostNameRung::Hostname));
    assert_eq!(cleared.hostname_source, AttributeSource::ReverseDns);

    let resynced = submit(
        &services,
        submission(
            network_id,
            controller_name("Core Switch".to_string()),
            Some("switch.lan"),
        ),
    )
    .await;
    assert_eq!(
        resynced.name, "Core Switch",
        "with the typed name gone, discovery names the host again"
    );
}
