use super::*;

// ============================================================================
// Dependencies
// ============================================================================

/// Generate demo dependencies. Every member is a service in one of the demo's applications, so
/// the Application view draws each edge between two application groups' services.
pub(super) fn generate_dependencies(
    sites: &[Site],
    hosts_with_services: &[HostWithServices],
) -> Vec<Dependency> {
    let now = crate::server::shared::clock::now();
    let hq = sites
        .iter()
        .find(|n| n.base.name == "Headquarters")
        .unwrap();
    let dc = sites.iter().find(|n| n.base.name == "Data Center").unwrap();

    // The service named `service` on the host named `host`.
    let find_service = |host: &str, service: &str| {
        hosts_with_services
            .iter()
            .filter(|h| h.host.base.name.value().as_str() == host)
            .flat_map(|h| &h.services)
            .find(|s| s.base.name == service)
            .unwrap_or_else(|| panic!("demo service {service} on {host}"))
    };
    let service = |host: &str, name: &str| find_service(host, name).id;
    let binding = |host: &str, name: &str| find_service(host, name).base.bindings[0].id;

    let dependency = |name: &str,
                      site: &Site,
                      description: &str,
                      dependency_type: DependencyType,
                      members: DependencyMembers,
                      color: Color| Dependency {
        valid_from: now,
        valid_to: None,
        lineage_id: None,
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: DependencyBase {
            name: name.to_string(),
            site_id: site.id,
            description: Some(description.to_string()),
            dependency_type,
            members,
            source: EntitySource::Manual,
            color,
            edge_style: EdgeStyle::Bezier,
            tags: vec![],
        },
    };

    vec![
        // ===== HQ =====
        // Monitoring: Prometheus (hub) ↔ Grafana, Uptime Kuma (spokes)
        dependency(
            "Monitoring Stack",
            hq,
            "Prometheus metrics collection with Grafana visualization",
            DependencyType::HubAndSpoke,
            DependencyMembers::Services {
                service_ids: vec![
                    service("prometheus", "Prometheus"),
                    service("grafana-mon", "Grafana"),
                    service("uptime-kuma", "Uptime Kuma"),
                    // Zabbix, retired when Prometheus replaced it. The dependency still names it,
                    // so the inspector shows a member that is no longer on the network.
                    Uuid::new_v4(),
                ],
            },
            Color::Purple,
        ),
        // CI/CD: a push to GitLab triggers a Jenkins build
        dependency(
            "Build Pipeline",
            hq,
            "Pushes to GitLab trigger Jenkins builds",
            DependencyType::RequestPath,
            DependencyMembers::Bindings {
                binding_ids: vec![
                    binding("gitlab-vm", "GitLab"),
                    binding("jenkins-ci", "Jenkins"),
                ],
            },
            Color::Pink,
        ),
        // Identity & Access: Vaultwarden signs users in through Keycloak, which stores them in
        // PostgreSQL
        dependency(
            "SSO Login",
            hq,
            "Vaultwarden sign-in through Keycloak, backed by PostgreSQL",
            DependencyType::RequestPath,
            DependencyMembers::Bindings {
                binding_ids: vec![
                    binding("docker-prod01", "Vaultwarden"),
                    binding("keycloak-vm", "Keycloak"),
                    binding("db-vm", "PostgreSQL"),
                ],
            },
            Color::Violet,
        ),
        // File Sharing: Nextcloud (hub) → Redis file locking, NFS data directory
        dependency(
            "Nextcloud Storage",
            hq,
            "Nextcloud file locking in Redis and its data directory on TrueNAS NFS",
            DependencyType::HubAndSpoke,
            DependencyMembers::Services {
                service_ids: vec![
                    service("nextcloud-vm", "Nextcloud"),
                    service("db-vm", "Redis"),
                    service("truenas-primary", "NFS"),
                ],
            },
            Color::Emerald,
        ),
        // Backups: TrueNAS replicates to the Synology each week
        dependency(
            "Backup Replication",
            hq,
            "Weekly replication from TrueNAS to the Synology backup NAS",
            DependencyType::RequestPath,
            DependencyMembers::Bindings {
                binding_ids: vec![
                    binding("truenas-primary", "SSH"),
                    binding("synology-backup", "Synology DSM"),
                ],
            },
            Color::Green,
        ),
        // Wiki: Confluence → its database
        dependency(
            "Wiki Request Path",
            hq,
            "Confluence pages served from its PostgreSQL database",
            DependencyType::RequestPath,
            DependencyMembers::Bindings {
                binding_ids: vec![
                    binding("atlassian-vm", "Confluence"),
                    binding("atlassian-vm", "Confluence Database"),
                ],
            },
            Color::Cyan,
        ),
        // Service Desk: Jira → its database
        dependency(
            "Service Desk Request Path",
            hq,
            "Jira tickets served from its PostgreSQL database",
            DependencyType::RequestPath,
            DependencyMembers::Bindings {
                binding_ids: vec![
                    binding("atlassian-vm", "Jira"),
                    binding("atlassian-vm", "Jira Database"),
                ],
            },
            Color::Fuchsia,
        ),
        // ===== DC =====
        // Customer Portal: Cloudflare tunnel → HAProxy → Tomcat → MariaDB
        dependency(
            "Portal Request Path",
            dc,
            "Customer requests from the Cloudflare tunnel through the load balancer and app servers to the database",
            DependencyType::RequestPath,
            DependencyMembers::Bindings {
                binding_ids: vec![
                    binding("cloudflared-tunnel", "Cloudflared"),
                    binding("haproxy-lb01", "HAProxy"),
                    binding("app-server-01", "Tomcat"),
                    binding("mariadb-vm", "MariaDB"),
                ],
            },
            Color::Blue,
        ),
        // Customer Portal: Tomcat → RabbitMQ → order worker → Elasticsearch
        dependency(
            "Order Queue",
            dc,
            "Orders queued in RabbitMQ, processed by the order worker and indexed for search",
            DependencyType::RequestPath,
            DependencyMembers::Bindings {
                binding_ids: vec![
                    binding("app-server-02", "Tomcat"),
                    binding("rabbitmq-node01", "RabbitMQ"),
                    binding("portal-worker01", "Order Worker"),
                    binding("elasticsearch-dc", "Elasticsearch"),
                ],
            },
            Color::Orange,
        ),
        // Monitoring: Prometheus (container hub) → Grafana, Jaeger (containers)
        dependency(
            "Observability Stack",
            dc,
            "Containerized observability: Prometheus, Grafana, and Jaeger",
            DependencyType::HubAndSpoke,
            DependencyMembers::Services {
                service_ids: vec![
                    service("dc-docker01", "Prometheus"),
                    service("dc-docker01", "Grafana"),
                    service("dc-docker01", "Jaeger"),
                ],
            },
            Color::Purple,
        ),
    ]
}
