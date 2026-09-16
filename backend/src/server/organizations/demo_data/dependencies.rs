use super::*;

// ============================================================================
// Dependencies
// ============================================================================

/// Generate demo dependencies with pre-generated service IDs for member wiring.
#[allow(clippy::vec_init_then_push)]
pub(super) fn generate_dependencies(
    networks: &[Network],
    tags: &[Tag],
    svc_ids: &DependencyServiceIds,
) -> Vec<Dependency> {
    let now = Utc::now();
    let hq = networks
        .iter()
        .find(|n| n.base.name == "Headquarters")
        .unwrap();
    let dc = networks
        .iter()
        .find(|n| n.base.name == "Data Center")
        .unwrap();

    let monitoring_tag = tags
        .iter()
        .find(|t| t.base.name == "Monitoring")
        .map(|t| t.id);

    let mut dependencies = Vec::new();

    // ===== HQ Dependencies (3) =====

    // 1. Monitoring Stack: Prometheus (hub) ↔ Grafana, Uptime Kuma (spokes)
    dependencies.push(Dependency {
        valid_from: now,
        valid_to: None,
        lineage_id: None,
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: DependencyBase {
            name: "Monitoring Stack".to_string(),
            network_id: hq.id,
            description: Some(
                "Prometheus metrics collection with Grafana visualization".to_string(),
            ),
            dependency_type: DependencyType::HubAndSpoke,
            members: DependencyMembers::Services {
                service_ids: vec![
                    svc_ids.prometheus_hq,
                    svc_ids.grafana_hq,
                    svc_ids.uptime_kuma,
                ],
            },
            source: EntitySource::Manual,
            color: Color::Purple,
            edge_style: EdgeStyle::Straight,
            tags: monitoring_tag.into_iter().collect(),
        },
    });

    // 2. Backup Flow: Proxmox VE (hv01) → TrueNAS
    dependencies.push(Dependency {
        valid_from: now,
        valid_to: None,
        lineage_id: None,
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: DependencyBase {
            name: "Backup Flow".to_string(),
            network_id: hq.id,
            description: Some("Server backup targets to TrueNAS storage".to_string()),
            dependency_type: DependencyType::RequestPath,
            members: DependencyMembers::Bindings {
                binding_ids: vec![svc_ids.pve_hq1_binding, svc_ids.truenas_binding],
            },
            source: EntitySource::Manual,
            color: Color::Green,
            edge_style: EdgeStyle::SmoothStep,
            tags: vec![],
        },
    });

    // 3. Reverse Proxy Path: Traefik → Gitea
    dependencies.push(Dependency {
        valid_from: now,
        valid_to: None,
        lineage_id: None,
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: DependencyBase {
            name: "Reverse Proxy Path".to_string(),
            network_id: hq.id,
            description: Some("Traffic path through reverse proxy to code hosting".to_string()),
            dependency_type: DependencyType::RequestPath,
            members: DependencyMembers::Bindings {
                binding_ids: vec![svc_ids.traefik_hq_binding, svc_ids.gitea_hq_binding],
            },
            source: EntitySource::Manual,
            color: Color::Cyan,
            edge_style: EdgeStyle::Bezier,
            tags: vec![],
        },
    });

    // ===== DC Dependencies (3) =====

    // 4. Web Traffic Flow: HAProxy → Tomcat → MariaDB
    dependencies.push(Dependency {
        valid_from: now,
        valid_to: None,
        lineage_id: None,
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: DependencyBase {
            name: "Web Traffic Flow".to_string(),
            network_id: dc.id,
            description: Some(
                "Production web request path from load balancer through app servers to database"
                    .to_string(),
            ),
            dependency_type: DependencyType::RequestPath,
            members: DependencyMembers::Bindings {
                binding_ids: vec![
                    svc_ids.haproxy_dc_binding,
                    svc_ids.app01_dc_binding,
                    svc_ids.mariadb_dc_binding,
                ],
            },
            source: EntitySource::Manual,
            color: Color::Blue,
            edge_style: EdgeStyle::Bezier,
            tags: vec![],
        },
    });

    // 5. Observability Stack: Prometheus (container) → Grafana (container), Jaeger (container)
    dependencies.push(Dependency {
        valid_from: now,
        valid_to: None,
        lineage_id: None,
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: DependencyBase {
            name: "Observability Stack".to_string(),
            network_id: dc.id,
            description: Some(
                "Containerized observability: Prometheus, Grafana, and Jaeger".to_string(),
            ),
            dependency_type: DependencyType::HubAndSpoke,
            members: DependencyMembers::Services {
                service_ids: vec![svc_ids.prometheus_dc, svc_ids.grafana_dc, svc_ids.jaeger_dc],
            },
            source: EntitySource::Manual,
            color: Color::Purple,
            edge_style: EdgeStyle::Straight,
            tags: monitoring_tag.into_iter().collect(),
        },
    });

    // 6. Storage Tier: MinIO → Ceph, Elasticsearch
    dependencies.push(Dependency {
        valid_from: now,
        valid_to: None,
        lineage_id: None,
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: DependencyBase {
            name: "Storage Tier".to_string(),
            network_id: dc.id,
            description: Some("Object storage, distributed storage, and search".to_string()),
            dependency_type: DependencyType::HubAndSpoke,
            members: DependencyMembers::Services {
                service_ids: vec![svc_ids.minio_dc, svc_ids.ceph_dc, svc_ids.elasticsearch_dc],
            },
            source: EntitySource::Manual,
            color: Color::Orange,
            edge_style: EdgeStyle::SmoothStep,
            tags: vec![],
        },
    });

    dependencies
}
