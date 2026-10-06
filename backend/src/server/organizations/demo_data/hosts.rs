//! Hosts and services for the demo org.
//!
//! Kept as one file rather than split further by site: this is one hand-authored host/
//! service dataset (the HQ/DC split lives in the inline comments below), not a set of
//! independent responsibilities -- splitting it would fragment a single dataset without
//! adding clarity.

use super::*;

/// A small set of "recently discovered" hosts, created AFTER the per-site
/// snapshot in the populate handler so the snapshot captures an earlier state
/// and the live view visibly differs (these hosts/services appear only in
/// live). Kept fully self-contained — no dependencies, neighbor links, or
/// interfaces — so they can be inserted after the main batch without
/// FK-ordering concerns. Uses only service-definition ids already proven in the
/// main demo set.
pub(super) fn generate_recent_hosts(
    sites: &[Site],
    subnets: &[Subnet],
    now: DateTime<Utc>,
) -> Vec<HostWithServices> {
    let find_site = |name: &str| sites.iter().find(|n| n.base.name.contains(name)).unwrap();
    let find_subnet = |name: &str| subnets.iter().find(|s| s.base.name.contains(name)).unwrap();

    let hq = find_site("Headquarters");
    let dc = find_site("Data Center");

    vec![
        // HQ: a new employee workstation on the office LAN. Windows ships OpenSSH Server as an
        // optional feature, and its banner names the OS.
        found_by_scan(host_with_services!(
            with_ssh_banner(
                create_host(
                    "hq-ws-jmartin",
                    Some("jmartin-pc.acme.local"),
                    Some("Workstation — J. Martin (new hire)"),
                    hq,
                    find_subnet("HQ Office LAN"),
                    Ipv4Addr::new(10, 0, 10, 201),
                    vec![],
                    None,
                    None,
                    now,
                ),
                WINDOWS_SSH_BANNER,
            ),
            now,
            ("Workstation", "Remote Desktop", Some(PortType::Rdp), vec![]),
            ("SSH", "SSH", Some(PortType::Ssh), vec![]),
        )),
        // HQ: a newly stood-up secondary DNS resolver.
        found_by_scan(host_with_services!(
            create_host(
                "hq-dns-secondary",
                Some("dns2.acme.local"),
                Some("Secondary DNS resolver (recently added)"),
                hq,
                find_subnet("HQ Servers"),
                Ipv4Addr::new(10, 0, 20, 201),
                vec![],
                None,
                None,
                now,
            ),
            now,
            ("Bind9", "BIND DNS", Some(PortType::DnsUdp), vec![]),
            ("SSH", "SSH", Some(PortType::Ssh), vec![]),
        )),
        // DC: a new VPN gateway.
        found_by_scan(host_with_services!(
            create_host(
                "dc-vpn-gw02",
                Some("vpn2.dc.acme.local"),
                Some("VPN gateway (recently added)"),
                dc,
                find_subnet("DC Compute"),
                Ipv4Addr::new(172, 16, 10, 201),
                vec![],
                None,
                None,
                now,
            ),
            now,
            ("OpenVPN", "OpenVPN", Some(PortType::OpenVPN), vec![]),
            ("SSH", "SSH", Some(PortType::Ssh), vec![]),
        )),
        // DC: a new compute node.
        found_by_scan(host_with_services!(
            create_host(
                "dc-node07",
                Some("node07.dc.acme.local"),
                Some("Compute node (recently added)"),
                dc,
                find_subnet("DC Compute"),
                Ipv4Addr::new(172, 16, 10, 202),
                vec![],
                None,
                None,
                now,
            ),
            now,
            ("SSH", "SSH", Some(PortType::Ssh), vec![]),
        )),
    ]
}

pub(super) fn generate_hosts_and_services(
    sites: &[Site],
    subnets: &[Subnet],
    tags: &[Tag],
    credentials: &[Credential],
    dep_svc_ids: &DependencyServiceIds,
    now: DateTime<Utc>,
) -> Vec<HostWithServices> {
    let mut result = Vec::new();

    // Helper to find entities
    let find_site = |name: &str| sites.iter().find(|n| n.base.name.contains(name)).unwrap();
    let find_subnet = |name: &str| subnets.iter().find(|s| s.base.name.contains(name)).unwrap();
    let find_tag = |name: &str| tags.iter().find(|t| t.base.name == name).map(|t| t.id);

    let network_devices_cred = credentials
        .iter()
        .find(|c| c.base.name == "Network Devices")
        .map(|c| c.id);
    let docker_proxy_cred = credentials
        .iter()
        .find(|c| c.base.name == "Docker TLS Proxy")
        .map(|c| c.id);
    let linux_inventory_cred = credentials
        .iter()
        .find(|c| c.base.name == "Linux Inventory")
        .map(|c| c.id);
    let backup_wake_cred = credentials
        .iter()
        .find(|c| c.base.name == "Backup NAS Wake")
        .map(|c| c.id);
    let find_cred = |name: &str| {
        credentials
            .iter()
            .find(|c| c.base.name == name)
            .map(|c| c.id)
    };
    let legacy_printers_cred = find_cred("Legacy Printers");
    let dc_snmpv3_cred = find_cred("Data Center SNMPv3");
    let arista_gnmi_cred = find_cred("Arista gNMI");
    let podman_proxy_cred = find_cred("Podman API Proxy");
    let unifi_admin_cred = find_cred("UniFi Controller Admin");
    let unifi_os_cred = find_cred("UniFi OS API Key");
    let instant_on_cred = find_cred("Instant On Cloud Account");
    let hypervisor_ssh_cred = find_cred("Hypervisor SSH");
    let proxmox_api_cred = find_cred("Proxmox Cluster API");

    let critical_tag = find_tag("Critical");
    let production_tag = find_tag("Production");
    let development_tag = find_tag("Development");
    let decommissioned_tag = find_tag("Decommissioned");
    let database_tag = find_tag("Database");
    let monitoring_tag = find_tag("Monitoring");
    let iot_tag = find_tag("IoT Device");
    let web_tier_tag = find_tag("Web Tier");
    let backup_tag = find_tag("Backup Target");
    let devops_tag = find_tag("DevOps Pipeline");
    let storage_tag = find_tag("Storage");
    let messaging_tag = find_tag("Messaging");

    // Pre-generated service UUIDs for virtualization wiring
    let pve_hq1_svc_id = Uuid::new_v4(); // Proxmox VE on proxmox-hv01
    let pve_hq2_svc_id = Uuid::new_v4(); // Proxmox VE on proxmox-hv02
    let docker_hq_svc_id = dep_svc_ids.docker_hq; // Docker daemon on docker-prod01 (shared with HQ Docker Bridge subnet)
    let pve_dc_svc_id = Uuid::new_v4(); // Proxmox VE on dc-proxmox-hv01
    let docker_dc_svc_id = dep_svc_ids.docker_dc; // Docker daemon on dc-docker01 (shared with DC Docker Bridge subnet)

    // Service UUIDs for HubAndSpoke dependency wiring (pre-generated at top level)
    let prometheus_hq_svc_id = dep_svc_ids.prometheus_hq;
    let grafana_hq_svc_id = dep_svc_ids.grafana_hq;
    let uptime_kuma_svc_id = dep_svc_ids.uptime_kuma;
    // Binding UUIDs for RequestPath dependency wiring (pre-generated at top level)
    let traefik_hq_binding_id = dep_svc_ids.traefik_hq_binding;
    let gitea_hq_binding_id = dep_svc_ids.gitea_hq_binding;
    let haproxy_dc_binding_id = dep_svc_ids.haproxy_dc_binding;
    let app01_dc_binding_id = dep_svc_ids.app01_dc_binding;
    let mariadb_dc_binding_id = dep_svc_ids.mariadb_dc_binding;
    let pve_hq1_binding_id = dep_svc_ids.pve_hq1_binding;
    let truenas_binding_id = dep_svc_ids.truenas_binding;
    // Service UUIDs for DC HubAndSpoke dependency wiring
    let prometheus_dc_svc_id = dep_svc_ids.prometheus_dc;
    let grafana_dc_svc_id = dep_svc_ids.grafana_dc;
    let jaeger_dc_svc_id = dep_svc_ids.jaeger_dc;
    let minio_dc_svc_id = dep_svc_ids.minio_dc;
    let ceph_dc_svc_id = dep_svc_ids.ceph_dc;
    let elasticsearch_dc_svc_id = dep_svc_ids.elasticsearch_dc;

    // ========================================================================
    // HEADQUARTERS SITE — 30 hosts
    // ========================================================================
    let hq = find_site("Headquarters");
    let hq_mgmt = find_subnet("HQ Management");
    let hq_servers = find_subnet("HQ Servers");
    let hq_storage = find_subnet("HQ Storage");
    let hq_lan = find_subnet("HQ Office LAN");
    let hq_iot = find_subnet("HQ IoT");
    let hq_docker = find_subnet("HQ Docker Bridge");
    let hq_guest = find_subnet("HQ Guest WiFi");

    // -- Management (10.0.1.x) --

    // 1. pfSense Firewall (Critical) — with host-level SNMP credential override
    let mut pfsense = {
        let (host, ip_address) = with_snmp(
            with_mac(
                create_host(
                    "pfsense-fw01",
                    Some("pfsense-fw01.acme.local"),
                    Some("Primary pfSense firewall"),
                    hq,
                    hq_mgmt,
                    Ipv4Addr::new(10, 0, 1, 1),
                    critical_tag.into_iter().collect(),
                    network_devices_cred,
                    None,
                    now,
                ),
                [0xa4, 0xbe, 0x2b, 0x10, 0x01, 0x01],
            ),
            Some("pfSense pfsense-fw01.acme.local 2.7.0-RELEASE FreeBSD 14.0-CURRENT amd64"),
            Some("1.3.6.1.4.1.12325.1.1"),
            Some("HQ Server Room, Rack A1"),
            Some("netops@acme-corp.com"),
            None,
            Some("Netgate"),
            Some("SG-3100"),
            Some("NG61003370A1F4"),
        );
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((svc, port)) = create_service(
            "pfSense",
            "pfSense",
            &host,
            &ip_addresses[0],
            Some(PortType::Https),
            critical_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        HostWithServices {
            host,
            ip_addresses,
            ports,
            services,
        }
    };
    pfsense.host.base.credential_assignments = network_devices_cred
        .into_iter()
        .map(|id| CredentialAssignment {
            credential_id: id,
            ip_address_ids: None,
        })
        .collect();
    result.push(pfsense);

    // 2. UniFi Controller: the self-hosted Network application, on its legacy 8443 port.
    result.push(with_credentials(
        host_with_services!(
            with_mac(
                create_host(
                    "unifi-controller",
                    Some("unifi.acme.local"),
                    Some("UniFi Network Controller"),
                    hq,
                    hq_mgmt,
                    Ipv4Addr::new(10, 0, 1, 10),
                    vec![],
                    None,
                    None,
                    now
                ),
                [0xfc, 0xec, 0xda, 0x10, 0x02, 0x01],
            ),
            now,
            (
                "UniFi Controller",
                "UniFi Controller",
                Some(PortType::Https8443),
                vec![]
            ),
        ),
        &[unifi_admin_cred],
    ));

    // 2b. UniFi OS console: a Cloud Key running the Network application under UniFi OS, which
    // serves it on 443 and accepts API keys.
    result.push(with_credentials(
        host_with_services!(
            with_mac(
                create_host(
                    "unifi-cloudkey",
                    Some("cloudkey.acme.local"),
                    Some("UniFi Cloud Key Gen2 Plus"),
                    hq,
                    hq_mgmt,
                    Ipv4Addr::new(10, 0, 1, 11),
                    vec![],
                    None,
                    None,
                    now
                ),
                [0xfc, 0xec, 0xda, 0x10, 0x02, 0x02],
            ),
            now,
            (
                "UniFi Controller",
                "UniFi OS Console",
                Some(PortType::Https),
                vec![]
            ),
        ),
        &[unifi_os_cred],
    ));

    // 3. Core switch (48 ports, SNMP/LLDP) — ENTITY-MIB reports bootloader and OS firmware
    // separately, so this is the one device that shows the split rather than a single blank.
    let mut unifi_switch = host_with_services!(
        with_snmp(
            with_mac(
                create_host(
                    "unifi-usw-48",
                    Some("switch.acme.local"),
                    Some("UniFi Switch 48 PoE"),
                    hq,
                    hq_mgmt,
                    Ipv4Addr::new(10, 0, 1, 3),
                    vec![],
                    network_devices_cred,
                    None,
                    now,
                ),
                [0xfc, 0xec, 0xda, 0x10, 0x03, 0x01],
            ),
            Some("UniFi USW-48-PoE, 6.6.65, Linux 5.4.0"),
            Some("1.3.6.1.4.1.41112.1.6"),
            Some("HQ Server Room, Rack A2"),
            Some("netops@acme-corp.com"),
            Some("78:45:c4:ab:cd:01"),
            Some("Ubiquiti"),
            Some("USW-48-PoE"),
            Some("UI7845C4ABCD01"),
        ),
        now,
        ("SNMP", "SNMP", Some(PortType::Snmp), vec![]),
    );
    let switch_revision_source = AttributeSource::Probe(ClientProbe::Snmp);
    unifi_switch.host.base.firmware_revision = Some(Attributed::new(
        HostFirmwareRevisionValue("1.0.5".to_string()),
        switch_revision_source,
    ));
    unifi_switch.host.base.software_revision = Some(Attributed::new(
        HostSoftwareRevisionValue("6.6.65".to_string()),
        switch_revision_source,
    ));
    result.push(unifi_switch);

    // 4. Pi-hole DNS: a macvlan container on docker-prod01, listed after it (13b).

    // 5. Grafana (pre-generated ID for dependency wiring)
    {
        let (host, ip_address) = with_mac(
            create_host(
                "grafana-mon",
                Some("grafana.acme.local"),
                Some("Grafana monitoring dashboard"),
                hq,
                hq_mgmt,
                Ipv4Addr::new(10, 0, 1, 50),
                monitoring_tag.into_iter().collect(),
                None,
                None,
                now,
            ),
            [0xf8, 0xbc, 0x12, 0x10, 0x05, 0x01],
        );
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((svc, port)) = create_service_with_id(
            grafana_hq_svc_id,
            "Grafana",
            "Grafana",
            &host,
            &ip_addresses[0],
            Some(PortType::Http3000),
            monitoring_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            ip_addresses,
            ports,
            services,
        });
    }

    // 6. Prometheus (pre-generated ID for dependency wiring)
    {
        let (host, ip_address) = with_mac(
            create_host(
                "prometheus",
                Some("prometheus.acme.local"),
                Some("Prometheus metrics server"),
                hq,
                hq_mgmt,
                Ipv4Addr::new(10, 0, 1, 51),
                monitoring_tag.into_iter().collect(),
                None,
                None,
                now,
            ),
            [0xf8, 0xbc, 0x12, 0x10, 0x06, 0x01],
        );
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((svc, port)) = create_service_with_id(
            prometheus_hq_svc_id,
            "Prometheus",
            "Prometheus",
            &host,
            &ip_addresses[0],
            Some(PortType::Http9000),
            monitoring_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            ip_addresses,
            ports,
            services,
        });
    }

    // 7. Uptime Kuma (pre-generated ID for dependency wiring) — an LXC container on hv02, bridged
    // onto the management VLAN it watches (vm_id=202)
    {
        let (host, ip_address) = with_mac(
            reported_by_proxmox(create_host(
                "uptime-kuma",
                Some("status.acme.local"),
                Some("Uptime Kuma status page (LXC container on proxmox-hv02)"),
                hq,
                hq_mgmt,
                Ipv4Addr::new(10, 0, 1, 52),
                monitoring_tag.into_iter().collect(),
                None,
                Some((
                    HostVirtualization::Proxmox(ProxmoxVirtualization {
                        vm_name: Some("uptime-kuma".to_string()),
                        vm_id: Some("202".to_string()),
                        guest_type: Some(ProxmoxGuestType::Lxc),
                    }),
                    pve_hq2_svc_id,
                )),
                now,
            )),
            [0xbc, 0x24, 0x11, 0x10, 0x07, 0x01],
        );
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((svc, port)) = create_service_with_id(
            uptime_kuma_svc_id,
            "UptimeKuma",
            "Uptime Kuma",
            &host,
            &ip_addresses[0],
            Some(PortType::Http3000),
            monitoring_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            ip_addresses,
            ports,
            services,
        });
    }

    // -- Servers (10.0.20.x) — hypervisors, VMs, Docker --

    // 8. Proxmox Hypervisor 1 (pre-generated Proxmox VE service ID)
    {
        let (host, ip_address) = with_snmp(
            with_mac(
                create_host(
                    "proxmox-hv01",
                    Some("proxmox-hv01.acme.local"),
                    Some("Proxmox hypervisor node 1"),
                    hq,
                    hq_servers,
                    Ipv4Addr::new(10, 0, 20, 5),
                    production_tag.into_iter().collect(),
                    None,
                    None,
                    now,
                ),
                [0xf8, 0xbc, 0x12, 0x20, 0x08, 0x01],
            ),
            Some("Linux proxmox-hv01 6.8.12-1-pve #1 SMP PVE 6.8.12-1 x86_64"),
            Some("1.3.6.1.4.1.8072.3.2.10"),
            Some("HQ Server Room, Rack B1"),
            Some("sysadmin@acme-corp.com"),
            None,
            Some("Dell Inc."),
            Some("PowerEdge R740"),
            Some("DL7QX2B1PVE1"),
        );
        // Proxmox VE 8 is Debian 12, and its OpenSSH package says so in the banner.
        let (host, ip_address) = with_ssh_banner((host, ip_address), PROXMOX_SSH_BANNER);
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((mut svc, port)) = create_service_with_id(
            pve_hq1_svc_id,
            "Proxmox VE",
            "Proxmox VE",
            &host,
            &ip_addresses[0],
            Some(PortType::new_tcp(8006)),
            production_tag.into_iter().collect(),
            now,
        ) {
            svc.base.bindings[0].id = pve_hq1_binding_id;
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        if let Some((svc, port)) = create_service(
            "SSH",
            "SSH",
            &host,
            &ip_addresses[0],
            Some(PortType::Ssh),
            vec![],
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(with_credentials(
            HostWithServices {
                host,
                ip_addresses,
                ports,
                services,
            },
            &[hypervisor_ssh_cred, proxmox_api_cred],
        ));
    }

    // 9. Proxmox Hypervisor 2 (pre-generated Proxmox VE service ID)
    {
        let (host, ip_address) = with_snmp(
            with_mac(
                create_host(
                    "proxmox-hv02",
                    Some("proxmox-hv02.acme.local"),
                    Some("Proxmox hypervisor node 2"),
                    hq,
                    hq_servers,
                    Ipv4Addr::new(10, 0, 20, 6),
                    production_tag.into_iter().collect(),
                    None,
                    None,
                    now,
                ),
                [0xf8, 0xbc, 0x12, 0x20, 0x09, 0x01],
            ),
            Some("Linux proxmox-hv02 6.8.12-1-pve #1 SMP PVE 6.8.12-1 x86_64"),
            Some("1.3.6.1.4.1.8072.3.2.10"),
            Some("HQ Server Room, Rack B2"),
            Some("sysadmin@acme-corp.com"),
            None,
            Some("Supermicro"),
            Some("AS-2124BT-HNTR"),
            Some("SMC2124B2PVE2"),
        );
        let (host, ip_address) = with_ssh_banner((host, ip_address), PROXMOX_SSH_BANNER);
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((svc, port)) = create_service_with_id(
            pve_hq2_svc_id,
            "Proxmox VE",
            "Proxmox VE",
            &host,
            &ip_addresses[0],
            Some(PortType::new_tcp(8006)),
            production_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        if let Some((svc, port)) = create_service(
            "SSH",
            "SSH",
            &host,
            &ip_addresses[0],
            Some(PortType::Ssh),
            vec![],
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(with_credentials(
            HostWithServices {
                host,
                ip_addresses,
                ports,
                services,
            },
            &[hypervisor_ssh_cred, proxmox_api_cred],
        ));
    }

    // 10. gitlab-vm — VM on hv01 (vm_id=100). Also runs a Docker engine for its CI runner, which
    // sits on a macvlan network with an address of its own (10b), so proxmox-hv01 → gitlab-vm →
    // gitlab-runner is a three-level virtualization chain.
    let docker_gitlab_svc_id = Uuid::new_v4();
    let mut gitlab_vm = host_with_services!(
        with_mac(
            reported_by_proxmox(create_host(
                "gitlab-vm",
                Some("gitlab.acme.local"),
                Some("GitLab instance (VM on proxmox-hv01)"),
                hq,
                hq_servers,
                Ipv4Addr::new(10, 0, 20, 10),
                production_tag.into_iter().collect(),
                None,
                Some((
                    HostVirtualization::Proxmox(ProxmoxVirtualization {
                        vm_name: Some("gitlab-vm".to_string()),
                        vm_id: Some("100".to_string()),
                        guest_type: Some(ProxmoxGuestType::Qemu),
                    }),
                    pve_hq1_svc_id,
                )),
                now
            )),
            [0x52, 0x54, 0x00, 0x20, 0x10, 0x01],
        ),
        now,
        (
            "GitLab",
            "GitLab",
            Some(PortType::Https),
            [production_tag, devops_tag].into_iter().flatten().collect()
        ),
    );
    if let Some((svc, port)) = create_service_with_id(
        docker_gitlab_svc_id,
        "Docker",
        "Docker Daemon",
        &gitlab_vm.host,
        &gitlab_vm.ip_addresses[0],
        Some(PortType::Docker),
        vec![],
        now,
    ) {
        if let Some(p) = port {
            gitlab_vm.ports.push(p);
        }
        gitlab_vm.services.push(svc);
    }
    result.push(gitlab_vm);

    // 10b. gitlab-runner — the CI runner container on gitlab-vm's macvlan network
    {
        let (mut host, mut ip_address) = create_host(
            "gitlab-runner",
            Some("runner.acme.local"),
            Some("GitLab CI runner (macvlan container on gitlab-vm)"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 13),
            [devops_tag].into_iter().flatten().collect(),
            None,
            Some((
                HostVirtualization::Docker(ContainerHostVirtualization {
                    container_name: Some("gitlab-runner".to_string()),
                    container_id: Some("7b1c2d3e4f5a".to_string()),
                    compose_project: Some("ci".to_string()),
                    network_type: ContainerNetworkType::MacVlan,
                }),
                docker_gitlab_svc_id,
            )),
            now,
        );
        // Discovered and titled by the Docker integration, as the daemon records a container host.
        host.base.source = EntitySource::Discovery;
        host.base.name =
            HostName::from_controller("gitlab-runner".to_string(), ClientProbe::Docker);
        ip_address.base.mac_address = Some(MacEvidence::new(
            MacEvidenceValue(MacAddress::new([0x02, 0x42, 0x0a, 0x00, 0x14, 0x0d])),
            AttributeSource::HypervisorConfig,
        ));
        ip_address.base.name = Some("lan".to_string());
        result.push(HostWithServices {
            host,
            ip_addresses: vec![ip_address],
            ports: vec![],
            services: vec![],
        });
    }

    // 11. nextcloud-vm — VM on hv01 (vm_id=101)
    result.push(host_with_services!(
        with_mac(
            reported_by_proxmox(create_host(
                "nextcloud-vm",
                Some("cloud.acme.local"),
                Some("Nextcloud file sharing (VM on proxmox-hv01)"),
                hq,
                hq_servers,
                Ipv4Addr::new(10, 0, 20, 11),
                production_tag.into_iter().collect(),
                None,
                Some((
                    HostVirtualization::Proxmox(ProxmoxVirtualization {
                        vm_name: Some("nextcloud-vm".to_string()),
                        vm_id: Some("101".to_string()),
                        guest_type: Some(ProxmoxGuestType::Qemu),
                    }),
                    pve_hq1_svc_id,
                )),
                now
            )),
            [0x52, 0x54, 0x00, 0x20, 0x11, 0x01],
        ),
        now,
        (
            "NextCloud",
            "Nextcloud",
            Some(PortType::Https),
            [production_tag, web_tier_tag]
                .into_iter()
                .flatten()
                .collect()
        ),
    ));

    // 12. keycloak-vm — VM on hv02 (vm_id=200)
    result.push(host_with_services!(
        with_mac(
            reported_by_proxmox(create_host(
                "keycloak-vm",
                Some("keycloak.acme.local"),
                Some("Keycloak SSO (VM on proxmox-hv02)"),
                hq,
                hq_servers,
                Ipv4Addr::new(10, 0, 20, 12),
                production_tag.into_iter().collect(),
                None,
                Some((
                    HostVirtualization::Proxmox(ProxmoxVirtualization {
                        vm_name: Some("keycloak-vm".to_string()),
                        vm_id: Some("200".to_string()),
                        guest_type: Some(ProxmoxGuestType::Qemu),
                    }),
                    pve_hq2_svc_id,
                )),
                now
            )),
            [0x52, 0x54, 0x00, 0x20, 0x12, 0x01],
        ),
        now,
        (
            "Keycloak",
            "Keycloak",
            Some(PortType::Https8443),
            production_tag.into_iter().collect()
        ),
    ));

    // 13. docker-prod01 — Docker host (2 ip_addresses: eth0 on Servers, docker0 on DockerBridge)
    {
        let host_id = Uuid::new_v4();
        let eth0 = IPAddress {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IPAddressBase {
                site_id: hq.id,
                host_id,
                subnet_id: hq_servers.id,
                ip_address: IpAddr::V4(Ipv4Addr::new(10, 0, 20, 20)),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xf8, 0xbc, 0x12, 0x20, 0x13, 0x01])),
                    AttributeSource::ArpReply,
                )),
                name: Some("eth0".to_string()),
                position: 0,
            },
        };
        let docker0 = IPAddress {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IPAddressBase {
                site_id: hq.id,
                host_id,
                subnet_id: hq_docker.id,
                ip_address: IpAddr::V4(Ipv4Addr::new(172, 17, 0, 1)),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0x02, 0x42, 0xac, 0x11, 0x00, 0x01])),
                    AttributeSource::ArpReply,
                )),
                name: Some("docker0".to_string()),
                position: 1,
            },
        };
        let host = Host {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: host_id,
            created_at: now,
            updated_at: now,
            base: HostBase {
                name: HostName::manual("docker-prod01".to_string()),
                site_id: hq.id,
                hostname: Some(Attributed::new(
                    crate::server::hosts::r#impl::attributes::HostHostnameValue(
                        "docker-prod01.acme.local".to_string(),
                    ),
                    AttributeSource::ReverseDns,
                )),
                description: Some("Production Docker host".to_string()),
                source: EntitySource::Manual,
                virtualization_metadata: None,
                virtualization_service_id: None,
                virtualization_interface_id: None,
                hidden: false,
                tags: production_tag.into_iter().collect(),
                sys_descr: None,
                sys_object_id: None,
                sys_location: None,
                sys_contact: None,
                management_url: None,
                chassis_id: None,
                sys_name: None,
                manufacturer: None,
                model: None,
                serial_number: None,
                asset_tag: None,
                firmware_revision: None,
                software_revision: None,
                os: docker_engine_host_os(),
                credential_assignments: docker_proxy_cred
                    .into_iter()
                    .map(|id| CredentialAssignment {
                        credential_id: id,
                        ip_address_ids: None,
                    })
                    .collect(),
            },
        };

        let mut ports = Vec::new();
        let mut services = Vec::new();

        // Docker Daemon service with pre-generated ID on eth0
        if let Some((svc, port)) = create_service_with_id(
            docker_hq_svc_id,
            "Docker",
            "Docker Daemon",
            &host,
            &eth0,
            Some(PortType::Docker),
            vec![],
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        // Portainer on eth0
        if let Some((svc, port)) = create_service(
            "Portainer",
            "Portainer",
            &host,
            &eth0,
            Some(PortType::Http9000),
            [production_tag, devops_tag].into_iter().flatten().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }

        // Traefik container (pre-generated binding ID for dependency wiring)
        if let Some((mut svc, port)) = create_container_service(
            "Traefik",
            "Traefik",
            &host,
            &docker0,
            Some(PortType::Https),
            "traefik",
            "a1b2c3d4e5f6",
            docker_hq_svc_id,
            web_tier_tag.into_iter().collect(),
            now,
        ) {
            svc.base.bindings[0].id = traefik_hq_binding_id;
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        // Gitea container (pre-generated binding ID for dependency wiring)
        if let Some((mut svc, port)) = create_container_service(
            "Gitea",
            "Gitea",
            &host,
            &docker0,
            Some(PortType::Http3000),
            "gitea",
            "g7h8i9j0k1l2",
            docker_hq_svc_id,
            devops_tag.into_iter().collect(),
            now,
        ) {
            svc.base.bindings[0].id = gitea_hq_binding_id;
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        // Other container services on docker0
        for (def_id, name, pt, cname, cid, tags) in [
            (
                "Vaultwarden",
                "Vaultwarden",
                Some(PortType::Http8080),
                "vaultwarden",
                "d4e5f6a7b8c9",
                devops_tag.into_iter().collect::<Vec<_>>(),
            ),
            (
                "mailcow",
                "mailcow",
                Some(PortType::Https8443),
                "mailcow",
                "j0k1l2m3n4o5",
                messaging_tag.into_iter().collect::<Vec<_>>(),
            ),
        ] {
            if let Some((svc, port)) = create_container_service(
                def_id,
                name,
                &host,
                &docker0,
                pt,
                cname,
                cid,
                docker_hq_svc_id,
                tags,
                now,
            ) {
                if let Some(p) = port {
                    ports.push(p);
                }
                services.push(svc);
            }
        }

        result.push(HostWithServices {
            host,
            ip_addresses: vec![eth0, docker0],
            ports,
            services,
        });
    }

    // 13b. Pi-hole DNS: a Compose-managed container on docker-prod01's macvlan network, so it
    // answers DNS from its own address on the Servers LAN rather than through a published port.
    // Docker assigns the endpoint's MAC (02:42 + the IP) and keeps it for the container's life.
    {
        let (mut host, mut ip_address) = create_host(
            "pihole",
            Some("pihole.acme.local"),
            Some("Pi-hole DNS ad blocker (macvlan container on docker-prod01)"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 21),
            vec![],
            None,
            Some((
                HostVirtualization::Docker(ContainerHostVirtualization {
                    container_name: Some("pihole".to_string()),
                    container_id: Some("e3f4a5b6c7d8".to_string()),
                    compose_project: Some("dns".to_string()),
                    network_type: ContainerNetworkType::MacVlan,
                }),
                docker_hq_svc_id,
            )),
            now,
        );
        // Discovered and titled by the Docker integration, as the daemon records a container host.
        host.base.source = EntitySource::Discovery;
        host.base.name = HostName::from_controller("pihole".to_string(), ClientProbe::Docker);
        ip_address.base.mac_address = Some(MacEvidence::new(
            MacEvidenceValue(MacAddress::new([0x02, 0x42, 0x0a, 0x00, 0x14, 0x15])),
            AttributeSource::HypervisorConfig,
        ));
        ip_address.base.name = Some("lan".to_string());
        result.push(host_with_services!(
            (host, ip_address),
            now,
            ("Pi-Hole", "Pi-hole", Some(PortType::Http), vec![]),
        ));
    }

    // 14. Jenkins CI — inventoried by the "Linux Inventory" SSH credential's script
    let mut jenkins = host_with_services!(
        with_ssh_inventory(
            with_mac(
                create_host(
                    "jenkins-ci",
                    Some("jenkins.acme.local"),
                    Some("Jenkins CI/CD server"),
                    hq,
                    hq_servers,
                    Ipv4Addr::new(10, 0, 20, 30),
                    development_tag.into_iter().collect(),
                    None,
                    None,
                    now
                ),
                [0xf8, 0xbc, 0x12, 0x20, 0x14, 0x01],
            ),
            "Ubuntu 24.04.1 LTS 6.8.0-45-generic",
            "Dell Inc.",
            "PowerEdge R250",
            "7QX2KT3",
            "1.11.2",
            HostOs {
                family: HostOsFamily::Linux,
                name: Some("Ubuntu".to_string()),
                version: Some("24.04.1".to_string()),
                edition: Some("LTS".to_string()),
                codename: Some("noble".to_string()),
                kernel_version: Some("6.8.0-45-generic".to_string()),
            },
        ),
        now,
        (
            "Jenkins",
            "Jenkins",
            Some(PortType::Http8080),
            [development_tag, devops_tag]
                .into_iter()
                .flatten()
                .collect()
        ),
        ("SSH", "SSH", Some(PortType::Ssh), vec![]),
    );
    jenkins.host.base.credential_assignments = linux_inventory_cred
        .into_iter()
        .map(|id| CredentialAssignment {
            credential_id: id,
            ip_address_ids: None,
        })
        .collect();
    result.push(jenkins);

    // 15. WireGuard VPN
    result.push(host_with_services!(
        with_mac(
            create_host(
                "wireguard-vpn",
                Some("vpn.acme.local"),
                Some("WireGuard VPN server"),
                hq,
                hq_servers,
                Ipv4Addr::new(10, 0, 20, 35),
                vec![],
                None,
                None,
                now
            ),
            [0xf8, 0xbc, 0x12, 0x20, 0x15, 0x01],
        ),
        now,
        (
            "WireGuard",
            "WireGuard VPN",
            Some(PortType::Wireguard),
            vec![]
        ),
    ));

    // -- Storage (10.0.40.x) --

    // 16. db-vm — VM on hv02 (vm_id=201)
    result.push(host_with_services!(
        with_mac(
            reported_by_proxmox(create_host(
                "db-vm",
                Some("db.acme.local"),
                Some("Database server (VM on proxmox-hv02)"),
                hq,
                hq_storage,
                Ipv4Addr::new(10, 0, 40, 10),
                database_tag.into_iter().chain(critical_tag).collect(),
                None,
                Some((
                    HostVirtualization::Proxmox(ProxmoxVirtualization {
                        vm_name: Some("db-vm".to_string()),
                        vm_id: Some("201".to_string()),
                        guest_type: Some(ProxmoxGuestType::Qemu),
                    }),
                    pve_hq2_svc_id,
                )),
                now
            )),
            [0x52, 0x54, 0x00, 0x40, 0x16, 0x01],
        ),
        now,
        (
            "PostgreSQL",
            "PostgreSQL",
            Some(PortType::PostgreSQL),
            database_tag.into_iter().collect()
        ),
        (
            "Redis",
            "Redis",
            Some(PortType::Redis),
            database_tag.into_iter().collect()
        ),
    ));

    // 17. TrueNAS Primary (pre-generated binding ID for Backup Flow dependency)
    {
        let (host, ip_address) = with_snmp(
            with_mac(
                create_host(
                    "truenas-primary",
                    Some("truenas.acme.local"),
                    Some("Primary NAS storage"),
                    hq,
                    hq_storage,
                    Ipv4Addr::new(10, 0, 40, 20),
                    critical_tag.into_iter().chain(backup_tag).collect(),
                    None,
                    None,
                    now,
                ),
                [0xd0, 0x50, 0x99, 0x40, 0x17, 0x01],
            ),
            Some("TrueNAS SCALE 24.04 (Dragonfish) - Kernel 6.6.44-production+truenas"),
            Some("1.3.6.1.4.1.50536.3"),
            Some("HQ Server Room, Rack C1"),
            Some("sysadmin@acme-corp.com"),
            None,
            Some("iXsystems"),
            Some("TrueNAS Mini X+"),
            Some("IX-TNMXP-2231C0087"),
        );
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((mut svc, port)) = create_service(
            "TrueNAS",
            "TrueNAS",
            &host,
            &ip_addresses[0],
            Some(PortType::Https),
            [backup_tag, storage_tag].into_iter().flatten().collect(),
            now,
        ) {
            svc.base.bindings[0].id = truenas_binding_id;
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        if let Some((svc, port)) = create_service(
            "NFS",
            "NFS",
            &host,
            &ip_addresses[0],
            Some(PortType::Nfs),
            storage_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            ip_addresses,
            ports,
            services,
        });
    }

    // 18. Synology Backup — powered on weekly for backups; Wake-on-LAN wakes it before the scan
    let mut synology = host_with_services!(
        with_snmp(
            with_mac(
                create_host(
                    "synology-backup",
                    Some("synology.acme.local"),
                    Some("Synology backup NAS"),
                    hq,
                    hq_storage,
                    Ipv4Addr::new(10, 0, 40, 21),
                    backup_tag.into_iter().collect(),
                    None,
                    None,
                    now,
                ),
                [0x00, 0x11, 0x32, 0x40, 0x18, 0x01],
            ),
            Some("Synology NAS DS1621+ DSM 7.2.1-69057 Update 5"),
            Some("1.3.6.1.4.1.6574.1"),
            Some("HQ Server Room, Rack C2"),
            Some("sysadmin@acme-corp.com"),
            None,
            Some("Synology"),
            Some("DS1621+"),
            Some("2140PDN6C201"),
        ),
        now,
        (
            "Synology DSM",
            "Synology DSM",
            Some(PortType::Https),
            [backup_tag, storage_tag].into_iter().flatten().collect()
        ),
    );
    synology.host.base.credential_assignments = backup_wake_cred
        .into_iter()
        .map(|id| CredentialAssignment {
            credential_id: id,
            ip_address_ids: None,
        })
        .collect();
    result.push(synology);

    // -- Office LAN (10.0.10.x) --

    // 19-22. Workstations
    for (name, hostname, desc, ip_last, mac_last) in [
        (
            "ws-engineering-01",
            "ws-eng-01.acme.local",
            "Engineering workstation 1",
            101,
            0x19u8,
        ),
        (
            "ws-engineering-02",
            "ws-eng-02.acme.local",
            "Engineering workstation 2",
            102,
            0x20,
        ),
        (
            "ws-accounting-01",
            "ws-acct-01.acme.local",
            "Accounting workstation",
            103,
            0x21,
        ),
        // Never named in Scanopy, so it is titled by the hostname its PTR record gives it.
        ("", "ws-hr-01.acme.local", "HR workstation", 104, 0x22),
    ] {
        let host = with_mac(
            create_host(
                name,
                Some(hostname),
                Some(desc),
                hq,
                hq_lan,
                Ipv4Addr::new(10, 0, 10, ip_last),
                vec![],
                None,
                None,
                now,
            ),
            [0xf8, 0xbc, 0x12, 0x10, mac_last, 0x01],
        );
        let host = if name.is_empty() { unnamed(host) } else { host };
        let hws = host_with_services!(
            host,
            now,
            ("Workstation", "Workstation", Some(PortType::Rdp), vec![]),
        );
        // The accounting workstation was retired and never came back on the LAN.
        result.push(if name == "ws-accounting-01" {
            gone_quiet(hws, now - Duration::days(1000), now - Duration::days(380))
        } else {
            hws
        });
    }

    // -- IoT (10.0.30.x) --

    // 23. UniFi AP Lobby
    result.push(host_with_services!(
        with_snmp(
            with_mac(
                create_host(
                    "unifi-ap-lobby",
                    Some("ap-lobby.acme.local"),
                    Some("UniFi AP - Main Lobby"),
                    hq,
                    hq_iot,
                    Ipv4Addr::new(10, 0, 30, 100),
                    iot_tag.into_iter().collect(),
                    network_devices_cred,
                    None,
                    now,
                ),
                [0xfc, 0xec, 0xda, 0x30, 0x23, 0x01],
            ),
            Some("UniFi U6-Pro, 6.6.65, Linux 5.4.0"),
            Some("1.3.6.1.4.1.41112.1.6"),
            Some("HQ Main Lobby, Ceiling Mount"),
            Some("netops@acme-corp.com"),
            Some("fc:ec:da:aa:bb:01"),
            Some("Ubiquiti"),
            Some("U6-Pro"),
            Some("UIFCECDAAABB01"),
        ),
        now,
        (
            "Unifi Access Point",
            "UniFi AP",
            None,
            iot_tag.into_iter().collect()
        ),
    ));

    // 24. UniFi AP Floor 2
    result.push(host_with_services!(
        with_snmp(
            with_mac(
                create_host(
                    "unifi-ap-floor2",
                    Some("ap-floor2.acme.local"),
                    Some("UniFi AP - Floor 2"),
                    hq,
                    hq_iot,
                    Ipv4Addr::new(10, 0, 30, 101),
                    iot_tag.into_iter().collect(),
                    network_devices_cred,
                    None,
                    now,
                ),
                [0xfc, 0xec, 0xda, 0x30, 0x24, 0x01],
            ),
            Some("UniFi U6-LR, 6.6.65, Linux 5.4.0"),
            Some("1.3.6.1.4.1.41112.1.6"),
            Some("HQ Floor 2, Hallway Ceiling"),
            Some("netops@acme-corp.com"),
            Some("fc:ec:da:aa:bb:02"),
            Some("Ubiquiti"),
            Some("U6-LR"),
            Some("UIFCECDAAABB02"),
        ),
        now,
        (
            "Unifi Access Point",
            "UniFi AP",
            None,
            iot_tag.into_iter().collect()
        ),
    ));

    // 25. Hue Bridge. Hidden from the topology: the lobby lighting has no bearing on the network.
    let mut hue_bridge = host_with_services!(
        with_mac(
            create_host(
                "hue-bridge",
                None,
                Some("Philips Hue Bridge"),
                hq,
                hq_iot,
                Ipv4Addr::new(10, 0, 30, 10),
                iot_tag.into_iter().collect(),
                None,
                None,
                now
            ),
            [0x00, 0x17, 0x88, 0x30, 0x25, 0x01],
        ),
        now,
        (
            "Philips Hue Bridge",
            "Philips Hue",
            Some(PortType::Https),
            iot_tag.into_iter().collect()
        ),
    );
    hue_bridge.host.base.hidden = true;
    result.push(hue_bridge);

    // 26. HP Printer. Never named in Scanopy, so it is titled by the sysName it reports. Unplugged
    // when the copy room got a replacement; it has not answered a scan since. Old enough to speak
    // only SNMPv1.
    result.push(with_credentials(
        gone_quiet(
            host_with_services!(
                unnamed(with_snmp(
                    with_mac(
                        create_host(
                            "printer-hp-main",
                            None,
                            Some("HP LaserJet Pro"),
                            hq,
                            hq_iot,
                            Ipv4Addr::new(10, 0, 30, 50),
                            iot_tag.into_iter().chain(decommissioned_tag).collect(),
                            None,
                            None,
                            now,
                        ),
                        [0x3c, 0xd9, 0x2b, 0x30, 0x26, 0x01],
                    ),
                    Some("HP LaserJet Pro MFP M428fdw, Firmware 20230809"),
                    Some("1.3.6.1.4.1.11.2.3.9.1"),
                    Some("HQ Floor 1, Copy Room"),
                    Some("helpdesk@acme-corp.com"),
                    None,
                    Some("HP"),
                    Some("LaserJet Pro MFP M428fdw"),
                    Some("CNBRK1F0X8"),
                )),
                now,
                (
                    "HP Printer",
                    "HP Printer",
                    Some(PortType::Ipp),
                    iot_tag.into_iter().collect()
                ),
            ),
            now - Duration::days(1100),
            now - Duration::days(430),
        ),
        &[legacy_printers_cred],
    ));

    // 27. Camera Entrance
    result.push(host_with_services!(
        with_mac(
            create_host(
                "cam-entrance",
                None,
                Some("Entrance security camera"),
                hq,
                hq_iot,
                Ipv4Addr::new(10, 0, 30, 60),
                iot_tag.into_iter().collect(),
                None,
                None,
                now
            ),
            [0xc0, 0x56, 0xe3, 0x30, 0x27, 0x01],
        ),
        now,
        (
            "RTSP Camera",
            "Security Camera",
            Some(PortType::Rtsp),
            iot_tag.into_iter().collect()
        ),
    ));

    // 28. Camera Parking. No name, no hostname and no SNMP, so it is titled by its address.
    result.push(host_with_services!(
        unnamed(with_mac(
            create_host(
                "",
                None,
                Some("Parking lot security camera"),
                hq,
                hq_iot,
                Ipv4Addr::new(10, 0, 30, 61),
                iot_tag.into_iter().collect(),
                None,
                None,
                now
            ),
            [0xc0, 0x56, 0xe3, 0x30, 0x28, 0x01],
        )),
        now,
        (
            "RTSP Camera",
            "Security Camera",
            Some(PortType::Rtsp),
            iot_tag.into_iter().collect()
        ),
    ));

    // 29. HVAC Controller (BACnet) — building automation, not a factory floor: the OT
    // positioning this demo needs shows up just as plausibly on the building's own HVAC. Found by a
    // scan, which matched BACnet on its well-known port alone, the weakest match there is.
    result.push(found_by_scan(host_with_services!(
        with_mac(
            create_host(
                "hvac-controller01",
                Some("hvac01.acme.local"),
                Some("Building HVAC controller"),
                hq,
                hq_iot,
                Ipv4Addr::new(10, 0, 30, 70),
                iot_tag.into_iter().collect(),
                None,
                None,
                now
            ),
            [0x00, 0x0b, 0x50, 0x30, 0x29, 0x01],
        ),
        now,
        (
            "BACnet",
            "BACnet",
            Some(PortType::BACnet),
            iot_tag.into_iter().collect()
        ),
    )));

    // 30. Facility UPS (Modbus TCP) — power monitoring, the other common source of an
    // industrial protocol in an office building. Modbus device identification yields a
    // firmware revision but nothing that maps to a separate software revision. Found by a scan,
    // whose Modbus probe matched the service outright.
    let mut facility_ups = found_by_scan(host_with_services!(
        with_mac(
            create_host(
                "facility-ups01",
                Some("ups01.acme.local"),
                Some("Facility UPS with network management card"),
                hq,
                hq_iot,
                Ipv4Addr::new(10, 0, 30, 71),
                iot_tag.into_iter().collect(),
                None,
                None,
                now
            ),
            [0x00, 0x0b, 0x50, 0x30, 0x30, 0x01],
        ),
        now,
        (
            "Modbus TCP",
            "Modbus TCP",
            Some(PortType::ModbusTcp),
            iot_tag.into_iter().collect()
        ),
    ));
    facility_ups.host.base.firmware_revision = Some(Attributed::new(
        HostFirmwareRevisionValue("2.1.3".to_string()),
        AttributeSource::Probe(ClientProbe::ModbusTcp),
    ));
    result.push(facility_ups);

    // 30b. Facility switch: an Instant On access switch for the cameras and UPS, managed from the
    // Instant On cloud portal rather than the UniFi controller.
    result.push(with_credentials(
        host_with_services!(
            with_mac(
                create_host(
                    "instanton-sw-facility",
                    Some("sw-facility.acme.local"),
                    Some("HPE Networking Instant On 1930 24G switch"),
                    hq,
                    hq_iot,
                    Ipv4Addr::new(10, 0, 30, 5),
                    iot_tag.into_iter().collect(),
                    None,
                    None,
                    now
                ),
                [0x20, 0x4c, 0x03, 0x30, 0x05, 0x01],
            ),
            now,
            ("Instant On Switch", "Instant On Switch", None, vec![]),
        ),
        &[instant_on_cred],
    ));

    // -- Guest WiFi (10.0.100.x) --

    // 31. Guest AP
    result.push(host_with_services!(
        with_snmp(
            with_mac(
                create_host(
                    "guest-ap",
                    Some("guest-ap.acme.local"),
                    Some("Guest WiFi access point"),
                    hq,
                    hq_guest,
                    Ipv4Addr::new(10, 0, 100, 1),
                    iot_tag.into_iter().collect(),
                    network_devices_cred,
                    None,
                    now,
                ),
                [0xfc, 0xec, 0xda, 0x00, 0x29, 0x01],
            ),
            Some("UniFi U6-Lite, 6.6.65, Linux 5.4.0"),
            Some("1.3.6.1.4.1.41112.1.6"),
            Some("HQ Guest Lobby, Ceiling Mount"),
            Some("netops@acme-corp.com"),
            Some("fc:ec:da:aa:bb:03"),
            Some("Ubiquiti"),
            Some("U6-Lite"),
            Some("UIFCECDAAABB03"),
        ),
        now,
        (
            "Unifi Access Point",
            "UniFi AP",
            None,
            iot_tag.into_iter().collect()
        ),
    ));

    // 32. Bind9 secondary DNS (Management)
    result.push(host_with_services!(
        with_mac(
            create_host(
                "bind9-dns",
                Some("bind9.acme.local"),
                Some("Bind9 secondary DNS server"),
                hq,
                hq_mgmt,
                Ipv4Addr::new(10, 0, 1, 6),
                vec![],
                None,
                None,
                now
            ),
            [0xf8, 0xbc, 0x12, 0x10, 0x30, 0x01],
        ),
        now,
        ("Bind9", "Bind9", Some(PortType::DnsUdp), vec![]),
    ));

    // -- Annex (10.0.50.x) — known only via an LLDP neighbor advertisement from the HQ core
    // switch; nothing here has been scanned directly, so this host is EntitySource::Inferred
    // rather than Manual, and its subnet (see generate_subnets) carries a provisional
    // cidr_source instead of a daemon self-report.
    {
        let hq_annex = find_subnet("HQ Annex");
        let host_id = Uuid::new_v4();
        let ip_address = IPAddress {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IPAddressBase {
                site_id: hq.id,
                host_id,
                subnet_id: hq_annex.id,
                ip_address: IpAddr::V4(Ipv4Addr::new(10, 0, 50, 1)),
                mac_address: None,
                name: None,
                position: 0,
            },
        };
        let host = Host {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: host_id,
            created_at: now,
            updated_at: now,
            base: HostBase {
                // An LLDP far end arrives nameless. It is titled by the chassis ID it advertised,
                // which here is a locally assigned string rather than a MAC.
                name: HostName::unnamed(),
                site_id: hq.id,
                hostname: None,
                description: Some(
                    "Seen via LLDP from the HQ core switch; not yet scanned directly".to_string(),
                ),
                source: EntitySource::Inferred,
                virtualization_metadata: None,
                virtualization_service_id: None,
                virtualization_interface_id: None,
                hidden: false,
                tags: vec![],
                sys_descr: None,
                sys_object_id: None,
                sys_location: None,
                sys_contact: None,
                management_url: None,
                chassis_id: Some(Attributed::new(
                    HostChassisIdValue("hq-annex-uplink".to_string()),
                    AttributeSource::LldpChassisId,
                )),
                sys_name: None,
                manufacturer: None,
                model: None,
                serial_number: None,
                asset_tag: None,
                firmware_revision: None,
                software_revision: None,
                os: None,
                credential_assignments: vec![],
            },
        };
        result.push(HostWithServices {
            host,
            ip_addresses: vec![ip_address],
            ports: vec![],
            services: vec![],
        });
    }

    // ========================================================================
    // DATA CENTER SITE — 20 hosts
    // ========================================================================
    let dc = find_site("Data Center");
    let dc_mgmt = find_subnet("DC Management");
    let dc_compute = find_subnet("DC Compute");
    let dc_storage = find_subnet("DC Storage");
    let dc_dmz = find_subnet("DC DMZ");
    let dc_docker = find_subnet("DC Docker Bridge");
    let dc_vpn = find_subnet("DC VPN");

    // -- Management (172.16.0.x) --

    // 1. DC Firewall
    result.push(with_credentials(
        host_with_services!(
            with_snmp(
                with_mac(
                    create_host(
                        "dc-fw01",
                        Some("fw01.dc.acme.io"),
                        Some("Data center firewall"),
                        dc,
                        dc_mgmt,
                        Ipv4Addr::new(172, 16, 0, 1),
                        critical_tag.into_iter().collect(),
                        network_devices_cred,
                        None,
                        now,
                    ),
                    [0x70, 0x4c, 0xa5, 0xdc, 0x01, 0x01],
                ),
                Some("FortiGate-60F v7.4.3, build 2573, 240514 (GA.F)"),
                Some("1.3.6.1.4.1.12356.101.1"),
                Some("DC-East, Cage 4, Rack 1"),
                Some("netops@acme-corp.com"),
                None,
                Some("Fortinet"),
                Some("FortiGate-60F"),
                Some("FGT60FTK24010123"),
            ),
            now,
            (
                "Fortinet",
                "FortiGate",
                Some(PortType::Https),
                critical_tag.into_iter().collect()
            ),
        ),
        &[dc_snmpv3_cred],
    ));

    // 2. DC Switch (24 ports, LLDP) — same bootloader/OS firmware split as the HQ core switch.
    let mut dc_switch = host_with_services!(
        with_snmp(
            with_mac(
                create_host(
                    "dc-switch-01",
                    Some("switch-01.dc.acme.io"),
                    Some("Data center managed switch"),
                    dc,
                    dc_mgmt,
                    Ipv4Addr::new(172, 16, 0, 2),
                    vec![],
                    network_devices_cred,
                    None,
                    now,
                ),
                [0x00, 0x1c, 0x73, 0xdc, 0x02, 0x01],
            ),
            Some(
                "Arista Networks EOS version 4.32.0F running on an Arista Networks DCS-7050SX3-48YC12"
            ),
            Some("1.3.6.1.4.1.30065.1.3011.7050.3735.48.3328.12"),
            Some("DC-East, Cage 4, Rack 2"),
            Some("netops@acme-corp.com"),
            Some("78:45:c4:ab:cd:02"),
            Some("Arista Networks"),
            Some("DCS-7050SX3-48YC12"),
            Some("JPE21120CD02"),
        ),
        now,
        ("SNMP", "SNMP", Some(PortType::Snmp), vec![]),
        ("Switch", "Switch", None, vec![]),
    );
    let dc_switch_revision_source = AttributeSource::Probe(ClientProbe::Snmp);
    // The one demo device whose administrator set entPhysicalAssetID, so the scan reads it.
    dc_switch.host.base.asset_tag = Some(Attributed::new(
        HostAssetTagValue("DC-NET-0031".to_string()),
        dc_switch_revision_source,
    ));
    dc_switch.host.base.firmware_revision = Some(Attributed::new(
        HostFirmwareRevisionValue("Aboot-4.0.1".to_string()),
        dc_switch_revision_source,
    ));
    dc_switch.host.base.software_revision = Some(Attributed::new(
        HostSoftwareRevisionValue("EOS-4.32.0F".to_string()),
        dc_switch_revision_source,
    ));
    result.push(with_credentials(
        dc_switch,
        &[dc_snmpv3_cred, arista_gnmi_cred],
    ));

    // 3. Zabbix Monitoring
    result.push(host_with_services!(
        with_mac(
            create_host(
                "zabbix-mon",
                Some("zabbix.dc.acme.io"),
                Some("Zabbix monitoring server"),
                dc,
                dc_mgmt,
                Ipv4Addr::new(172, 16, 0, 10),
                monitoring_tag.into_iter().collect(),
                None,
                None,
                now
            ),
            [0xf8, 0xbc, 0x12, 0xdc, 0x03, 0x01],
        ),
        now,
        (
            "Zabbix",
            "Zabbix",
            Some(PortType::Http8080),
            monitoring_tag.into_iter().collect()
        ),
    ));

    // -- DMZ (172.16.30.x) --

    // 4. HAProxy Load Balancer (pre-generated ID for dependency wiring)
    {
        let (host, ip_address) = with_mac(
            create_host(
                "haproxy-lb01",
                Some("lb01.dc.acme.io"),
                Some("HAProxy load balancer"),
                dc,
                dc_dmz,
                Ipv4Addr::new(172, 16, 30, 10),
                production_tag
                    .into_iter()
                    .chain(critical_tag)
                    .chain(web_tier_tag)
                    .collect(),
                None,
                None,
                now,
            ),
            [0xf8, 0xbc, 0x12, 0xdc, 0x04, 0x01],
        );
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((mut svc, port)) = create_service(
            "HAProxy",
            "HAProxy",
            &host,
            &ip_addresses[0],
            Some(PortType::Https),
            web_tier_tag.into_iter().collect(),
            now,
        ) {
            svc.base.bindings[0].id = haproxy_dc_binding_id;
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            ip_addresses,
            ports,
            services,
        });
    }

    // 5. App Server 01 (pre-generated Tomcat ID for dependency wiring)
    {
        let (host, ip_address) = with_mac(
            create_host(
                "app-server-01",
                Some("app-01.dc.acme.io"),
                Some("Application server 1"),
                dc,
                dc_dmz,
                Ipv4Addr::new(172, 16, 30, 20),
                production_tag.into_iter().chain(web_tier_tag).collect(),
                None,
                None,
                now,
            ),
            [0xf8, 0xbc, 0x12, 0xdc, 0x05, 0x01],
        );
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((mut svc, port)) = create_service(
            "Tomcat",
            "Tomcat",
            &host,
            &ip_addresses[0],
            Some(PortType::Http8080),
            web_tier_tag.into_iter().collect(),
            now,
        ) {
            svc.base.bindings[0].id = app01_dc_binding_id;
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        if let Some((svc, port)) = create_service(
            "SSH",
            "SSH",
            &host,
            &ip_addresses[0],
            Some(PortType::Ssh),
            vec![],
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            ip_addresses,
            ports,
            services,
        });
    }

    // 6. App Server 02. Runs its sidecars under rootless Podman.
    result.push(with_credentials(
        host_with_services!(
            with_mac(
                create_host(
                    "app-server-02",
                    Some("app-02.dc.acme.io"),
                    Some("Application server 2"),
                    dc,
                    dc_dmz,
                    Ipv4Addr::new(172, 16, 30, 21),
                    production_tag.into_iter().chain(web_tier_tag).collect(),
                    None,
                    None,
                    now
                ),
                [0xf8, 0xbc, 0x12, 0xdc, 0x06, 0x01],
            ),
            now,
            (
                "Tomcat",
                "Tomcat",
                Some(PortType::Http8080),
                web_tier_tag.into_iter().collect()
            ),
            ("SSH", "SSH", Some(PortType::Ssh), vec![]),
            ("Podman", "Podman", Some(PortType::Docker), vec![]),
        ),
        &[podman_proxy_cred],
    ));

    // -- Compute (172.16.10.x) --

    // 7. DC Proxmox Hypervisor (pre-generated Proxmox VE service ID)
    {
        let (host, ip_address) = with_snmp(
            with_mac(
                create_host(
                    "dc-proxmox-hv01",
                    Some("proxmox-hv01.dc.acme.io"),
                    Some("Data center Proxmox hypervisor"),
                    dc,
                    dc_compute,
                    Ipv4Addr::new(172, 16, 10, 5),
                    production_tag.into_iter().collect(),
                    None,
                    None,
                    now,
                ),
                [0xf8, 0xbc, 0x12, 0xdc, 0x07, 0x01],
            ),
            Some("Linux dc-proxmox-hv01 6.8.12-1-pve #1 SMP PVE 6.8.12-1 x86_64"),
            Some("1.3.6.1.4.1.8072.3.2.10"),
            Some("DC-East, Cage 4, Rack 3"),
            Some("sysadmin@acme-corp.com"),
            None,
            Some("Dell Inc."),
            Some("PowerEdge R640"),
            Some("DL9RT4DC07PVE"),
        );
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((svc, port)) = create_service_with_id(
            pve_dc_svc_id,
            "Proxmox VE",
            "Proxmox VE",
            &host,
            &ip_addresses[0],
            Some(PortType::new_tcp(8006)),
            production_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        if let Some((svc, port)) = create_service(
            "SSH",
            "SSH",
            &host,
            &ip_addresses[0],
            Some(PortType::Ssh),
            vec![],
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            ip_addresses,
            ports,
            services,
        });
    }

    // 8. argocd-vm — VM on dc-proxmox-hv01 (vm_id=300)
    result.push(host_with_services!(
        with_mac(
            create_host(
                "argocd-vm",
                Some("argocd.dc.acme.io"),
                Some("ArgoCD (VM on dc-proxmox-hv01)"),
                dc,
                dc_compute,
                Ipv4Addr::new(172, 16, 10, 10),
                production_tag.into_iter().collect(),
                None,
                Some((
                    HostVirtualization::Proxmox(ProxmoxVirtualization {
                        vm_name: Some("argocd-vm".to_string()),
                        vm_id: Some("300".to_string()),
                        guest_type: Some(ProxmoxGuestType::Qemu),
                    }),
                    pve_dc_svc_id,
                )),
                now
            ),
            [0x52, 0x54, 0x00, 0xdc, 0x08, 0x01],
        ),
        now,
        (
            "ArgoCD",
            "ArgoCD",
            Some(PortType::Https8443),
            [production_tag, devops_tag].into_iter().flatten().collect()
        ),
    ));

    // 9. graylog-vm — VM on dc-proxmox-hv01 (vm_id=301)
    result.push(host_with_services!(
        with_mac(
            create_host(
                "graylog-vm",
                Some("graylog.dc.acme.io"),
                Some("Graylog log management (VM on dc-proxmox-hv01)"),
                dc,
                dc_compute,
                Ipv4Addr::new(172, 16, 10, 11),
                monitoring_tag.into_iter().collect(),
                None,
                Some((
                    HostVirtualization::Proxmox(ProxmoxVirtualization {
                        vm_name: Some("graylog-vm".to_string()),
                        vm_id: Some("301".to_string()),
                        guest_type: Some(ProxmoxGuestType::Qemu),
                    }),
                    pve_dc_svc_id,
                )),
                now
            ),
            [0x52, 0x54, 0x00, 0xdc, 0x09, 0x01],
        ),
        now,
        (
            "Graylog",
            "Graylog",
            Some(PortType::Http9000),
            monitoring_tag.into_iter().collect()
        ),
    ));

    // 10. mariadb-vm — VM on dc-proxmox-hv01 (vm_id=302, on Storage subnet)
    // Pre-generated MariaDB service ID for dependency wiring
    {
        let (host, ip_address) = with_mac(
            create_host(
                "mariadb-vm",
                Some("mariadb.dc.acme.io"),
                Some("MariaDB database (VM on dc-proxmox-hv01)"),
                dc,
                dc_storage,
                Ipv4Addr::new(172, 16, 20, 10),
                database_tag.into_iter().collect(),
                None,
                Some((
                    HostVirtualization::Proxmox(ProxmoxVirtualization {
                        vm_name: Some("mariadb-vm".to_string()),
                        vm_id: Some("302".to_string()),
                        guest_type: Some(ProxmoxGuestType::Qemu),
                    }),
                    pve_dc_svc_id,
                )),
                now,
            ),
            [0x52, 0x54, 0x00, 0xdc, 0x10, 0x01],
        );
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((mut svc, port)) = create_service(
            "MariaDB",
            "MariaDB",
            &host,
            &ip_addresses[0],
            Some(PortType::MySql),
            database_tag.into_iter().collect(),
            now,
        ) {
            svc.base.bindings[0].id = mariadb_dc_binding_id;
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            ip_addresses,
            ports,
            services,
        });
    }

    // 11. dc-docker01 — Docker host (2 ip_addresses: eth0 on Compute, docker0 on DC Docker Bridge)
    {
        let host_id = Uuid::new_v4();
        let eth0 = IPAddress {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IPAddressBase {
                site_id: dc.id,
                host_id,
                subnet_id: dc_compute.id,
                ip_address: IpAddr::V4(Ipv4Addr::new(172, 16, 10, 20)),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0xf8, 0xbc, 0x12, 0xdc, 0x11, 0x01])),
                    AttributeSource::ArpReply,
                )),
                name: Some("eth0".to_string()),
                position: 0,
            },
        };
        let docker0 = IPAddress {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IPAddressBase {
                site_id: dc.id,
                host_id,
                subnet_id: dc_docker.id,
                ip_address: IpAddr::V4(Ipv4Addr::new(172, 18, 0, 1)),
                mac_address: Some(MacEvidence::new(
                    MacEvidenceValue(MacAddress::new([0x02, 0x42, 0xac, 0x12, 0x00, 0x01])),
                    AttributeSource::ArpReply,
                )),
                name: Some("docker0".to_string()),
                position: 1,
            },
        };
        let host = Host {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: host_id,
            created_at: now,
            updated_at: now,
            base: HostBase {
                name: HostName::manual("dc-docker01".to_string()),
                site_id: dc.id,
                hostname: Some(Attributed::new(
                    crate::server::hosts::r#impl::attributes::HostHostnameValue(
                        "docker01.dc.acme.io".to_string(),
                    ),
                    AttributeSource::ReverseDns,
                )),
                description: Some("Data center Docker host".to_string()),
                source: EntitySource::Manual,
                virtualization_metadata: None,
                virtualization_service_id: None,
                virtualization_interface_id: None,
                hidden: false,
                tags: production_tag.into_iter().collect(),
                sys_descr: None,
                sys_object_id: None,
                sys_location: None,
                sys_contact: None,
                management_url: None,
                chassis_id: None,
                sys_name: None,
                manufacturer: None,
                model: None,
                serial_number: None,
                asset_tag: None,
                firmware_revision: None,
                software_revision: None,
                os: docker_engine_host_os(),
                credential_assignments: docker_proxy_cred
                    .into_iter()
                    .map(|id| CredentialAssignment {
                        credential_id: id,
                        ip_address_ids: None,
                    })
                    .collect(),
            },
        };

        let mut ports = Vec::new();
        let mut services = Vec::new();

        // Docker Daemon service with pre-generated ID on eth0
        if let Some((svc, port)) = create_service_with_id(
            docker_dc_svc_id,
            "Docker",
            "Docker Daemon",
            &host,
            &eth0,
            Some(PortType::Docker),
            vec![],
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        // Portainer on eth0
        if let Some((svc, port)) = create_service(
            "Portainer",
            "Portainer",
            &host,
            &eth0,
            Some(PortType::Http9000),
            production_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }

        // Container services on docker0 (pre-generated IDs for Observability Stack)
        for (def_id, name, pt, cname, cid, override_svc_id) in [
            (
                "Prometheus",
                "Prometheus",
                Some(PortType::new_tcp(9090)),
                "prometheus",
                "p1r2o3m4e5t6",
                Some(prometheus_dc_svc_id),
            ),
            (
                "Grafana",
                "Grafana",
                Some(PortType::Http3000),
                "grafana",
                "g1r2a3f4a5n6",
                Some(grafana_dc_svc_id),
            ),
            (
                "Jaeger",
                "Jaeger",
                Some(PortType::Https),
                "jaeger",
                "j1a2e3g4e5r6",
                Some(jaeger_dc_svc_id),
            ),
            (
                "Loki",
                "Loki",
                Some(PortType::new_tcp(3100)),
                "loki",
                "l1o2k3i4d5c6",
                None,
            ),
        ] {
            if let Some((mut svc, port)) = create_container_service(
                def_id,
                name,
                &host,
                &docker0,
                pt,
                cname,
                cid,
                docker_dc_svc_id,
                monitoring_tag.into_iter().collect(),
                now,
            ) {
                if let Some(id) = override_svc_id {
                    svc.id = id;
                }
                if let Some(p) = port {
                    ports.push(p);
                }
                services.push(svc);
            }
        }

        result.push(HostWithServices {
            host,
            ip_addresses: vec![eth0, docker0],
            ports,
            services,
        });
    }

    // 12. RabbitMQ
    result.push(host_with_services!(
        with_mac(
            create_host(
                "rabbitmq-node01",
                Some("mq.dc.acme.io"),
                Some("RabbitMQ message broker"),
                dc,
                dc_compute,
                Ipv4Addr::new(172, 16, 10, 30),
                production_tag.into_iter().collect(),
                None,
                None,
                now
            ),
            [0xf8, 0xbc, 0x12, 0xdc, 0x12, 0x01],
        ),
        now,
        (
            "RabbitMQ",
            "RabbitMQ",
            Some(PortType::AMQP),
            [production_tag, messaging_tag]
                .into_iter()
                .flatten()
                .collect()
        ),
    ));

    // 13. Redis Cluster
    result.push(host_with_services!(
        with_mac(
            create_host(
                "redis-cluster01",
                Some("redis.dc.acme.io"),
                Some("Redis cluster node"),
                dc,
                dc_compute,
                Ipv4Addr::new(172, 16, 10, 31),
                database_tag.into_iter().collect(),
                None,
                None,
                now
            ),
            [0xf8, 0xbc, 0x12, 0xdc, 0x13, 0x01],
        ),
        now,
        (
            "Redis",
            "Redis",
            Some(PortType::Redis),
            database_tag.into_iter().collect()
        ),
    ));

    // -- Storage (172.16.20.x) --

    // 14. MinIO (pre-generated service ID for Storage Tier dependency)
    {
        let (host, ip_address) = with_mac(
            create_host(
                "minio-storage",
                Some("minio.dc.acme.io"),
                Some("MinIO object storage"),
                dc,
                dc_storage,
                Ipv4Addr::new(172, 16, 20, 20),
                backup_tag.into_iter().collect(),
                None,
                None,
                now,
            ),
            [0xf8, 0xbc, 0x12, 0xdc, 0x14, 0x01],
        );
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((mut svc, port)) = create_service(
            "MinIO",
            "MinIO",
            &host,
            &ip_addresses[0],
            Some(PortType::Https),
            [backup_tag, storage_tag].into_iter().flatten().collect(),
            now,
        ) {
            svc.id = minio_dc_svc_id;
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            ip_addresses,
            ports,
            services,
        });
    }

    // 15. Ceph (pre-generated service ID for Storage Tier dependency)
    {
        let (host, ip_address) = with_mac(
            create_host(
                "ceph-node01",
                Some("ceph.dc.acme.io"),
                Some("Ceph storage node"),
                dc,
                dc_storage,
                Ipv4Addr::new(172, 16, 20, 21),
                backup_tag.into_iter().collect(),
                None,
                None,
                now,
            ),
            [0xf8, 0xbc, 0x12, 0xdc, 0x15, 0x01],
        );
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((mut svc, port)) = create_service(
            "Ceph",
            "Ceph",
            &host,
            &ip_addresses[0],
            None,
            [backup_tag, storage_tag].into_iter().flatten().collect(),
            now,
        ) {
            svc.id = ceph_dc_svc_id;
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            ip_addresses,
            ports,
            services,
        });
    }

    // 16. Elasticsearch (pre-generated service ID for Storage Tier dependency)
    {
        let (host, ip_address) = with_mac(
            create_host(
                "elasticsearch-dc",
                Some("es.dc.acme.io"),
                Some("Elasticsearch cluster"),
                dc,
                dc_storage,
                Ipv4Addr::new(172, 16, 20, 30),
                database_tag.into_iter().collect(),
                None,
                None,
                now,
            ),
            [0xf8, 0xbc, 0x12, 0xdc, 0x16, 0x01],
        );
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((mut svc, port)) = create_service(
            "Elasticsearch",
            "Elasticsearch",
            &host,
            &ip_addresses[0],
            Some(PortType::Elasticsearch),
            database_tag.into_iter().collect(),
            now,
        ) {
            svc.id = elasticsearch_dc_svc_id;
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            ip_addresses,
            ports,
            services,
        });
    }

    // 17. InfluxDB. Decommissioned after metrics moved to Prometheus; it has not answered a scan
    // since.
    result.push(gone_quiet(
        host_with_services!(
            with_mac(
                create_host(
                    "influxdb-metrics",
                    Some("influxdb.dc.acme.io"),
                    Some("InfluxDB metrics store"),
                    dc,
                    dc_storage,
                    Ipv4Addr::new(172, 16, 20, 31),
                    monitoring_tag.into_iter().collect(),
                    None,
                    None,
                    now
                ),
                [0xf8, 0xbc, 0x12, 0xdc, 0x17, 0x01],
            ),
            now,
            (
                "InfluxDB",
                "InfluxDB",
                Some(PortType::InfluxDb),
                database_tag.into_iter().collect()
            ),
        ),
        now - Duration::days(900),
        now - Duration::days(410),
    ));

    // -- VPN Tunnel (10.8.0.x) --

    // 18. DC VPN
    result.push(host_with_services!(
        with_mac(
            create_host(
                "dc-vpn",
                Some("vpn.dc.acme.io"),
                Some("Data center VPN endpoint"),
                dc,
                dc_vpn,
                Ipv4Addr::new(10, 8, 0, 1),
                vec![],
                None,
                None,
                now
            ),
            [0xf8, 0xbc, 0x12, 0xdc, 0x18, 0x01],
        ),
        now,
        ("OpenVPN", "OpenVPN", Some(PortType::OpenVPN), vec![]),
    ));

    // -- Additional --

    // 19. Cloudflared Tunnel (DMZ)
    result.push(host_with_services!(
        with_mac(
            create_host(
                "cloudflared-tunnel",
                Some("cloudflared.dc.acme.io"),
                Some("Cloudflare tunnel endpoint"),
                dc,
                dc_dmz,
                Ipv4Addr::new(172, 16, 30, 5),
                production_tag.into_iter().collect(),
                None,
                None,
                now
            ),
            [0xf8, 0xbc, 0x12, 0xdc, 0x19, 0x01],
        ),
        now,
        (
            "Cloudflared",
            "Cloudflared",
            Some(PortType::Https),
            production_tag.into_iter().collect()
        ),
    ));

    // 20. DC Admin Workstation (Compute). A Windows jump box with OpenSSH Server enabled, whose
    // banner names the OS.
    result.push(host_with_services!(
        with_ssh_banner(
            with_mac(
                create_host(
                    "dc-ws-admin",
                    Some("ws-admin.dc.acme.io"),
                    Some("DC admin workstation"),
                    dc,
                    dc_compute,
                    Ipv4Addr::new(172, 16, 10, 100),
                    vec![],
                    None,
                    None,
                    now
                ),
                [0xf8, 0xbc, 0x12, 0xdc, 0x20, 0x01],
            ),
            WINDOWS_SSH_BANNER,
        ),
        now,
        ("Workstation", "Workstation", Some(PortType::Rdp), vec![]),
        ("SSH", "SSH", Some(PortType::Ssh), vec![]),
    ));

    result
}
