//! A port changes hands only after a scan that covered it at that address.
//!
//! Something else answering hands the port over at once. Nothing answering releases it once the
//! port has gone unseen past the site's stale window. A holder left with no bindings is deleted.
//! What must stay put lives in `discovery_stale_port_guards`, which shares this file's `Lab`.

use chrono::Duration;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::bindings::r#impl::base::Binding;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::ports::r#impl::base::{Port, TransportProtocol};
use crate::server::ports::r#impl::coverage::PortCoverage;
use crate::server::services::r#impl::base::Service;
use crate::server::services::r#impl::patterns::MatchDetails;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Storage;
use crate::server::shared::types::entities::EntitySource;

use super::discovery_stale_port_guards::{LAN_IP, Lab, SECOND_LAN_IP, at, at_lan};
use super::test_services;

/// TCP coverage of `numbers` at `ip`.
pub(super) fn covering(ip: &str, numbers: &[u16]) -> PortCoverage {
    let mut coverage = PortCoverage::default();
    coverage.add(
        ip.parse().unwrap(),
        TransportProtocol::Tcp,
        numbers.iter().copied(),
    );
    coverage
}

impl Lab<'_> {
    /// A scan that covered `coverage`, as a current daemon's network scan reports it.
    pub(super) async fn scan_covering(
        &self,
        ip_addresses: Vec<IPAddress>,
        ports: Vec<Port>,
        services: Vec<Service>,
        coverage: PortCoverage,
    ) {
        let mut host = Host::new(HostBase {
            site_id: self.site_id,
            source: EntitySource::Discovery,
            ..Default::default()
        });
        host.id = self.host_id;
        self.services
            .host_service
            .discover_host(
                host,
                ip_addresses,
                ports,
                services,
                vec![],
                vec![],
                false,
                InterfaceDataComplete::none(),
                &coverage,
                None,
                AuthenticatedEntity::System,
                None,
            )
            .await
            .expect("the scan persists");
    }

    /// Mark the stored port as last answering well past the site's stale window.
    pub(super) async fn age_port(&self, number: u16) {
        let mut port = self.stored_tcp(number).await;
        port.last_seen_at = crate::server::shared::clock::now() - Duration::days(30);
        self.services
            .port_service
            .storage()
            .update_many(&[port])
            .await
            .unwrap();
    }
}

/// Software replacement: Caddy answers where Traefik was stored, on a scan that covered 443.
/// Caddy takes the port, and Traefik, left with nothing, is deleted.
#[tokio::test]
async fn a_covered_replacement_takes_the_port_and_the_emptied_holder_goes() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p443) = (lab.lan_ip(), lab.tcp(443));
    let traefik = lab.service("Traefik", vec![(&p443, &lan)]);
    lab.scan(vec![lan], vec![p443], vec![traefik]).await;

    let (lan, p443) = (lab.lan_ip(), lab.tcp(443));
    let caddy = lab.service("Caddy", vec![(&p443, &lan)]);
    lab.scan_covering(vec![lan], vec![p443], vec![caddy], covering(LAN_IP, &[443]))
        .await;

    assert_eq!(lab.ports_named("Caddy").await, Some(vec![at_lan(443)]));
    assert_eq!(lab.named("Traefik").await, None);
}

/// A holder that keeps another port survives the handover with that port.
#[tokio::test]
async fn a_handover_leaves_the_holders_other_ports() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p80, p443) = (lab.lan_ip(), lab.tcp(80), lab.tcp(443));
    let traefik = lab.service("Traefik", vec![(&p80, &lan), (&p443, &lan)]);
    lab.scan(vec![lan], vec![p80, p443], vec![traefik]).await;

    let (lan, p443) = (lab.lan_ip(), lab.tcp(443));
    let caddy = lab.service("Caddy", vec![(&p443, &lan)]);
    lab.scan_covering(vec![lan], vec![p443], vec![caddy], covering(LAN_IP, &[443]))
        .await;

    assert_eq!(lab.ports_named("Caddy").await, Some(vec![at_lan(443)]));
    assert_eq!(lab.ports_named("Traefik").await, Some(vec![at_lan(80)]));
}

/// Container swap: the runtime reports a different container on the published port. The new
/// container takes it, and the old one keeps its bridge address.
#[tokio::test]
async fn a_swapped_container_takes_the_published_port() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, bridge, p3001) = (lab.lan_ip(), lab.bridge_ip("172.17.0.5"), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let kuma = lab.container_as(
        "UptimeKuma",
        "uptime-kuma",
        &runtime,
        vec![(&p3001, &lan), (&p3001, &bridge)],
    );
    lab.scan(vec![lan, bridge], vec![p3001], vec![runtime, kuma])
        .await;

    let (lan, bridge, p3001) = (lab.lan_ip(), lab.bridge_ip("172.17.0.6"), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let gitea = lab.container_as(
        "Gitea",
        "gitea",
        &runtime,
        vec![(&p3001, &lan), (&p3001, &bridge)],
    );
    lab.scan_covering(
        vec![lan, bridge],
        vec![p3001],
        vec![runtime, gitea],
        covering(LAN_IP, &[3001]),
    )
    .await;

    assert_eq!(
        lab.ports_named("gitea").await,
        Some(vec![at(3001, "172.17.0.6"), at_lan(3001)])
    );
    assert_eq!(
        lab.ports_named("uptime-kuma").await,
        Some(vec![at(3001, "172.17.0.5")])
    );
}

/// A container stored as a generic placeholder, now identified: the runtime reports the same
/// container id, so it is one container. The identified service takes every binding the
/// placeholder held, including ones this scan didn't report or cover, and the placeholder goes.
#[tokio::test]
async fn an_identified_container_takes_everything_its_placeholder_held() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, bridge, p3001) = (lab.lan_ip(), lab.bridge_ip("172.17.0.5"), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let placeholder = lab.container_as(
        "Docker Container",
        "kuma",
        &runtime,
        vec![(&p3001, &lan), (&p3001, &bridge)],
    );
    lab.scan(vec![lan, bridge], vec![p3001], vec![runtime, placeholder])
        .await;

    let (lan, bridge, p3001) = (lab.lan_ip(), lab.bridge_ip("172.17.0.5"), lab.tcp(3001));
    let runtime = lab.runtime(&lan);
    let mut kuma = lab.container_as("UptimeKuma", "kuma", &runtime, vec![(&p3001, &lan)]);
    kuma.base.name = "Uptime Kuma".to_string();
    lab.scan(vec![lan, bridge], vec![p3001], vec![runtime, kuma])
        .await;

    assert_eq!(lab.named("kuma").await, None);
    assert_eq!(
        lab.ports_named("Uptime Kuma").await,
        Some(vec![at(3001, "172.17.0.5"), at_lan(3001)])
    );
}

/// A generic port-only match is weaker evidence than a stored identification and takes nothing,
/// covered or not.
#[tokio::test]
async fn a_covered_generic_match_takes_nothing() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p5432) = (lab.lan_ip(), lab.tcp(5432));
    let grafana = lab.service("Grafana", vec![(&p5432, &lan)]);
    lab.scan(vec![lan], vec![p5432], vec![grafana]).await;

    let (lan, p5432) = (lab.lan_ip(), lab.tcp(5432));
    let postgres = lab.service("PostgreSQL", vec![(&p5432, &lan)]);
    lab.scan_covering(
        vec![lan],
        vec![p5432],
        vec![postgres],
        covering(LAN_IP, &[5432]),
    )
    .await;

    assert_eq!(lab.ports_named("Grafana").await, Some(vec![at_lan(5432)]));
    assert_eq!(lab.named("PostgreSQL").await, None);
}

/// Both answering in one covered scan: nothing moves.
#[tokio::test]
async fn a_covered_scan_where_both_answer_moves_nothing() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p443) = (lab.lan_ip(), lab.tcp(443));
    let traefik = lab.service("Traefik", vec![(&p443, &lan)]);
    lab.scan(vec![lan], vec![p443], vec![traefik]).await;

    let (lan, p443) = (lab.lan_ip(), lab.tcp(443));
    let caddy = lab.service("Caddy", vec![(&p443, &lan)]);
    let traefik = lab.service("Traefik", vec![(&p443, &lan)]);
    lab.scan_covering(
        vec![lan],
        vec![p443],
        vec![caddy, traefik],
        covering(LAN_IP, &[443]),
    )
    .await;

    assert_eq!(lab.ports_named("Traefik").await, Some(vec![at_lan(443)]));
    assert_eq!(lab.named("Caddy").await, None);
}

/// Nothing answering on a covered port releases it only once the port is stale. A fresh port
/// (a restart during the scan) stays.
#[tokio::test]
async fn nothing_answering_releases_a_stale_port_and_keeps_a_fresh_one() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p443, p8443, p9443) = (lab.lan_ip(), lab.tcp(443), lab.tcp(8443), lab.tcp(9443));
    let traefik = lab.service(
        "Traefik",
        vec![(&p443, &lan), (&p8443, &lan), (&p9443, &lan)],
    );
    lab.scan(vec![lan], vec![p443, p8443, p9443], vec![traefik])
        .await;
    lab.age_port(8443).await;

    let (lan, p443) = (lab.lan_ip(), lab.tcp(443));
    let traefik = lab.service("Traefik", vec![(&p443, &lan)]);
    lab.scan_covering(
        vec![lan],
        vec![p443],
        vec![traefik],
        covering(LAN_IP, &[443, 8443, 9443]),
    )
    .await;

    assert_eq!(
        lab.ports_named("Traefik").await,
        Some(vec![at_lan(443), at_lan(9443)])
    );
}

/// A holder whose only port went stale and silent is deleted once the port is released.
#[tokio::test]
async fn a_holder_released_of_its_only_port_is_deleted() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p22, p443) = (lab.lan_ip(), lab.tcp(22), lab.tcp(443));
    let traefik = lab.service("Traefik", vec![(&p443, &lan)]);
    let open_ports = lab.service("Unclaimed Open Ports", vec![(&p22, &lan)]);
    lab.scan(vec![lan], vec![p22, p443], vec![traefik, open_ports])
        .await;
    lab.age_port(443).await;

    let (lan, p22) = (lab.lan_ip(), lab.tcp(22));
    let open_ports = lab.service("Unclaimed Open Ports", vec![(&p22, &lan)]);
    lab.scan_covering(
        vec![lan],
        vec![p22],
        vec![open_ports],
        covering(LAN_IP, &[22, 443]),
    )
    .await;

    assert_eq!(lab.named("Traefik").await, None);
    assert_eq!(
        lab.ports_named("Unclaimed Open Ports").await,
        Some(vec![at_lan(22)])
    );
}

/// Unclaimed Open Ports gives up a stale port a covered scan found closed, and goes when empty.
#[tokio::test]
async fn open_ports_gives_up_a_stale_closed_port() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p8080) = (lab.lan_ip(), lab.tcp(8080));
    let open_ports = lab.service("Unclaimed Open Ports", vec![(&p8080, &lan)]);
    lab.scan(vec![lan], vec![p8080], vec![open_ports]).await;
    lab.age_port(8080).await;

    let lan = lab.lan_ip();
    lab.scan_covering(vec![lan], vec![], vec![], covering(LAN_IP, &[8080]))
        .await;

    assert_eq!(lab.named("Unclaimed Open Ports").await, None);
}

/// A stale port the scan did not cover (a light scan without it, a Docker-only submission)
/// stays with its holder.
#[tokio::test]
async fn a_stale_port_the_scan_did_not_cover_stays() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p443, p8443) = (lab.lan_ip(), lab.tcp(443), lab.tcp(8443));
    let traefik = lab.service("Traefik", vec![(&p443, &lan), (&p8443, &lan)]);
    lab.scan(vec![lan], vec![p443, p8443], vec![traefik]).await;
    lab.age_port(8443).await;

    let (lan, p443) = (lab.lan_ip(), lab.tcp(443));
    let traefik = lab.service("Traefik", vec![(&p443, &lan)]);
    lab.scan_covering(
        vec![lan],
        vec![p443],
        vec![traefik],
        covering(LAN_IP, &[443]),
    )
    .await;

    assert_eq!(
        lab.ports_named("Traefik").await,
        Some(vec![at_lan(443), at_lan(8443)])
    );
}

/// A manually created service keeps its port when something else answers on a covered scan,
/// and when nothing does.
#[tokio::test]
async fn a_manual_holder_keeps_its_port_on_a_covered_scan() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p443, p8443) = (lab.lan_ip(), lab.tcp(443), lab.tcp(8443));
    lab.scan(vec![lan], vec![p443, p8443], vec![]).await;
    lab.create_manual("Traefik", 443, LAN_IP).await;
    lab.create_manual("Nginx Proxy Manager", 8443, LAN_IP).await;
    lab.age_port(8443).await;

    let (lan, p443) = (lab.lan_ip(), lab.tcp(443));
    let caddy = lab.service("Caddy", vec![(&p443, &lan)]);
    lab.scan_covering(
        vec![lan],
        vec![p443],
        vec![caddy],
        covering(LAN_IP, &[443, 8443]),
    )
    .await;

    assert_eq!(lab.ports_named("Traefik").await, Some(vec![at_lan(443)]));
    assert_eq!(
        lab.ports_named("Nginx Proxy Manager").await,
        Some(vec![at_lan(8443)])
    );
    assert_eq!(lab.named("Caddy").await, None);
}

/// A manually created service that discovery later matches stays manual, so it is never
/// released.
#[tokio::test]
async fn a_matched_manual_service_keeps_its_source() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, p443) = (lab.lan_ip(), lab.tcp(443));
    lab.scan(vec![lan], vec![p443], vec![]).await;
    lab.create_manual("Traefik", 443, LAN_IP).await;

    let (lan, p443) = (lab.lan_ip(), lab.tcp(443));
    let mut traefik = lab.service("Traefik", vec![(&p443, &lan)]);
    // What a daemon sends for an identified service.
    traefik.base.source = EntitySource::DiscoveryWithMatch {
        details: MatchDetails::new_certain("Traefik dashboard answered"),
    };
    lab.scan(vec![lan], vec![p443], vec![traefik]).await;

    let traefik = lab.named("Traefik").await.expect("traefik stored");
    assert_eq!(traefik.base.source, EntitySource::Manual);
}

/// An all-addresses binding is covered only when the port was covered at every address the host
/// has. A scan of one of two addresses leaves it.
#[tokio::test]
async fn an_all_addresses_binding_needs_coverage_at_every_address() {
    let (storage, services, _container) = test_services().await;
    let lab = Lab::new(&services, &storage).await;
    let (lan, second, p443) = (lab.lan_ip(), lab.second_lan_ip(), lab.tcp(443));
    let mut traefik = lab.service("Traefik", vec![]);
    traefik.base.bindings = vec![Binding::new_port_serviceless(p443.id, None)];
    lab.scan(vec![lan, second], vec![p443], vec![traefik]).await;

    let (lan, second, p443) = (lab.lan_ip(), lab.second_lan_ip(), lab.tcp(443));
    let caddy = lab.service("Caddy", vec![(&p443, &second)]);
    lab.scan_covering(
        vec![lan, second],
        vec![p443],
        vec![caddy],
        covering(SECOND_LAN_IP, &[443]),
    )
    .await;

    assert_eq!(lab.ports_named("Traefik").await, Some(vec![(443, None)]));
    assert_eq!(lab.named("Caddy").await, None);
}
