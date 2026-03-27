use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::shared::types::metadata::HasId;
use inventory;

/// Allowed external domains for service logo URLs.
/// Used both by CSP policy (img-src) and logo URL validation tests.
pub const ALLOWED_LOGO_DOMAINS: &[&str] = &[
    "cdn.jsdelivr.net",
    "simpleicons.org",
    "vectorlogo.zone",
    "cdn.prod.website-files.com",
];

#[derive(Debug, Clone, Copy)]
pub struct ServiceDefinitionFactory(pub fn() -> Box<dyn ServiceDefinition>);

impl ServiceDefinitionFactory {
    pub const fn new(factory: fn() -> Box<dyn ServiceDefinition>) -> Self {
        Self(factory)
    }

    pub fn create(&self) -> Box<dyn ServiceDefinition> {
        (self.0)()
    }
}

pub fn create_service<T>() -> Box<dyn ServiceDefinition>
where
    T: ServiceDefinition + Default + 'static,
{
    Box::new(T::default())
}

inventory::collect!(ServiceDefinitionFactory);

pub struct ServiceDefinitionRegistry;

impl ServiceDefinitionRegistry {
    /// Get all registered services as instances
    pub fn all_service_definitions() -> Vec<Box<dyn ServiceDefinition>> {
        inventory::iter::<ServiceDefinitionFactory>()
            .map(|factory| factory.create())
            .collect()
    }

    pub fn service_exists(id: &str) -> bool {
        inventory::iter::<ServiceDefinitionFactory>().any(|factory| factory.create().id() == id)
    }

    pub fn find_by_id(id: &str) -> Option<Box<dyn ServiceDefinition>> {
        inventory::iter::<ServiceDefinitionFactory>().find_map(|factory| {
            let service_definition = factory.create();
            if service_definition.id() == id {
                Some(service_definition)
            } else {
                None
            }
        })
    }
}

// ============= NETWORK INFRASTRUCTURE =============

// NetworkCore
pub mod dhcp_server;
pub mod gateway;
pub mod ntp;
pub mod rdp;
pub mod saltmaster;
pub mod saltproxy;
pub mod snmp;
pub mod ssh;
pub mod switch;
pub mod telnet;

// NetworkAccess
pub mod access_point;
pub mod eero_gateway;
pub mod eero_repeater;
pub mod fios_extender;
pub mod fios_gateway;
pub mod google_nest_repeater;
pub mod google_nest_router;
pub mod mikrotik;
pub mod tp_link_eap;
pub mod ubiquiti_discovery;
pub mod unifi_access_point;
pub mod unifi_controller;

// NetworkSecurity
pub mod crowdsec;
pub mod firewall;
pub mod fortigate;
pub mod opn_sense;
pub mod pf_blocker_ng;
pub mod pf_sense;

// ============= NETWORK SERVICES =============

// DNS
pub mod bind9;
pub mod dns_server;
pub mod power_dns;
pub mod unbound;

// VPN
pub mod cloudflared;
pub mod openvpn;
pub mod wg_dashboard;
pub mod wireguard;

// ReverseProxy
pub mod caddy;
pub mod haproxy;
pub mod kong;
pub mod nginx_proxy_manager;
pub mod traefik;
pub mod tyk;

// AdBlock
pub mod adguard_home;
pub mod pi_hole;

// ============= SERVER SERVICES =============

// Storage
pub mod ceph;
pub mod file_server;
pub mod filezilla_server;
pub mod minio;
pub mod next_cloud;
pub mod nfs_server;
pub mod open_media_vault;
pub mod owncloud;
pub mod qnap;
pub mod samba;
pub mod seafile;
pub mod syncthing;
pub mod synology;
pub mod true_nas;

// Backup
pub mod bacula;
pub mod duplicati;
pub mod proxmox_backup_server;
pub mod restic;
pub mod veeam;

// Media
pub mod audiobookshelf;
pub mod bazarr;
pub mod booklore;
pub mod emby;
pub mod immich;
pub mod jellyfin;
pub mod jellyseerr;
pub mod jellystat;
pub mod komga;
pub mod overseerr;
pub mod plex;
pub mod sabnzbd;
pub mod slskd;
pub mod tautulli;

// HomeAutomation
pub mod domoticz;
pub mod esphome;
pub mod home_assistant;
pub mod openhab;
pub mod philips_hue_bridge;
pub mod zigbee2mqtt;
pub mod zwave_js;

// Virtualization
pub mod docker_container;
pub mod docker_daemon;
pub mod docker_swarm;
pub mod dockge;
pub mod kubernetes;
pub mod nomad;
pub mod openshift;
pub mod portainer;
pub mod proxmox;
pub mod rancher;

// ============= APPLICATION SERVICES =============

// Database
pub mod cassandra;
pub mod couchdb;
pub mod denodo;
pub mod elasticsearch;
pub mod influxdb;
pub mod mariadb;
pub mod mongodb;
pub mod mssql_server;
pub mod mysql;
pub mod neo4j;
pub mod oracle_db;
pub mod postgresql;
pub mod redis_db;

// Message Queues
pub mod activemq;
pub mod ampq;
pub mod kafka;
pub mod mqtt;
pub mod ntfy;
pub mod rabbitmq;

// Development

// Collaboration
pub mod confluence_server;
pub mod discourse;
pub mod jira_server;
pub mod mattermost;
pub mod radicale;
pub mod rocket_chat;

// Development
pub mod ansible_awx;
pub mod appwrite;
pub mod argocd;
pub mod bamboo;
pub mod bitbucket_server;
pub mod drone_ci;
pub mod forgejo;
pub mod gitea;
pub mod github_enterprise;
pub mod gitlab;
pub mod jenkins;
pub mod nats;
pub mod spinnaker;
pub mod teamcity;

// Web
pub mod ghost;
pub mod tomcat;
pub mod web_service;
pub mod wordpress;

// Identity and Access
pub mod active_directory;
pub mod authentik;
pub mod bitwarden;
pub mod freeipa;
pub mod kerberos;
pub mod keycloak;
pub mod ldap;
pub mod vault;
pub mod vaultwarden;

// Dashboard
pub mod glance_app;
pub mod homarr;
pub mod homepage;
pub mod jump;
pub mod linkstack;
pub mod proxmox_datacenter_manager;

// Monitoring
pub mod apc;
pub mod beszel_agent;
pub mod cadvisor;
pub mod checkmk_agent;
pub mod coolercontrol;
pub mod elastic_apm;
pub mod gatus;
pub mod glances;
pub mod grafana;
pub mod graylog;
pub mod icinga;
pub mod jaeger;
pub mod loki;
pub mod nagios;
pub mod ncpa_agent;
pub mod netdata;
pub mod nvidia_gpu_exporter;
pub mod openspeedtest;
pub mod prometheus;
pub mod prometheus_node_exporter;
pub mod promtail;
pub mod pulse;
pub mod sensu;
pub mod sentry;
pub mod splunk;
pub mod uptime_kuma;
pub mod wazuh;
pub mod zabbix;
pub mod zabbix_agent;
pub mod zipkin;

// Communication
pub mod asterisk;
pub mod bigbluebutton;
pub mod freepbx;
pub mod jitsi_meet;
pub mod mailcow;
pub mod sip_server;

// ============= END DEVICES =============

// Workstation
pub mod workstation;

// Mobile
pub mod client;

// IoT
pub mod amazon_echo;
pub mod bacnet;
pub mod camera;
pub mod chromecast;
pub mod frigate;
pub mod google_home;
pub mod iot;
pub mod nest_protect;
pub mod nest_thermostat;
pub mod ring_doorbell;
pub mod roborock_vacuum;
pub mod roku;
pub mod sonos_speaker;
pub mod tasmota;

// Printer
pub mod cups;
pub mod hp_printer;
pub mod jetdirect;
pub mod print_server;

// ============= UTILITIES & MISC =============

// Various Web/Dashboard/Development apps
pub mod actual_budget;
pub mod autobrr;
pub mod backrest;
pub mod cleanuparr;
pub mod freshrss;
pub mod grocy;
pub mod huntarr;
pub mod jotty;
pub mod karakeep;
pub mod lidarr;
pub mod lubelogger;
pub mod me_tube;
pub mod mealie;
pub mod memos;
pub mod netbootxyz;
pub mod nut;
pub mod ollama;
pub mod open_webui;
pub mod paperless_ngx;
pub mod peanut;
pub mod pocket_id;
pub mod prowlarr;
pub mod qbittorrent;
pub mod radarr;
pub mod sonarr;
pub mod wizarr;

// ============= SPECIAL =============

// Scanopy
pub mod open_ports;
pub mod scanopy_daemon;
pub mod scanopy_server;
