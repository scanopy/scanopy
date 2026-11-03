use std::collections::HashMap;
use std::net::IpAddr;

use crate::daemon::utils::app_probe::tls::PortCertificate;
use crate::server::credentials::r#impl::mapping::SnmpCredentialMapping;
use crate::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback};
use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
use crate::server::services::r#impl::base::Service;
use crate::server::services::r#impl::virtualization::ServiceVirtualization;
use crate::server::shared::attribution::AttributeSource;
use crate::tests::{organization, site};
use uuid::Uuid;

use crate::{
    server::{
        ip_addresses::r#impl::base::IPAddress,
        ports::r#impl::base::PortType,
        services::{
            definitions::ServiceDefinitionRegistry,
            r#impl::{
                base::{
                    DiscoverySessionServiceMatchParams, ServiceMatchBaselineParams,
                    ServiceMatchServiceParams,
                },
                definitions::ServiceDefinition,
                endpoints::{Endpoint, EndpointResponse},
                patterns::Pattern,
            },
        },
        subnets::r#impl::base::Subnet,
    },
    tests::{ip_address, subnet},
};

struct TestContext {
    subnet: Subnet,
    ip_address: IPAddress,
    pi: Box<dyn ServiceDefinition>,
    host_id: Uuid,
    daemon_id: Uuid,
    site_id: Uuid,
    discovery_type: DiscoveryType,
    gateway_ips: Vec<IpAddr>,
    endpoint_responses: Vec<EndpointResponse>,
    virtualization: Option<ServiceVirtualization>,
    matched_services: Vec<Service>,
    claimed_ports: Vec<PortType>,
    client_responses: std::collections::HashMap<super::ClientProbe, Vec<PortType>>,
    managed_device: Option<super::ManagedDevice>,
    dns_sd: Option<crate::daemon::discovery::service::network::mdns::DnsSdHost>,
    certificates: Vec<crate::daemon::utils::app_probe::tls::PortCertificate>,
    fingerprints: Vec<crate::daemon::discovery::service::network::emitted_os::PortFingerprint>,
}

impl TestContext {
    fn new() -> Self {
        let organization = organization();
        let site = site(&organization.id);
        let subnet = subnet(&site.id);
        let ip_address = ip_address(&site.id, &subnet.id);
        let pi =
            ServiceDefinitionRegistry::find_by_id("Pi-Hole").expect("Pi-hole service not found");

        let endpoint_responses = vec![EndpointResponse {
            endpoint: Endpoint::http(Some(ip_address.base.ip_address), "/admin"),
            body: "Pi-hole".to_string(),
            headers: HashMap::new(),
            status: 200,
            over_tls: false,
            set_cookies: Vec::new(),
        }];

        Self {
            subnet,
            ip_address,
            pi,
            host_id: Uuid::new_v4(),
            site_id: Uuid::new_v4(),
            daemon_id: Uuid::new_v4(),
            discovery_type: DiscoveryType::Network {
                subnet_ids: None,
                host_naming_fallback: HostNamingFallback::BestService,
                snmp_credentials: SnmpCredentialMapping::default(),
            },
            gateway_ips: vec![],
            endpoint_responses,
            virtualization: None,
            matched_services: vec![],
            claimed_ports: vec![],
            client_responses: std::collections::HashMap::new(),
            managed_device: None,
            dns_sd: None,
            certificates: Vec::new(),
            fingerprints: Vec::new(),
        }
    }

    fn create_params_with_ports<'a>(
        &'a self,
        baseline_params: &'a ServiceMatchBaselineParams<'a>,
        unbound_ports: &'a Vec<PortType>,
    ) -> DiscoverySessionServiceMatchParams<'a> {
        DiscoverySessionServiceMatchParams {
            host_id: &self.host_id,
            gateway_ips: &self.gateway_ips,
            daemon_id: &self.daemon_id,
            site_id: &self.site_id,
            discovery_type: &self.discovery_type,
            baseline_params,
            service_params: ServiceMatchServiceParams {
                service_definition: self.pi.clone(),
                matched_services: &self.matched_services,
                claimed_ports: &self.claimed_ports,
                unbound_ports,
            },
        }
    }

    fn create_baseline_params<'a>(
        &'a self,
        all_ports: &'a Vec<PortType>,
    ) -> ServiceMatchBaselineParams<'a> {
        ServiceMatchBaselineParams {
            subnet: &self.subnet,
            ip_address: &self.ip_address,
            all_ports,
            endpoint_responses: &self.endpoint_responses,
            virtualization_metadata: &self.virtualization,
            virtualization_service_id: None,
            client_responses: &self.client_responses,
            managed_device: &self.managed_device,
            dns_sd: &self.dns_sd,
            certificates: &self.certificates,
            fingerprints: &self.fingerprints,
        }
    }
}

#[test]
fn test_container_virtualization_matches_docker_and_podman() {
    use crate::server::services::definitions::docker_container::DockerContainer;
    use crate::server::services::definitions::podman_container::PodmanContainer;
    use crate::server::services::r#impl::virtualization::PodmanVirtualization;

    let ports: Vec<PortType> = vec![];

    // Podman container virtualization → ContainerVirtualization matches, the
    // PodmanContainer generic claims it, and the DockerContainer generic does not.
    let mut ctx = TestContext::new();
    ctx.virtualization = Some(ServiceVirtualization::Podman(PodmanVirtualization {
        state: Default::default(),
        container_name: Some("scanopy-test-web".to_string()),
        container_id: Some("73413cba1d1c".to_string()),
        compose_project: None,
    }));
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);

    assert!(
        Pattern::ContainerVirtualization.matches(&params).is_ok(),
        "ContainerVirtualization should match a Podman container"
    );
    assert!(
        PodmanContainer.discovery_pattern().matches(&params).is_ok(),
        "PodmanContainer generic should claim a Podman container"
    );
    assert!(
        DockerContainer
            .discovery_pattern()
            .matches(&params)
            .is_err(),
        "DockerContainer generic must NOT claim a Podman container"
    );

    // Symmetric check: a Docker container is claimed by DockerContainer, not PodmanContainer.
    let mut dctx = TestContext::new();
    dctx.virtualization = Some(ServiceVirtualization::Docker(
        crate::server::services::r#impl::virtualization::DockerVirtualization {
            state: Default::default(),
            container_name: Some("nginx".to_string()),
            container_id: Some("abc123".to_string()),
            compose_project: None,
        },
    ));
    let dbaseline = dctx.create_baseline_params(&ports);
    let dparams = dctx.create_params_with_ports(&dbaseline, &ports);
    assert!(
        DockerContainer
            .discovery_pattern()
            .matches(&dparams)
            .is_ok(),
        "DockerContainer generic should claim a Docker container"
    );
    assert!(
        PodmanContainer
            .discovery_pattern()
            .matches(&dparams)
            .is_err(),
        "PodmanContainer generic must NOT claim a Docker container"
    );
}

#[test]
fn test_pattern_port_matching() {
    let ctx = TestContext::new();

    let ports = vec![PortType::DnsUdp, PortType::DnsTcp];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let pattern = ctx.pi.discovery_pattern();
    let result = pattern.matches(&params);

    assert!(
        result.is_ok(),
        "Pi-hole pattern should match port 53 and endpoint"
    );

    // Test with wrong port - should not match
    let ports = vec![PortType::new_tcp(80)];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let pattern = ctx.pi.discovery_pattern();
    let result = pattern.matches(&params);

    assert!(result.is_err(), "Pi-hole pattern should not match port 80");
}

#[test]
fn test_pattern_and_logic() {
    let ctx = TestContext::new();

    let pattern = Pattern::AllOf(vec![
        Pattern::Port(PortType::new_tcp(80)),
        Pattern::Port(PortType::new_tcp(443)),
    ]);

    let ports = vec![PortType::new_tcp(80), PortType::new_tcp(443)];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let result = pattern.matches(&params);

    assert!(
        result.is_ok(),
        "AND pattern should match when both conditions met"
    );

    // Test with only one port - should not match
    let ports = vec![PortType::new_tcp(80)];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let result = pattern.matches(&params);

    assert!(
        result.is_err(),
        "AND pattern should not match when only one condition met"
    );

    // Test with neither port - should not match
    let ports = vec![PortType::new_tcp(22)];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let result = pattern.matches(&params);

    assert!(
        result.is_err(),
        "AND pattern should not match when no conditions met"
    );
}

#[test]
fn test_pattern_or_logic() {
    let ctx = TestContext::new();

    // Create OR pattern for database ports (MySQL or PostgreSQL)
    let pattern = Pattern::AnyOf(vec![
        Pattern::Port(PortType::new_tcp(3306)), // MySQL
        Pattern::Port(PortType::new_tcp(5432)), // PostgreSQL
    ]);

    let ports = vec![PortType::new_tcp(3306)];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let result = pattern.matches(&params);
    assert!(result.is_ok(), "OR pattern should match MySQL port");

    // Test with PostgreSQL port - should match
    let ports = vec![PortType::new_tcp(5432)];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let result = pattern.matches(&params);
    assert!(result.is_ok(), "OR pattern should match PostgreSQL port");

    // Test with both ports - should match
    let ports = vec![PortType::new_tcp(3306), PortType::new_tcp(5432)];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let result = pattern.matches(&params);
    assert!(result.is_ok(), "OR pattern should match with both ports");

    // Test with neither port - should not match
    let ports = vec![PortType::new_tcp(22)];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let result = pattern.matches(&params);
    assert!(
        result.is_err(),
        "OR pattern should not match when no conditions met"
    );
}

/// Whether the registered definition `id` matches one 200 response from `path` on `port` carrying
/// `headers`.
fn definition_matches_response(
    id: &str,
    port: PortType,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> bool {
    definition_matches_status(id, port, path, 200, headers, body)
}

/// Whether the registered definition `id` matches one response with `status` from `path` on
/// `port` carrying `headers`.
fn definition_matches_status(
    id: &str,
    port: PortType,
    path: &str,
    status: u16,
    headers: &[(&str, &str)],
    body: &str,
) -> bool {
    let ctx = TestContext::new();
    let service =
        ServiceDefinitionRegistry::find_by_id(id).unwrap_or_else(|| panic!("{id} is registered"));
    let ports = vec![port];
    let endpoint_responses = vec![EndpointResponse {
        endpoint: Endpoint::for_pattern(port, path).use_ip(ctx.ip_address.base.ip_address),
        body: body.to_string(),
        headers: headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect(),
        status,
        over_tls: false,
        set_cookies: Vec::new(),
    }];
    let client_responses = HashMap::new();
    let baseline = ServiceMatchBaselineParams {
        subnet: &ctx.subnet,
        ip_address: &ctx.ip_address,
        all_ports: &ports,
        endpoint_responses: &endpoint_responses,
        virtualization_metadata: &ctx.virtualization,
        virtualization_service_id: None,
        client_responses: &client_responses,
        managed_device: &None,
        dns_sd: &None,
        certificates: &[],
        fingerprints: &[],
    };
    let params = DiscoverySessionServiceMatchParams {
        host_id: &ctx.host_id,
        gateway_ips: &ctx.gateway_ips,
        daemon_id: &ctx.daemon_id,
        site_id: &ctx.site_id,
        discovery_type: &ctx.discovery_type,
        baseline_params: &baseline,
        service_params: ServiceMatchServiceParams {
            service_definition: service.clone(),
            matched_services: &ctx.matched_services,
            claimed_ports: &ctx.claimed_ports,
            unbound_ports: &ports,
        },
    };
    service.discovery_pattern().matches(&params).is_ok()
}

/// An echo server (`traefik/whoami`) writes the request line back into the body, so the path
/// `/zabbix` alone used to satisfy "body contains zabbix". The echo proves nothing; a real
/// Zabbix page still matches.
#[test]
fn an_echoed_request_path_does_not_match_the_service_it_names() {
    let echo = "Hostname: 4f1c2a\nIP: 192.168.7.230\nGET /zabbix HTTP/1.1\nHost: 192.168.7.230\nUser-Agent: scanopy\n";
    assert!(!definition_matches_response(
        "Zabbix",
        PortType::Http,
        "/zabbix",
        &[],
        echo
    ));

    let zabbix = "<!DOCTYPE html><html><head><title>Zabbix</title></head><body>Sign in to Zabbix</body></html>";
    assert!(definition_matches_response(
        "Zabbix",
        PortType::Http,
        "/zabbix",
        &[],
        zabbix
    ));
}

const NGINX: (&str, &str) = ("server", "nginx/1.24.0 (Ubuntu)");
const NGINX_WELCOME: &str = "<!DOCTYPE html><html><head><title>Welcome to nginx!</title></head><body><h1>Welcome to nginx!</h1></body></html>";

/// The linuxserver.io calibre image puts its Selkies remote desktop on 8080. That page names
/// nothing calibre-specific, and every other Selkies image serves the same one.
const SELKIES_DESKTOP: &str = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><link rel=\"stylesheet\" href=\"src/selkies-core.css\"></head><body><div id=\"app\"></div><div id=\"touch-gamepad-host\"></div><script src=\"src/universalTouchGamepad.js\"></script></body></html>";

#[test]
fn calibre_is_identified_by_its_server_header_or_its_index_page() {
    let index = "<!DOCTYPE html><html><head><title>calibre</title></head><body><div id=\"page_load_progress\"><progress></progress><div>Loading, please wait&hellip;</div><div>You can also use a simple, non-JavaScript version of this site: <a href=\"mobile\">here</a></div></div></body></html>";
    for port in [PortType::Http8080, PortType::Http8081] {
        assert!(definition_matches_response(
            "Calibre",
            port,
            "/",
            &[("server", "calibre 8.11.1")],
            index
        ));
        // Behind a proxy that rewrites `Server`, the page alone identifies it.
        assert!(definition_matches_response(
            "Calibre",
            port,
            "/",
            &[NGINX],
            index
        ));
    }

    assert!(!definition_matches_response(
        "Calibre",
        PortType::Http8080,
        "/",
        &[NGINX],
        SELKIES_DESKTOP
    ));
    assert!(!definition_matches_response(
        "Calibre",
        PortType::Http8080,
        "/",
        &[NGINX],
        NGINX_WELCOME
    ));
}

#[test]
fn calibre_web_is_identified_by_its_login_page() {
    let port = PortType::new_tcp(8083);
    let login = "<!DOCTYPE html><html lang=\"en\"><head><title>Calibre-Web | Login</title><link href=\"/static/css/libs/bootstrap.min.css\" rel=\"stylesheet\"></head><body class=\"login\"><a class=\"navbar-brand\" href=\"/\">Calibre-Web</a><form method=\"POST\" role=\"form\"><input type=\"checkbox\" name=\"remember_me\" checked></form></body></html>";
    assert!(definition_matches_response(
        "Calibre-Web",
        port,
        "/login",
        &[],
        login
    ));

    assert!(!definition_matches_response(
        "Calibre-Web",
        port,
        "/login",
        &[NGINX],
        NGINX_WELCOME
    ));
    // A dashboard tile linking to Calibre-Web names it without the title separator.
    assert!(!definition_matches_response(
        "Calibre-Web",
        port,
        "/login",
        &[],
        "<html><head><title>Homepage</title></head><body><a href=\"http://books.lan\">Calibre-Web</a></body></html>"
    ));
}

#[test]
fn dawarich_is_identified_by_its_health_endpoints_version_header() {
    let health = "{\"status\":\"ok\"}";
    assert!(definition_matches_response(
        "Dawarich",
        PortType::Http3000,
        "/api/v1/health",
        &[
            ("x-dawarich-response", "Hey, I'm alive!"),
            ("x-dawarich-version", "0.30.6"),
        ],
        health
    ));

    // Another app on 3000 with the same health body is not Dawarich.
    assert!(!definition_matches_response(
        "Dawarich",
        PortType::Http3000,
        "/api/v1/health",
        &[("content-type", "application/json")],
        health
    ));
    assert!(!definition_matches_response(
        "Dawarich",
        PortType::Http3000,
        "/",
        &[NGINX],
        NGINX_WELCOME
    ));
}

/// Bodies captured from a Spring Boot 3 app built from start.spring.io with web and actuator.
#[test]
fn spring_boot_is_identified_by_its_actuator_health_endpoint_up_or_down() {
    let up = "{\"groups\":[\"liveness\",\"readiness\"],\"status\":\"UP\"}";
    let down = "{\"groups\":[\"liveness\",\"readiness\"],\"status\":\"DOWN\"}";
    for port in [PortType::Http8080, PortType::Http8081] {
        assert!(definition_matches_status(
            "Spring Boot",
            port,
            "/actuator/health",
            200,
            &[],
            up
        ));
        assert!(definition_matches_status(
            "Spring Boot",
            port,
            "/actuator/health",
            503,
            &[],
            down
        ));
    }

    // With Actuator on a separate management port, the app port answers the health path with
    // Spring's error body. Nothing else at that path is identified either.
    assert!(!definition_matches_status(
        "Spring Boot",
        PortType::Http8080,
        "/actuator/health",
        404,
        &[],
        "{\"timestamp\":\"2026-10-20T20:40:54.744Z\",\"status\":404,\"error\":\"Not Found\",\"path\":\"/actuator/health\"}"
    ));
    assert!(!definition_matches_response(
        "Spring Boot",
        PortType::Http8080,
        "/actuator/health",
        &[NGINX],
        NGINX_WELCOME
    ));
}

/// Captured from `mher/flower:2.0` against a Redis broker.
#[test]
fn celery_is_identified_by_its_flower_dashboard() {
    let port = PortType::new_tcp(5555);
    let flower = "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Flower</title><link href=\"/static/css/flower.css\" rel=\"stylesheet\"></head><body><nav class=\"navbar\"><a class=\"navbar-brand\" href=\"/\">Flower</a><a class=\"nav-link\" href=\"https://flower.readthedocs.io/\">Documentation</a></nav></body></html>";
    assert!(definition_matches_response(
        "Celery",
        port,
        "/",
        &[],
        flower
    ));

    assert!(!definition_matches_response(
        "Celery",
        port,
        "/",
        &[NGINX],
        NGINX_WELCOME
    ));
    // A page that only links to Flower is not Flower.
    assert!(!definition_matches_response(
        "Celery",
        port,
        "/",
        &[],
        "<html><head><title>Homepage</title></head><body><a href=\"http://flower.lan\">Flower</a></body></html>"
    ));
}

/// Captured from `Sidekiq::Web` mounted at /sidekiq in a Rack app. Every asset and link sits
/// under /sidekiq, so the title and the navbar brand are what name it once the requested path is
/// set aside.
#[test]
fn sidekiq_is_identified_by_its_web_dashboard() {
    let dashboard = "<!doctype html><html dir=\"ltr\"><head><title>[DEVELOPMENT] Sidekiq</title><link href=\"/sidekiq/stylesheets/style.css\" media=\"screen\" rel=\"stylesheet\" type=\"text/css\"><script type=\"text/javascript\" src=\"/sidekiq/javascripts/application.js\"></script></head><body><nav class=\"navbar\"><a class=\"navbar-brand\" href=\"/sidekiq/\">Sidekiq</a><ul><li><a href=\"/sidekiq/\">Dashboard</a></li><li><a href=\"/sidekiq/busy\">Busy</a></li><li><a href=\"/sidekiq/queues\">Queues</a></li><li><a href=\"/sidekiq/retries\">Retries</a></li></ul></nav></body></html>";
    for port in [PortType::Http3000, PortType::Http] {
        assert!(definition_matches_response(
            "Sidekiq",
            port,
            "/sidekiq",
            &[],
            dashboard
        ));
    }

    // A Rails page that links to the dashboard names only the path.
    assert!(!definition_matches_response(
        "Sidekiq",
        PortType::Http3000,
        "/sidekiq",
        &[],
        "<html><head><title>Admin</title></head><body><a href=\"/sidekiq\">Background jobs</a></body></html>"
    ));
    assert!(!definition_matches_response(
        "Sidekiq",
        PortType::Http,
        "/sidekiq",
        &[NGINX],
        NGINX_WELCOME
    ));
}

#[test]
fn test_jenkins_https_header_detection() {
    let ctx = TestContext::new();
    let service = ServiceDefinitionRegistry::find_by_id("Jenkins")
        .expect("Jenkins service should be registered");

    let ports = vec![PortType::Https];
    let endpoint_responses = vec![EndpointResponse {
        endpoint: Endpoint::for_pattern(PortType::Https, "/")
            .use_ip(ctx.ip_address.base.ip_address),
        body: "Authentication required".to_string(),
        headers: HashMap::from([("x-jenkins".to_string(), "2.541.3".to_string())]),
        status: 403,
        over_tls: false,
        set_cookies: Vec::new(),
    }];
    let client_responses = HashMap::new();
    let baseline = ServiceMatchBaselineParams {
        subnet: &ctx.subnet,
        ip_address: &ctx.ip_address,
        all_ports: &ports,
        endpoint_responses: &endpoint_responses,
        virtualization_metadata: &ctx.virtualization,
        virtualization_service_id: None,
        client_responses: &client_responses,
        managed_device: &None,
        dns_sd: &None,
        certificates: &[],
        fingerprints: &[],
    };
    let params = DiscoverySessionServiceMatchParams {
        host_id: &ctx.host_id,
        gateway_ips: &ctx.gateway_ips,
        daemon_id: &ctx.daemon_id,
        site_id: &ctx.site_id,
        discovery_type: &ctx.discovery_type,
        baseline_params: &baseline,
        service_params: ServiceMatchServiceParams {
            service_definition: service.clone(),
            matched_services: &ctx.matched_services,
            claimed_ports: &ctx.claimed_ports,
            unbound_ports: &ports,
        },
    };

    let result = service.discovery_pattern().matches(&params);
    assert!(
        result.is_ok(),
        "Jenkins should match HTTPS responses that include the X-Jenkins header"
    );
}

#[test]
fn test_jenkins_http8080_body_detection() {
    let ctx = TestContext::new();
    let service = ServiceDefinitionRegistry::find_by_id("Jenkins")
        .expect("Jenkins service should be registered");

    let ports = vec![PortType::Http8080];
    let endpoint_responses = vec![EndpointResponse {
        endpoint: Endpoint::for_pattern(PortType::Http8080, "/")
            .use_ip(ctx.ip_address.base.ip_address),
        body: "powered by jenkins.io".to_string(),
        headers: HashMap::new(),
        status: 200,
        over_tls: false,
        set_cookies: Vec::new(),
    }];
    let client_responses = HashMap::new();
    let baseline = ServiceMatchBaselineParams {
        subnet: &ctx.subnet,
        ip_address: &ctx.ip_address,
        all_ports: &ports,
        endpoint_responses: &endpoint_responses,
        virtualization_metadata: &ctx.virtualization,
        virtualization_service_id: None,
        client_responses: &client_responses,
        managed_device: &None,
        dns_sd: &None,
        certificates: &[],
        fingerprints: &[],
    };
    let params = DiscoverySessionServiceMatchParams {
        host_id: &ctx.host_id,
        gateway_ips: &ctx.gateway_ips,
        daemon_id: &ctx.daemon_id,
        site_id: &ctx.site_id,
        discovery_type: &ctx.discovery_type,
        baseline_params: &baseline,
        service_params: ServiceMatchServiceParams {
            service_definition: service.clone(),
            matched_services: &ctx.matched_services,
            claimed_ports: &ctx.claimed_ports,
            unbound_ports: &ports,
        },
    };

    let result = service.discovery_pattern().matches(&params);
    assert!(
        result.is_ok(),
        "Jenkins should keep matching the existing HTTP 8080 body signature"
    );
}

#[test]
fn test_client_response_with_port() {
    let mut ctx = TestContext::new();
    let probed_port = PortType::new_tcp(2376);
    ctx.client_responses
        .insert(super::ClientProbe::Docker, vec![probed_port]);

    let ports = vec![probed_port];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let pattern = Pattern::ClientResponse(super::ClientProbe::Docker);
    let result = pattern.matches(&params);
    assert!(
        result.is_ok(),
        "ClientResponse should match when probe succeeded"
    );
    let result = result.unwrap();
    assert_eq!(
        result.ports,
        vec![probed_port],
        "Should return the probed port"
    );
    assert_eq!(result.details.confidence, super::MatchConfidence::Certain);
}

/// An SSH credential configured for a port other than 22 identifies the SSH service on the port
/// it authenticated on. The service used to require 22 open as well, so a credential on 2222
/// authenticated and then collected nothing, because nothing on the host was SSH.
#[test]
fn ssh_matches_on_the_port_its_credential_authenticated_on() {
    use crate::server::services::definitions::ssh::Ssh;
    use crate::server::services::r#impl::definitions::ServiceDefinition;

    let mut ctx = TestContext::new();
    let credential_port = PortType::new_tcp(2222);
    ctx.client_responses
        .insert(super::ClientProbe::Ssh, vec![credential_port]);

    let ports = vec![credential_port];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let result = Ssh
        .discovery_pattern()
        .matches(&params)
        .expect("SSH matches on the credential's port");

    assert_eq!(result.ports, vec![credential_port]);
    assert_eq!(result.details.confidence, super::MatchConfidence::Certain);
}

#[test]
fn test_client_response_without_port() {
    let mut ctx = TestContext::new();
    ctx.client_responses
        .insert(super::ClientProbe::Docker, vec![]);

    let ports = vec![];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let pattern = Pattern::ClientResponse(super::ClientProbe::Docker);
    let result = pattern.matches(&params);
    assert!(
        result.is_ok(),
        "ClientResponse should match even without ports (local socket)"
    );
    let result = result.unwrap();
    assert!(
        result.ports.is_empty(),
        "Should return no ports for local socket case"
    );
    assert!(
        result.reached_off_network,
        "a probe that answered on no port reached the service off the network"
    );
}

/// A service a probe reached on a port is on the network there, even when another arm of its
/// pattern answered off it.
#[test]
fn a_client_response_with_a_port_is_on_the_network() {
    let mut ctx = TestContext::new();
    let api_port = PortType::new_tcp(2376);
    ctx.client_responses
        .insert(super::ClientProbe::Docker, vec![api_port]);
    ctx.client_responses
        .insert(super::ClientProbe::Podman, vec![]);

    let ports = vec![api_port];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let result = Pattern::AnyOf(vec![
        Pattern::ClientResponse(super::ClientProbe::Docker),
        Pattern::ClientResponse(super::ClientProbe::Podman),
    ])
    .matches(&params)
    .expect("both probes answered");
    assert_eq!(result.ports, vec![api_port]);
    assert!(!result.reached_off_network);
}

/// An off-network answer carries through a composite that adds no port, so the service still
/// gets no binding.
#[test]
fn an_off_network_answer_carries_through_a_composite_without_ports() {
    let mut ctx = TestContext::new();
    ctx.client_responses
        .insert(super::ClientProbe::Libvirt, vec![]);

    let ports = vec![];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let result = Pattern::AllOf(vec![
        Pattern::ClientResponse(super::ClientProbe::Libvirt),
        Pattern::Optional(Box::new(Pattern::Port(PortType::new_tcp(16509)))),
    ])
    .matches(&params)
    .expect("the probe answered");
    assert!(result.ports.is_empty());
    assert!(result.reached_off_network);
}

#[test]
fn test_client_response_no_probe() {
    let ctx = TestContext::new();

    let ports = vec![PortType::new_tcp(2376)];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let pattern = Pattern::ClientResponse(super::ClientProbe::Docker);
    let result = pattern.matches(&params);
    assert!(
        result.is_err(),
        "ClientResponse should not match when probe not present"
    );
}

#[test]
fn test_mac_vendor_pattern_match() {
    let mut ctx = TestContext::new();
    // Set a known Sonos MAC address (B8:E9:37 is a Sonos OUI prefix)
    ctx.ip_address.base.mac_address = Some(MacEvidence::new(
        MacEvidenceValue("B8:E9:37:00:00:01".parse().expect("valid MAC")),
        AttributeSource::ArpReply,
    ));

    let ports = vec![];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let pattern = Pattern::MacVendor(super::Vendor::SONOS);
    let result = pattern.matches(&params);

    assert!(
        result.is_ok(),
        "MacVendor should match Sonos MAC: {:?}",
        result.err()
    );
    let match_result = result.unwrap();
    assert!(match_result.mac_vendor.is_some());
}

#[test]
fn test_mac_vendor_pattern_no_mac() {
    let ctx = TestContext::new();
    // Default interface has mac_address: None

    let ports = vec![];
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let pattern = Pattern::MacVendor(super::Vendor::SONOS);
    let result = pattern.matches(&params);

    assert!(
        result.is_err(),
        "MacVendor should error when ip_address has no MAC"
    );
}

/// A UniFi switch answers no distinguishing network probe, so the controller's report is
/// the only thing that can identify it. Both halves matter: the report must be sufficient,
/// and it must also be *necessary* — a Ubiquiti MAC alone must not claim every Ubiquiti
/// device on the network is a switch.
#[test]
fn controller_report_is_what_identifies_a_unifi_switch() {
    use crate::server::services::definitions::unifi_switch::UnifiSwitch;

    let mut ctx = TestContext::new();
    ctx.ip_address.base.mac_address = Some(MacEvidence::new(
        MacEvidenceValue("78:8A:20:00:00:01".parse().expect("valid MAC")),
        AttributeSource::ArpReply,
    ));

    let ports = vec![];
    let pattern = UnifiSwitch.discovery_pattern();

    // Without the controller's report, a Ubiquiti MAC is not enough.
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    assert!(
        pattern.matches(&params).is_err(),
        "a Ubiquiti MAC alone must not identify a switch"
    );

    // With it, the switch is identified.
    ctx.managed_device = Some(super::ManagedDevice {
        device_type: super::UnifiDeviceType::SWITCH.to_string(),
    });
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    let result = pattern.matches(&params);
    assert!(
        result.is_ok(),
        "controller-reported 'usw' should identify a UniFi switch: {:?}",
        result.err()
    );

    // A different device class on the same host must not match the switch definition.
    ctx.managed_device = Some(super::ManagedDevice {
        device_type: super::UnifiDeviceType::ACCESS_POINT.to_string(),
    });
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    assert!(
        pattern.matches(&params).is_err(),
        "an access point must not match the switch definition"
    );
}

use super::DnsSdServiceType;

/// A host announcing each `(service_type, txt)` pair, where the TXT entries are `key=value`.
fn announcing(
    services: &[(&str, &[&str])],
) -> crate::daemon::discovery::service::network::mdns::DnsSdHost {
    crate::daemon::discovery::service::network::mdns::DnsSdHost {
        services: services
            .iter()
            .map(|(service, txt)| {
                let entries = txt
                    .iter()
                    .filter_map(|pair| pair.split_once('='))
                    .map(|(k, v)| (k.to_ascii_lowercase(), v.to_string()))
                    .collect();
                (service.to_string(), entries)
            })
            .collect(),
        ..Default::default()
    }
}

/// An Apple TV answers no distinguishing network probe — the whole reason it needed mDNS — so
/// its announcement is both necessary and sufficient, and each half is worth pinning.
///
/// The third case is the one that bites: `_airplay._tcp` alone is a HomePod, an AirPort or a
/// Roku, and a definition matching on it would claim all of them are Apple TVs.
#[test]
fn an_apple_tv_is_identified_by_its_announcement_and_only_by_it() {
    use crate::server::services::definitions::apple_tv::AppleTv;
    use crate::server::services::definitions::homepod::HomePod;

    // The three TXT records verbatim from the network that exposed this: a MacBook, and two
    // Sonos speakers. All three advertise `_airplay._tcp`, which is why matching on the
    // service type alone labelled the MacBook an Apple TV and both speakers HomePods.
    let macbook = ("_airplay._tcp", &["model=Mac17,2"][..]);
    let sonos_five = ("_airplay._tcp", &["model=Five", "manufacturer=Sonos"][..]);
    let apple_tv = ("_airplay._tcp", &["model=AppleTV6,2"][..]);
    let homepod = ("_airplay._tcp", &["model=AudioAccessory5,1"][..]);

    let ports = vec![];
    let tv_pattern = AppleTv.discovery_pattern();
    let pod_pattern = HomePod.discovery_pattern();

    let mut ctx = TestContext::new();
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    assert!(
        tv_pattern.matches(&params).is_err(),
        "no mDNS response at all must not identify anything"
    );

    for (label, announcement, tv_matches, pod_matches) in [
        ("an Apple TV", apple_tv, true, false),
        ("a HomePod", homepod, false, true),
        ("a MacBook", macbook, false, false),
        ("a Sonos speaker", sonos_five, false, false),
    ] {
        ctx.dns_sd = Some(announcing(&[announcement]));
        let baseline = ctx.create_baseline_params(&ports);
        let params = ctx.create_params_with_ports(&baseline, &ports);
        assert_eq!(
            tv_pattern.matches(&params).is_ok(),
            tv_matches,
            "{label} vs the Apple TV definition"
        );
        assert_eq!(
            pod_pattern.matches(&params).is_ok(),
            pod_matches,
            "{label} vs the HomePod definition"
        );
    }
}

/// iPhones and iPads are told apart by the `_device-info` model prefix, including models newer
/// than any exact identifier Recog lists, and a Mac's model matches neither.
#[test]
fn iphones_and_ipads_are_identified_by_their_device_info_model() {
    use crate::server::services::definitions::{ipad::IPad, iphone::IPhone};

    let ports = vec![];
    let phone_pattern = IPhone.discovery_pattern();
    let tablet_pattern = IPad.discovery_pattern();

    for (label, model, phone_matches, tablet_matches) in [
        (
            "an iPhone newer than Recog's list",
            "model=iPhone17,1",
            true,
            false,
        ),
        (
            "an iPad newer than Recog's list",
            "model=iPad16,3",
            false,
            true,
        ),
        ("a MacBook", "model=Mac17,2", false, false),
    ] {
        let mut ctx = TestContext::new();
        ctx.dns_sd = Some(announcing(&[(DnsSdServiceType::DEVICE_INFO, &[model][..])]));
        let baseline = ctx.create_baseline_params(&ports);
        let params = ctx.create_params_with_ports(&baseline, &ports);
        assert_eq!(
            phone_pattern.matches(&params).is_ok(),
            phone_matches,
            "{label} vs the iPhone definition"
        );
        assert_eq!(
            tablet_pattern.matches(&params).is_ok(),
            tablet_matches,
            "{label} vs the iPad definition"
        );
    }
}

/// "The browse never reached this host" and "the host does not run this service" are different
/// statements, and the difference is not cosmetic: mDNS is link-local, so a device on a routed
/// subnet can run the service and never be asked. The reason text has to say which happened,
/// or an operator reads a scoping limit as an absence.
#[test]
fn an_unbrowsed_host_is_distinguished_from_one_that_did_not_answer() {
    let ctx_without = TestContext::new();
    let ports = vec![];
    let pattern = Pattern::DnsSd(DnsSdServiceType::GOOGLE_CAST, None);

    let baseline = ctx_without.create_baseline_params(&ports);
    let params = ctx_without.create_params_with_ports(&baseline, &ports);
    let unbrowsed = pattern.matches(&params).unwrap_err().to_string();

    let mut ctx_with = TestContext::new();
    ctx_with.dns_sd = Some(announcing(&[(DnsSdServiceType::AIRPLAY, &[][..])]));
    let baseline = ctx_with.create_baseline_params(&ports);
    let params = ctx_with.create_params_with_ports(&baseline, &ports);
    let answered_otherwise = pattern.matches(&params).unwrap_err().to_string();

    assert_ne!(
        unbrowsed, answered_otherwise,
        "a host we never browsed must not report the same reason as one that answered \
         without this service"
    );
}

/// A routing-table gateway is a Gateway even when a DHCP or DNS server matched on it first. Those
/// are generic definitions sorted ahead of Gateway, and a gateway nearly always runs one. Only a
/// gateway-specific definition (an Eero, a Fios router) already matched displaces the generic one.
#[test]
fn a_gateway_running_dhcp_still_matches_the_generic_gateway() {
    let service = |id: &str| Service {
        base: crate::server::services::r#impl::base::ServiceBase {
            service_definition: ServiceDefinitionRegistry::find_by_id(id).expect("definition"),
            ..Default::default()
        },
        ..Default::default()
    };

    let mut ctx = TestContext::new();
    ctx.gateway_ips = vec![ctx.ip_address.base.ip_address];
    ctx.pi = ServiceDefinitionRegistry::find_by_id("Gateway").expect("Gateway");
    let pattern = ctx.pi.discovery_pattern();
    let ports = vec![];

    ctx.matched_services = vec![service("DHCP Server"), service("DNS Server")];
    let baseline = ctx.create_baseline_params(&ports);
    assert!(
        pattern
            .matches(&ctx.create_params_with_ports(&baseline, &ports))
            .is_ok(),
        "DHCP and DNS on the gateway must not block the Gateway match"
    );

    ctx.matched_services = vec![service("Eero Gateway")];
    let baseline = ctx.create_baseline_params(&ports);
    assert!(
        pattern
            .matches(&ctx.create_params_with_ports(&baseline, &ports))
            .is_err(),
        "a gateway-specific definition already matched displaces the generic Gateway"
    );
}

/// Only an address a router can hold is a gateway, under either of `IsGateway`'s rules. A Mac's
/// loopback routes name 127.0.0.1 as their gateway, and with those dropped the subnet has no
/// gateway, so 127.0.0.1 ending in .1 then matched the fallback rule instead.
#[test]
fn a_loopback_address_is_never_a_gateway() {
    let ip = |s: &str| s.parse::<IpAddr>().unwrap();
    assert!(!super::is_router_gateway(&ip("127.0.0.1")));
    assert!(!super::is_router_gateway(&ip("::1")));
    assert!(!super::is_router_gateway(&ip("0.0.0.0")));
    assert!(!super::is_router_gateway(&ip("224.0.0.1")));
    assert!(super::is_router_gateway(&ip("fe80::1")));

    let matches_gateway = |address: &str, gateways: Vec<IpAddr>| {
        let mut ctx = TestContext::new();
        ctx.ip_address.base.ip_address = ip(address);
        ctx.gateway_ips = gateways;
        let ports = vec![];
        let baseline = ctx.create_baseline_params(&ports);
        Pattern::IsGateway
            .matches(&ctx.create_params_with_ports(&baseline, &ports))
            .is_ok()
    };
    // The .1 fallback rule, with no routing-table gateways known.
    assert!(!matches_gateway("127.0.0.1", vec![]));
    assert!(matches_gateway("192.168.1.1", vec![]));
    // The routing-table rule.
    assert!(!matches_gateway("127.0.0.1", vec![ip("127.0.0.1")]));
}

/// A host announcing `service_type` with `txt`, on `port` in its SRV record.
fn announcing_on(
    service_type: &str,
    txt: &[&str],
    port: u16,
) -> crate::daemon::discovery::service::network::mdns::DnsSdHost {
    let mut host = announcing(&[(service_type, txt)]);
    host.ports.insert(service_type.to_string(), port);
    host
}

/// A DnsSd match binds the port the SRV record names: the scanned port when the scan found it, a
/// port from the advertisement when the scan did not cover it, and none for a TXT-only service.
#[test]
fn a_dns_sd_match_binds_the_advertised_port() {
    use crate::server::services::definitions::{
        airprint_printer::AirPrintPrinter, homepod::HomePod, iphone::IPhone,
    };

    let mut ctx = TestContext::new();
    let scanned = vec![PortType::Ipp];
    ctx.dns_sd = Some(announcing_on("_ipp._tcp", &[], 631));
    let baseline = ctx.create_baseline_params(&scanned);
    let params = ctx.create_params_with_ports(&baseline, &scanned);
    let printer = AirPrintPrinter
        .discovery_pattern()
        .matches(&params)
        .expect("an advertised printer matches");
    assert_eq!(printer.ports, vec![PortType::Ipp]);

    // AirPlay's 7000 is not in the scan list; the speaker said it listens there.
    let none = vec![];
    ctx.dns_sd = Some(announcing_on(
        "_airplay._tcp",
        &["model=AudioAccessory5,1"],
        7000,
    ));
    let baseline = ctx.create_baseline_params(&none);
    let params = ctx.create_params_with_ports(&baseline, &none);
    let pod = HomePod
        .discovery_pattern()
        .matches(&params)
        .expect("a HomePod matches");
    assert_eq!(pod.ports, vec![PortType::new_tcp(7000)]);

    // `_device-info` advertises port 0, so the wire parser records no port for it.
    ctx.dns_sd = Some(announcing(&[("_device-info._tcp", &["model=iPhone17,1"])]));
    let baseline = ctx.create_baseline_params(&none);
    let params = ctx.create_params_with_ports(&baseline, &none);
    let phone = IPhone
        .discovery_pattern()
        .matches(&params)
        .expect("an iPhone matches");
    assert!(phone.ports.is_empty());
}

/// A port a service matched earlier this round holds is not bound a second time, and that claim
/// does not make `Not(DnsSd(..))` read as the host not advertising the service.
#[test]
fn a_claimed_advertised_port_is_not_bound_twice() {
    use crate::server::services::definitions::{
        airprint_printer::AirPrintPrinter, homekit_accessory::HomeKitAccessory,
    };

    let mut ctx = TestContext::new();
    let scanned = vec![PortType::Ipp];
    ctx.dns_sd = Some(announcing_on("_ipp._tcp", &[], 631));
    ctx.claimed_ports = vec![PortType::Ipp];
    let unbound = vec![];
    let baseline = ctx.create_baseline_params(&scanned);
    let params = ctx.create_params_with_ports(&baseline, &unbound);
    assert!(
        AirPrintPrinter
            .discovery_pattern()
            .matches(&params)
            .is_err(),
        "CUPS already holds 631"
    );

    // An Apple hub advertises HAP and AirPlay. With its AirPlay port held by the Apple TV
    // service, it still advertises AirPlay, so it is still not a HomeKit accessory.
    let mut hub = announcing_on("_airplay._tcp", &["model=AppleTV14,1"], 7000);
    hub.services
        .insert("_hap._tcp".to_string(), Default::default());
    ctx.dns_sd = Some(hub);
    ctx.claimed_ports = vec![PortType::new_tcp(7000)];
    let none = vec![];
    let baseline = ctx.create_baseline_params(&none);
    let params = ctx.create_params_with_ports(&baseline, &none);
    assert!(
        HomeKitAccessory
            .discovery_pattern()
            .matches(&params)
            .is_err()
    );
}

/// One response a host gave during the scan: port, path, status, headers, body.
type Response<'a> = (PortType, &'a str, u16, &'a [(&'a str, &'a str)], &'a str);

/// Whether the registered definition `id` matches a host with `mac`, open `ports`, and these
/// `responses`.
fn definition_matches_host(
    id: &str,
    mac: &str,
    ports: &[PortType],
    responses: &[Response],
) -> bool {
    definition_binds_on_host(id, mac, ports, responses).is_some()
}

/// The ports the registered definition `id` binds on that host, if it matches.
fn definition_binds_on_host(
    id: &str,
    mac: &str,
    ports: &[PortType],
    responses: &[Response],
) -> Option<Vec<PortType>> {
    definition_binds_with_certificates(id, mac, ports, responses, &[])
}

/// [`definition_binds_on_host`] for a host whose TLS ports presented `(port, issuer)` certificates.
fn definition_binds_with_certificates(
    id: &str,
    mac: &str,
    ports: &[PortType],
    responses: &[Response],
    certificates: &[(PortType, &str)],
) -> Option<Vec<PortType>> {
    let mut ctx = TestContext::new();
    ctx.certificates = certificates
        .iter()
        .map(|(port, issuer)| PortCertificate {
            port: *port,
            subject: "CN=device".to_string(),
            issuer: issuer.to_string(),
        })
        .collect();
    ctx.ip_address.base.mac_address = Some(MacEvidence::new(
        MacEvidenceValue(mac.parse().expect("valid MAC")),
        AttributeSource::ArpReply,
    ));
    ctx.pi =
        ServiceDefinitionRegistry::find_by_id(id).unwrap_or_else(|| panic!("{id} is registered"));
    ctx.endpoint_responses = responses
        .iter()
        .map(|(port, path, status, headers, body)| EndpointResponse {
            endpoint: Endpoint::for_pattern(*port, path).use_ip(ctx.ip_address.base.ip_address),
            body: body.to_string(),
            headers: headers
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
            status: *status,
            over_tls: false,
            set_cookies: Vec::new(),
        })
        .collect();
    let ports = ports.to_vec();
    let baseline = ctx.create_baseline_params(&ports);
    let params = ctx.create_params_with_ports(&baseline, &ports);
    ctx.pi
        .discovery_pattern()
        .matches(&params)
        .ok()
        .map(|matched| matched.ports)
}

/// TP-Link's newer OUI, on a Kasa HS103.
const TPLINK_SYSTEMS_MAC: &str = "8C:86:DD:00:00:01";
/// A Sonos OUI, standing in for any vendor the definitions do not name.
const OTHER_MAC: &str = "B8:E9:37:00:00:01";

/// What a Kasa plug sends on 80: the `SHIP 2.0` server header and a bare page.
#[test]
fn a_kasa_plug_is_identified_by_its_server_header_on_a_tp_link_mac() {
    let ship: Response = (
        PortType::Http,
        "/",
        200,
        &[("server", "SHIP 2.0")],
        "<html><body><center>200 OK</center></body></html>",
    );
    let ports = [PortType::Http];
    assert!(definition_matches_host(
        "TP-Link Kasa",
        TPLINK_SYSTEMS_MAC,
        &ports,
        &[ship]
    ));
    // The older TP-Link OUI ships on earlier Kasa hardware.
    assert!(definition_matches_host(
        "TP-Link Kasa",
        "50:C7:BF:00:00:01",
        &ports,
        &[ship]
    ));
    assert!(!definition_matches_host(
        "TP-Link Kasa",
        OTHER_MAC,
        &ports,
        &[ship]
    ));
    let plain: Response = (PortType::Http, "/", 200, &[("server", "nginx")], "200 OK");
    assert!(!definition_matches_host(
        "TP-Link Kasa",
        TPLINK_SYSTEMS_MAC,
        &ports,
        &[plain]
    ));
}

/// A Google Wifi point 404s `/`, so the Google Wifi probe reads its status API and reports a
/// `ClientProbe::GoogleWifi` response; that, on a non-gateway Google MAC, is a repeater. The root
/// also answers the API, and both definitions match on it: the gateway root is the router and the
/// non-gateway leaf is the repeater, split by `IsGateway` / `Not(IsGateway)`. The root's "Nest
/// Wifi" page identifies a unit that serves it either way.
#[test]
fn a_mesh_point_is_identified_by_its_status_api() {
    let google_mac = "B0:2A:43:00:00:01";
    let ports = vec![PortType::Http];

    // A Google unit whose status API answered the probe, matched against a definition.
    let probe_answered = |def_id: &str, is_gateway: bool| {
        let mut ctx = TestContext::new();
        ctx.ip_address.base.mac_address = Some(MacEvidence::new(
            MacEvidenceValue(google_mac.parse().expect("valid MAC")),
            AttributeSource::ArpReply,
        ));
        if is_gateway {
            ctx.gateway_ips = vec![ctx.ip_address.base.ip_address];
        }
        ctx.client_responses
            .insert(super::ClientProbe::GoogleWifi, vec![PortType::Http]);
        let baseline = ctx.create_baseline_params(&ports);
        let params = ctx.create_params_with_ports(&baseline, &ports);
        ServiceDefinitionRegistry::find_by_id(def_id)
            .unwrap_or_else(|| panic!("{def_id} is registered"))
            .discovery_pattern()
            .matches(&params)
            .is_ok()
    };
    // The probe response alone types each unit, split by whether it is the gateway.
    assert!(
        probe_answered("Google Nest Repeater", false),
        "a leaf's status API makes a repeater"
    );
    assert!(
        !probe_answered("Google Nest Repeater", true),
        "the gateway root is the router, not a repeater"
    );
    assert!(
        probe_answered("Google Nest Router", true),
        "the gateway root's status API makes a router"
    );
    assert!(
        !probe_answered("Google Nest Router", false),
        "a non-gateway leaf is not the router"
    );

    // The page the root serves still identifies a unit that serves it.
    assert!(definition_matches_host(
        "Google Nest Repeater",
        google_mac,
        &ports,
        &[(PortType::Http, "/", 200, &[], "<title>Nest Wifi</title>")]
    ));

    // A bare Google MAC on port 80 with neither the page nor a probe response is not a repeater.
    assert!(!definition_matches_host(
        "Google Nest Repeater",
        google_mac,
        &ports,
        &[]
    ));
}

/// Where the response identifies only a generic embedded web server, the OUI identifies the
/// product, and an open port gives the service something to bind.
#[test]
fn appliances_with_generic_web_servers_are_identified_by_their_oui() {
    let http = [PortType::Http];
    for (id, mac) in [
        ("Home Connect Appliance", "68:A4:0E:00:00:01"),
        ("myQ", "64:52:99:00:00:01"),
        ("Bose Speaker", "C8:7B:23:00:00:01"),
    ] {
        assert!(definition_matches_host(id, mac, &http, &[]), "{id}");
        assert!(
            !definition_matches_host(id, OTHER_MAC, &http, &[]),
            "{id} on another vendor's MAC"
        );
        assert!(
            !definition_matches_host(id, mac, &[], &[]),
            "{id} with nothing open"
        );
    }
    assert!(definition_matches_host(
        "Bose Speaker",
        "C8:7B:23:00:00:01",
        &[PortType::new_tcp(8091)],
        &[]
    ));
}

/// Spotify Connect binds the port its advertisement names, whatever the vendor chose.
#[test]
fn spotify_connect_binds_its_advertised_port() {
    use crate::server::services::definitions::spotify_connect::SpotifyConnect;

    let mut ctx = TestContext::new();
    ctx.dns_sd = Some(announcing_on(
        super::DnsSdServiceType::SPOTIFY_CONNECT,
        &["cpath=/zc"],
        8200,
    ));
    let none = vec![];
    let baseline = ctx.create_baseline_params(&none);
    let params = ctx.create_params_with_ports(&baseline, &none);
    let matched = SpotifyConnect
        .discovery_pattern()
        .matches(&params)
        .expect("an advertised receiver matches");
    assert_eq!(matched.ports, vec![PortType::new_tcp(8200)]);
}

/// Issuers as a live Chromecast Ultra, Nest Mini, Bose amp and Sonos presented them.
const CAST_VIDEO_ISSUER: &str =
    "CN=Chromecast ICA 5 (4K),OU=Cast,O=Google Inc,L=Mountain View,ST=California,C=US";
const CAST_AUDIO_ISSUER: &str =
    "CN=Chromecast ICA 16 (Audio Assist 4),OU=Cast,O=Google Inc,L=Mountain View,ST=California,C=US";
const CAST_HOME_ISSUER: &str =
    "CN=Usonia us-east1 ICA,OU=Cast,O=Google Inc,L=Mountain View,ST=California,C=US";

/// The Cast device certificate routes a receiver: a video CA to Chromecast, an `Audio Assist` CA to
/// Google Home. Each binds every port signed under Google's Cast CAs, on whatever numbers the unit
/// opened, and still matches on its ports alone when no certificate was read.
#[test]
fn cast_receivers_are_told_apart_and_bound_by_their_certificates() {
    let google_mac = "7C:D9:5C:00:00:01";
    let cast = [PortType::new_tcp(8008), PortType::new_tcp(8009)];
    let with_tls = |extra: &[u16]| {
        let mut ports = cast.to_vec();
        ports.extend(extra.iter().map(|p| PortType::new_tcp(*p)));
        ports
    };

    // Den TV: 4K Chromecast.
    let den_tv_ports = with_tls(&[9000, 10001, 10101]);
    let den_tv = [
        (PortType::new_tcp(9000), CAST_VIDEO_ISSUER),
        (PortType::new_tcp(10001), CAST_VIDEO_ISSUER),
        (PortType::new_tcp(10101), CAST_HOME_ISSUER),
    ];
    let bound =
        definition_binds_with_certificates("Chromecast", google_mac, &den_tv_ports, &[], &den_tv)
            .expect("a video Cast receiver is a Chromecast");
    for port in [9000, 10001, 10101] {
        assert!(
            bound.contains(&PortType::new_tcp(port)),
            "Chromecast binds {port}"
        );
    }
    assert!(
        definition_binds_with_certificates("Google Home", google_mac, &den_tv_ports, &[], &den_tv)
            .is_none(),
        "a video Cast receiver is no Google Home"
    );

    // Dining Room speaker: Nest Mini.
    let mini_ports = with_tls(&[11101, 10005]);
    let mini = [
        (PortType::new_tcp(11101), CAST_AUDIO_ISSUER),
        (PortType::new_tcp(10005), CAST_HOME_ISSUER),
    ];
    let bound =
        definition_binds_with_certificates("Google Home", google_mac, &mini_ports, &[], &mini)
            .expect("an audio Cast receiver is a Google Home");
    assert!(bound.contains(&PortType::new_tcp(11101)) && bound.contains(&PortType::new_tcp(10005)));
    assert!(
        definition_binds_with_certificates("Chromecast", google_mac, &mini_ports, &[], &mini)
            .is_none(),
        "an audio Cast receiver is no Chromecast"
    );

    // No certificate read: the port pair still identifies both, as before.
    for id in ["Chromecast", "Google Home"] {
        assert!(definition_matches_host(id, google_mac, &cast, &[]), "{id}");
    }
}

/// Bose and Sonos sign their TLS ports with their own CAs, which binds ports their port lists do
/// not name.
#[test]
fn speaker_vendors_bind_ports_their_certificates_sign() {
    let bose_mac = "C8:7B:23:00:00:01";
    let bose_ports: Vec<PortType> = [80, 8082, 8087, 10003]
        .into_iter()
        .map(PortType::new_tcp)
        .collect();
    let bose_issuer = "CN=Bose CEPD Device CA - PSE2,OU=Consumer Electronics Product Division,O=Bose Corporation,C=US";
    let bound = definition_binds_with_certificates(
        "Bose Speaker",
        bose_mac,
        &bose_ports,
        &[],
        &[
            (PortType::new_tcp(8082), bose_issuer),
            (PortType::new_tcp(8087), bose_issuer),
            (PortType::new_tcp(10003), bose_issuer),
        ],
    )
    .expect("a Bose speaker matches");
    for port in [8082, 8087, 10003] {
        assert!(
            bound.contains(&PortType::new_tcp(port)),
            "Bose binds {port}"
        );
    }

    let sonos_mac = "B8:E9:37:00:00:01";
    let sonos_ports = [PortType::new_tcp(1400), PortType::new_tcp(1443)];
    let bound = definition_binds_with_certificates(
        "Sonos Speaker",
        sonos_mac,
        &sonos_ports,
        &[],
        &[(
            PortType::new_tcp(1443),
            "CN=Sonos Device Authentication Root CA,OU=Sonos Devices,O=Sonos\\, Inc,L=Santa Barbara,ST=California,C=US",
        )],
    )
    .expect("a Sonos speaker matches");
    assert!(bound.contains(&PortType::new_tcp(1443)));
}

/// `CertificateIssuer` holds as evidence whichever service took the port, binds only ports still
/// unbound, and `Not` of it asks about the host.
#[test]
fn a_certificate_issuer_is_host_evidence_and_binds_only_unbound_ports() {
    let mut ctx = TestContext::new();
    ctx.certificates = vec![
        PortCertificate {
            port: PortType::new_tcp(9000),
            subject: "CN=a".to_string(),
            issuer: CAST_VIDEO_ISSUER.to_string(),
        },
        PortCertificate {
            port: PortType::new_tcp(10001),
            subject: "CN=b".to_string(),
            issuer: CAST_VIDEO_ISSUER.to_string(),
        },
    ];
    let all = vec![PortType::new_tcp(9000), PortType::new_tcp(10001)];
    // 9000 is already held by another service.
    let unbound = vec![PortType::new_tcp(10001)];
    let baseline = ctx.create_baseline_params(&all);
    let params = ctx.create_params_with_ports(&baseline, &unbound);

    let matched = Pattern::CertificateIssuer("ou=cast", &[])
        .matches(&params)
        .expect("case-insensitive issuer match");
    assert_eq!(matched.ports, vec![PortType::new_tcp(10001)]);

    assert!(
        Pattern::Not(Box::new(Pattern::CertificateIssuer("Chromecast ICA", &[])))
            .matches(&params)
            .is_err()
    );
    assert!(
        Pattern::CertificateIssuer("Bose", &[])
            .matches(&params)
            .is_err()
    );
}

/// macOS's AirPlay Receiver and MiniUPnPd both answer `/` on 5000 with an error status, and the
/// `Server` header is what names them.
#[test]
fn port_5000_is_identified_by_its_server_header_on_an_error_response() {
    let ports = [PortType::Http5000];
    let airtunes: Response = (
        PortType::Http5000,
        "/",
        403,
        &[("server", "AirTunes/980.77.5")],
        "",
    );
    let miniupnpd: Response = (
        PortType::Http5000,
        "/",
        404,
        &[("server", "Linux/5.4.0-122-generic UPnP/1.1 MiniUPnPd/1.9")],
        "",
    );
    assert!(definition_matches_host(
        "AirPlay Receiver",
        OTHER_MAC,
        &ports,
        &[airtunes]
    ));
    assert!(!definition_matches_host(
        "AirPlay Receiver",
        OTHER_MAC,
        &ports,
        &[miniupnpd]
    ));
    // AirPlay 2 receivers (Sonos, Bose, Macs) answer the same way on 7000.
    let airplay_2: Response = (
        PortType::new_tcp(7000),
        "/",
        404,
        &[("server", "AirTunes/366.0")],
        "",
    );
    assert!(definition_matches_host(
        "AirPlay Receiver",
        OTHER_MAC,
        &[PortType::new_tcp(7000)],
        &[airplay_2]
    ));
    assert!(definition_matches_host(
        "UPnP IGD",
        OTHER_MAC,
        &ports,
        &[miniupnpd]
    ));
    assert!(!definition_matches_host(
        "UPnP IGD",
        OTHER_MAC,
        &ports,
        &[airtunes]
    ));
}

/// The Brother OUI names the printer without any HTTP response, and it binds the web and LPD ports
/// it finds open.
#[test]
fn a_brother_printer_is_identified_by_its_oui_and_binds_web_and_lpd() {
    let brother_mac = "30:05:5C:00:00:01";
    let bound = definition_binds_on_host(
        "Brother Printer",
        brother_mac,
        &[PortType::Http, PortType::LdpTcp],
        &[],
    )
    .expect("a Brother printer matches");
    assert!(bound.contains(&PortType::Http) && bound.contains(&PortType::LdpTcp));
    assert!(!definition_matches_host(
        "Brother Printer",
        OTHER_MAC,
        &[PortType::Http, PortType::LdpTcp],
        &[]
    ));
    assert!(!definition_matches_host(
        "Brother Printer",
        brother_mac,
        &[],
        &[]
    ));
}

/// A web page and a Telnet console make a Switch only on a host with no port that names another
/// kind of device. The Brother HL-2270DW shows both, alongside its print ports.
#[test]
fn a_host_with_an_end_device_port_is_no_switch() {
    use crate::server::services::definitions::switch::Switch;

    let switch_matches = |extra: Option<PortType>| {
        let mut ctx = TestContext::new();
        ctx.client_responses
            .insert(super::ClientProbe::Telnet, vec![PortType::Telnet]);
        let mut ports = vec![PortType::Http, PortType::Telnet];
        ports.extend(extra);
        let baseline = ctx.create_baseline_params(&ports);
        let params = ctx.create_params_with_ports(&baseline, &ports);
        Switch.discovery_pattern().matches(&params).is_ok()
    };

    assert!(switch_matches(None), "web + Telnet alone is a switch");
    for port in [
        PortType::Ipp,
        PortType::LdpTcp,
        PortType::JetDirect,
        PortType::Rtsp,
        PortType::Sip,
        PortType::Samba,
        PortType::Nfs,
        PortType::Rdp,
    ] {
        assert!(!switch_matches(Some(port)), "{port} rules out a switch");
    }
    // A managed industrial switch serves EtherNet/IP itself.
    assert!(switch_matches(Some(PortType::EtherNetIp)));
}

/// ESP-IDF's default 404 on an Espressif MAC; the same body elsewhere, or a 200, is not it.
#[test]
fn an_esp_idf_web_server_is_identified_by_its_default_404() {
    let espressif_mac = "0C:B8:15:00:00:01";
    let ports = [PortType::Http];
    let not_found: Response = (PortType::Http, "/", 404, &[], "This URI does not exist");
    // ESP-IDF v5.0 reworded the default 404.
    let not_found_v5: Response = (
        PortType::Http,
        "/",
        404,
        &[],
        "Nothing matches the given URI",
    );
    for response in [not_found, not_found_v5] {
        assert!(definition_matches_host(
            "Espressif Device",
            espressif_mac,
            &ports,
            &[response]
        ));
    }
    assert!(!definition_matches_host(
        "Espressif Device",
        OTHER_MAC,
        &ports,
        &[not_found]
    ));
    assert!(!definition_matches_host(
        "Espressif Device",
        espressif_mac,
        &ports,
        &[(PortType::Http, "/", 200, &[], "<title>WLED</title>")]
    ));
}

/// Every `Endpoint` pattern schedules its own request, whatever status range it accepts: a ranged
/// endpoint that waits on another definition to request the same path never matches.
#[test]
fn every_endpoint_pattern_schedules_its_own_request() {
    let pattern = Pattern::Endpoint(PortType::Http, "/only-here", "x", Some(404..405));
    assert!(pattern.endpoints().iter().any(|e| e.path == "/only-here"));
}

/// Bodies a device sends for every path it is asked for, whatever the path. A Kasa plug answers
/// `/bigbluebutton/api` and `/ipa/ui` with its one page; an SPA serves its shell; some servers echo
/// the path they did not find, with a 200.
const CATCH_ALL_BODIES: &[(&str, &str)] = &[
    (
        "Kasa page",
        "<html><body><center>200 OK</center></body></html>",
    ),
    ("empty body", ""),
    (
        "SPA shell",
        "<!doctype html><html><head><title>App</title></head><body><div id=\"app\"></div></body></html>",
    ),
    ("path echo", "<html><body>No page at {path}</body></html>"),
];

/// No definition matches on what a catch-all web server says for every path. Each definition is
/// put to a host that has open exactly the ports its endpoints use and answers each request with
/// one catch-all body at 200. A match there that the same host does not give with no responses at
/// all came from the endpoint text alone, which a catch-all server cannot be trusted to mean.
#[test]
fn no_definition_matches_a_catch_all_web_server() {
    // Nokia Shanghai Bell: a vendor no definition names.
    let unlisted_mac = "28:6F:B9:00:00:01";
    let mut false_positives = Vec::new();

    for definition in ServiceDefinitionRegistry::all_service_definitions() {
        let endpoints = definition.discovery_pattern().endpoints();
        if endpoints.is_empty() {
            continue;
        }
        let mut ports: Vec<PortType> = endpoints.iter().map(|e| e.port_type).collect();
        ports.sort_by_key(|p| (p.number(), p.protocol()));
        ports.dedup();

        let matches_with = |body: Option<&str>| {
            let mut ctx = TestContext::new();
            ctx.ip_address.base.mac_address = Some(MacEvidence::new(
                MacEvidenceValue(unlisted_mac.parse().expect("valid MAC")),
                AttributeSource::ArpReply,
            ));
            ctx.pi = definition.clone();
            if let Some(body) = body {
                ctx.endpoint_responses = endpoints
                    .iter()
                    .map(|endpoint| EndpointResponse {
                        endpoint: endpoint.use_ip(ctx.ip_address.base.ip_address),
                        body: body.replace("{path}", &endpoint.path),
                        headers: HashMap::new(),
                        status: 200,
                        over_tls: false,
                        set_cookies: Vec::new(),
                    })
                    .collect();
            }
            let baseline = ctx.create_baseline_params(&ports);
            let params = ctx.create_params_with_ports(&baseline, &ports);
            ctx.pi.discovery_pattern().matches(&params).is_ok()
        };

        if matches_with(None) {
            continue;
        }
        for (shape, body) in CATCH_ALL_BODIES {
            if matches_with(Some(body)) {
                false_positives.push(format!("{} ({shape})", definition.name()));
            }
        }
    }

    assert!(
        false_positives.is_empty(),
        "definitions matching a catch-all web server: {false_positives:?}"
    );
}

/// A Pulse hub's HomeKit advertisement names its model; another HomeKit bridge's does not.
#[test]
fn a_pulse_hub_is_identified_by_its_homekit_model() {
    use crate::server::services::definitions::rollease_acmeda_pulse::RolleaseAcmedaPulse;

    let matches_with = |txt: &[&str]| {
        let mut ctx = TestContext::new();
        ctx.dns_sd = Some(announcing_on(super::DnsSdServiceType::HOMEKIT, txt, 80));
        let ports = vec![PortType::Http];
        let baseline = ctx.create_baseline_params(&ports);
        let params = ctx.create_params_with_ports(&baseline, &ports);
        RolleaseAcmedaPulse
            .discovery_pattern()
            .matches(&params)
            .ok()
    };

    // The TXT record a Pulse 2 hub advertised.
    let matched = matches_with(&["ci=2", "md=MT02-0401-067001", "pv=1.1"])
        .expect("the Pulse model string identifies the hub");
    assert_eq!(matched.ports, vec![PortType::Http]);
    assert!(matches_with(&["ci=2", "md=Hue Bridge"]).is_none());
}

/// A certificate issuer's ports go into the light scan, and an open one is still not evidence:
/// only a certificate from that issuer is.
#[test]
fn certificate_issuer_ports_are_scanned_but_are_not_evidence() {
    let pattern = Pattern::CertificateIssuer("O=Sonos", &[1443]);
    assert_eq!(pattern.ports(), vec![PortType::new_tcp(1443)]);

    let ctx = TestContext::new();
    let open = vec![PortType::new_tcp(1443)];
    let baseline = ctx.create_baseline_params(&open);
    let params = ctx.create_params_with_ports(&baseline, &open);
    assert!(
        pattern.matches(&params).is_err(),
        "an open port without a certificate is no match"
    );

    // And every definition that lists ports for an issuer puts them into the scan.
    let chromecast =
        ServiceDefinitionRegistry::find_by_id("Chromecast").expect("Chromecast is registered");
    assert!(
        chromecast
            .discovery_pattern()
            .ports()
            .contains(&PortType::new_tcp(10101))
    );
}

/// A myQ hub's setup server on 443 presents its own `setup.myqdevice.com` certificate.
#[test]
fn a_myq_hub_binds_its_setup_server_by_its_certificate() {
    let chamberlain_mac = "64:52:99:00:00:01";
    let bound = definition_binds_with_certificates(
        "myQ",
        chamberlain_mac,
        &[PortType::Http, PortType::Https],
        &[],
        &[(PortType::Https, "CN=setup.myqdevice.com,O=CGI,C=US")],
    )
    .expect("a myQ hub matches");
    assert!(bound.contains(&PortType::Http) && bound.contains(&PortType::Https));
}

fn fingerprint(
    port: PortType,
    database: crate::server::hosts::r#impl::os::recog::RecogDatabase,
    vendor: &str,
    product: &str,
) -> crate::daemon::discovery::service::network::emitted_os::PortFingerprint {
    use crate::server::hosts::r#impl::os::recog::{RecogMatch, RecogSoftware};
    crate::daemon::discovery::service::network::emitted_os::PortFingerprint {
        port,
        database,
        matched: RecogMatch {
            software: Some(RecogSoftware {
                vendor: vendor.to_string(),
                product: product.to_string(),
            }),
            ..Default::default()
        },
    }
}

/// A fingerprint naming the product binds its port; one naming another vendor's product of the
/// same name does not, and a favicon alone is weaker evidence than a title.
#[test]
fn a_recog_fingerprint_names_the_product_by_vendor_and_name() {
    use crate::server::hosts::r#impl::os::recog::RecogDatabase;
    use crate::server::services::r#impl::patterns::MatchConfidence;

    let pattern = Pattern::Recog(&[("Kubernetes", "Dashboard")], &[8443]);
    let port = PortType::new_tcp(8443);
    let open = vec![port];

    let mut ctx = TestContext::new();
    ctx.fingerprints = vec![fingerprint(
        port,
        RecogDatabase::HtmlTitle,
        "OpenStack",
        "Dashboard",
    )];
    let baseline = ctx.create_baseline_params(&open);
    let params = ctx.create_params_with_ports(&baseline, &open);
    assert!(
        pattern.matches(&params).is_err(),
        "another vendor's Dashboard"
    );

    let mut ctx = TestContext::new();
    ctx.fingerprints = vec![fingerprint(
        port,
        RecogDatabase::Favicon,
        "Kubernetes",
        "Dashboard",
    )];
    let baseline = ctx.create_baseline_params(&open);
    let params = ctx.create_params_with_ports(&baseline, &open);
    let matched = pattern.matches(&params).expect("the favicon names it");
    assert_eq!(matched.ports, vec![port]);
    assert_eq!(matched.details.confidence, MatchConfidence::Medium);

    ctx.fingerprints.push(fingerprint(
        port,
        RecogDatabase::HtmlTitle,
        "Kubernetes",
        "Dashboard",
    ));
    let baseline = ctx.create_baseline_params(&open);
    let params = ctx.create_params_with_ports(&baseline, &open);
    let matched = pattern.matches(&params).expect("the title names it");
    assert_eq!(matched.details.confidence, MatchConfidence::High);
}

/// Every product a definition matches on is one the vendored Recog data can name, so a typo or a
/// product Recog renamed in a refresh fails here instead of never matching.
#[test]
fn every_recog_product_a_definition_names_exists_in_the_vendored_data() {
    use crate::server::hosts::r#impl::os::recog::RecogDatabase;
    use strum::IntoEnumIterator;

    fn recog_pairs(pattern: &Pattern<'_>, out: &mut Vec<(&'static str, &'static str)>) {
        match pattern {
            Pattern::Recog(pairs, _) => out.extend(pairs.iter().copied()),
            Pattern::AnyOf(patterns) | Pattern::AllOf(patterns) => {
                patterns.iter().for_each(|p| recog_pairs(p, out))
            }
            Pattern::Not(inner) | Pattern::Optional(inner) => recog_pairs(inner, out),
            _ => {}
        }
    }

    let known: std::collections::HashSet<(&str, &str)> =
        RecogDatabase::iter().flat_map(|db| db.software()).collect();
    let mut missing = Vec::new();
    let mut named = 0;
    for definition in ServiceDefinitionRegistry::all_service_definitions() {
        let mut pairs = Vec::new();
        recog_pairs(&definition.discovery_pattern(), &mut pairs);
        named += pairs.len();
        for pair in pairs {
            if !known.contains(&pair) {
                missing.push(format!("{}: {pair:?}", definition.name()));
            }
        }
    }
    assert!(named > 0, "no definition matches on Recog");
    assert!(
        missing.is_empty(),
        "not in the vendored Recog data:\n{}",
        missing.join("\n")
    );
}

/// Web Service binds the ports that answered HTTP and are still unbound, and stands down once a
/// product holds them.
#[test]
fn a_web_service_takes_only_unbound_ports_that_answered_http() {
    let ctx = TestContext::new();
    let web = ServiceDefinitionRegistry::find_by_id("Web Service").expect("Web Service exists");
    // The context's one response is on 80. 2222 is open and answered nothing.
    let all_ports = vec![PortType::Http, PortType::new_tcp(2222)];
    let baseline = ctx.create_baseline_params(&all_ports);

    let params = ctx.create_params_with_ports(&baseline, &all_ports);
    let result = web
        .discovery_pattern()
        .matches(&params)
        .expect("80 answered HTTP");
    assert_eq!(result.ports, vec![PortType::Http]);

    let unbound = vec![PortType::new_tcp(2222)];
    let params = ctx.create_params_with_ports(&baseline, &unbound);
    assert!(web.discovery_pattern().matches(&params).is_err());
}

/// An optional part binds its ports when present and is never evidence: the match rests on what
/// sits beside it.
#[test]
fn an_optional_part_binds_beside_the_evidence_and_never_alone() {
    let ctx = TestContext::new();
    let pattern = Pattern::AllOf(vec![
        Pattern::Port(PortType::Http),
        Pattern::Optional(Box::new(Pattern::Port(PortType::new_tcp(1400)))),
    ]);
    let ports_bound = |open: Vec<PortType>| {
        let baseline = ctx.create_baseline_params(&open);
        let params = ctx.create_params_with_ports(&baseline, &open);
        pattern.matches(&params).ok().map(|result| result.ports)
    };

    assert_eq!(
        ports_bound(vec![PortType::Http]),
        Some(vec![PortType::Http])
    );
    assert_eq!(
        ports_bound(vec![PortType::Http, PortType::new_tcp(1400)]),
        Some(vec![PortType::Http, PortType::new_tcp(1400)])
    );
    assert_eq!(ports_bound(vec![PortType::new_tcp(1400)]), None);
}
