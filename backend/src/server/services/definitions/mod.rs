use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::shared::types::metadata::HasId;
use inventory;

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

    /// Every `(definition, port)` pair where a completed TCP connection alone is enough to name the
    /// service.
    ///
    /// Derived from each definition's own pattern, so it needs no maintenance: writing a probe or
    /// an endpoint match drops a definition out of this set, and adding a bare `Pattern::Port` puts
    /// one in. `connect_only_definitions_are_declared` holds it equal to the definitions declaring
    /// a [`ConnectOnly`](crate::server::services::r#impl::definitions::ConnectOnly) rationale.
    pub fn connect_only_definitions() -> Vec<(&'static str, PortType)> {
        let mut pairs: Vec<(&'static str, PortType)> =
            inventory::iter::<ServiceDefinitionFactory>()
                .flat_map(|factory| {
                    let definition = factory.create();
                    let pattern = definition.discovery_pattern();
                    let id = definition.id();
                    pattern
                        .ports()
                        .into_iter()
                        .filter(|port| pattern.matches_on_connect_alone(*port))
                        .map(|port| (id, port))
                        .collect::<Vec<_>>()
                })
                .collect();
        pairs.sort_by_key(|(id, port)| (*id, port.number(), port.protocol()));
        pairs.dedup();
        pairs
    }
}

// ============= NETWORK INFRASTRUCTURE =============

// NetworkCore
pub mod cockpit;
pub mod dhcp_server;
pub mod gateway;
pub mod gnmi;
pub mod h323;
pub mod mgcp;
pub mod ntp;
pub mod rdp;
pub mod rpcbind;
pub mod rustdesk;
pub mod saltmaster;
pub mod saltproxy;
pub mod smux;
pub mod snmp;
pub mod ssh;
pub mod switch;
pub mod telnet;
pub mod termix;
pub mod tftp;
pub mod unifi_gateway;
pub mod unifi_switch;
pub mod vnc;
pub mod wake_on_lan;

// NetworkAccess
pub mod access_point;
pub mod eero_gateway;
pub mod eero_repeater;
pub mod fios_extender;
pub mod fios_gateway;
pub mod google_nest_repeater;
pub mod google_nest_router;
pub mod instant_on;
pub mod instant_on_access_point;
pub mod instant_on_gateway;
pub mod instant_on_switch;
pub mod mikrotik;
pub mod tp_link_eap;
pub mod ubiquiti_discovery;
pub mod unifi_access_point;
pub mod unifi_controller;
pub mod upnp_igd;

// NetworkSecurity
pub mod crowdsec;
pub mod firewall;
pub mod fortigate;
pub mod open_wrt;
pub mod opn_sense;
pub mod pf_blocker_ng;
pub mod pf_sense;

// ============= NETWORK SERVICES =============

// DNS
pub mod bind9;
pub mod dns_server;
pub mod llmnr;
pub mod power_dns;
pub mod unbound;

// VPN
pub mod cloudflared;
pub mod openvpn;
pub mod strongswan;
pub mod wg_dashboard;
pub mod wireguard;

// ReverseProxy
pub mod caddy;
pub mod haproxy;
pub mod iis;
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
pub mod iscsi;
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
pub mod unimus;
pub mod veeam;

// Media
pub mod audiobookshelf;
pub mod bazarr;
pub mod booklore;
pub mod calibre;
pub mod calibre_web;
pub mod emby;
pub mod immich;
pub mod jellyfin;
pub mod jellyseerr;
pub mod jellystat;
pub mod komga;
pub mod ombi;
pub mod overseerr;
pub mod plex;
pub mod sabnzbd;
pub mod serviio;
pub mod slskd;
pub mod spotify_connect;
pub mod tautulli;

// HomeAutomation
pub mod domoticz;
pub mod esphome;
pub mod espressif_device;
pub mod home_assistant;
pub mod openhab;
pub mod philips_hue_bridge;
pub mod zigbee2mqtt;
pub mod zwave_js;

// Virtualization
pub mod consul;
pub mod docker_api_proxy;
pub mod docker_container;
pub mod docker_daemon;
pub mod docker_swarm;
pub mod dockge;
pub mod esxi;
pub mod kubernetes;
pub mod libvirt;
pub mod nomad;
pub mod openshift;
pub mod podman;
pub mod podman_api_proxy;
pub mod podman_container;
pub mod portainer;
pub mod proxmox;
pub mod rancher;
pub mod vcenter;

// ============= APPLICATION SERVICES =============

// Database
pub mod cassandra;
pub mod couchdb;
pub mod denodo;
pub mod elasticsearch;
pub mod hyperfile;
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
pub mod celery;
pub mod kafka;
pub mod mqtt;
pub mod ntfy;
pub mod rabbitmq;
pub mod sidekiq;

// Development

// Collaboration
pub mod bookstack;
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
pub mod n8n;
pub mod nats;
pub mod spinnaker;
pub mod teamcity;

// Web
pub mod ghost;
pub mod mendelson_as2;
pub mod spring_boot;
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
pub mod dashy;
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
pub mod apple_mobile_device;
pub mod client;
pub mod ipad;
pub mod iphone;

// IoT
pub mod airplay_receiver;
pub mod amazon_echo;
pub mod apple_tv;
pub mod bose_speaker;
pub mod camera;
pub mod chromecast;
pub mod frigate;
pub mod google_home;
pub mod home_connect;
pub mod homekit_accessory;
pub mod homepod;
pub mod iot;
pub mod myq;
pub mod nest_protect;
pub mod nest_thermostat;
pub mod ring_doorbell;
pub mod roborock_vacuum;
pub mod roku;
pub mod rollease_acmeda_pulse;
pub mod sonos_speaker;
pub mod tasmota;
pub mod tp_link_kasa;
pub mod tuyasmart;
pub mod wiz;

// Industrial
pub mod bacnet;
pub mod epics;
pub mod ethernet_ip;
pub mod modbus_tcp;
pub mod opc_ua;

// Printer
pub mod airprint_printer;
pub mod bambu_lab_printer;
pub mod brother_printer;
pub mod cups;
pub mod hp_printer;
pub mod ipp_printer;
pub mod jetdirect;
pub mod lpd;
pub mod print_server;

// ============= UTILITIES & MISC =============

// Various Web/Dashboard/Development apps
pub mod actual_budget;
pub mod autobrr;
pub mod backrest;
pub mod cleanuparr;
pub mod dawarich;
pub mod docmost;
pub mod firefly_iii;
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
pub mod network_identities;
pub mod open_ports;
pub mod scanopy_daemon;
pub mod scanopy_server;

// ============= MATCHED BY RECOG FINGERPRINTS =============
pub mod adminer;
pub mod apache_airflow;
pub mod apache_druid;
pub mod apache_flink;
pub mod apache_solr;
pub mod apache_spark;
pub mod apache_superset;
pub mod aruba_clearpass;
pub mod atlassian_fisheye;
pub mod avtech_dvr;
pub mod blue_coat_proxy;
pub mod bugzilla;
pub mod cacti;
pub mod check_point_gateway;
pub mod cisco_apic;
pub mod cisco_asdm;
pub mod citrix_netscaler;
pub mod citrix_xenserver;
pub mod cloudpanel;
pub mod cockroachdb;
pub mod control_web_panel;
pub mod cpanel;
pub mod crushftp;
pub mod dell_openmanage;
pub mod f5_big_ip;
pub mod gerrit;
pub mod goanywhere_mft;
pub mod gogs;
pub mod google_cast;
pub mod greenbone;
pub mod hikvision;
pub mod hp_smh;
pub mod ibm_spectrum_protect;
pub mod ignition_gateway;
pub mod influxdata_chronograf;
pub mod ivanti_endpoint_manager_agent;
pub mod jamf_pro;
pub mod jboss;
pub mod jira_service_management;
pub mod jupyter;
pub mod kibana;
pub mod kubernetes_dashboard;
pub mod lansweeper;
pub mod librenms;
pub mod manageengine_access_manager_plus;
pub mod manageengine_adaudit_plus;
pub mod manageengine_admanager_plus;
pub mod manageengine_adselfservice_plus;
pub mod manageengine_assetexplorer;
pub mod manageengine_endpoint_central;
pub mod manageengine_opmanager;
pub mod manageengine_pam360;
pub mod manageengine_servicedesk_plus;
pub mod metabase;
pub mod mongo_express;
pub mod monit;
pub mod moodle;
pub mod moveit_transfer;
pub mod n_able_n_central;
pub mod nessus;
pub mod netflix_nrdp;
pub mod nexus_repository;
pub mod novnc;
pub mod ntopng;
pub mod observium;
pub mod octopus_deploy;
pub mod openfire;
pub mod opengear_console;
pub mod openstack_horizon;
pub mod oracle_access_manager;
pub mod oracle_secure_global_desktop;
pub mod oracle_weblogic;
pub mod outlook_web_access;
pub mod phpmyadmin;
pub mod plesk;
pub mod prtg;
pub mod pulse_connect_secure;
pub mod r1soft_backup;
pub mod rapid7_insightvm;
pub mod redis_commander;
pub mod redmine;
pub mod roundcube;
pub mod rundeck;
pub mod samsung_smart_tv;
pub mod sangoma_switchvox;
pub mod sap_netweaver;
pub mod shellinabox;
pub mod skype_for_business;
pub mod solarwinds_kiwi_syslog;
pub mod solarwinds_orion;
pub mod sonarqube;
pub mod sonicwall_uma;
pub mod spiceworks;
pub mod sunbird_power_iq;
pub mod supervisor;
pub mod symantec_sepm;
pub mod tenable_security_center;
pub mod three_cx;
pub mod twonky_media_server;
pub mod unraid;
pub mod vmware_horizon;
pub mod vmware_site_recovery_manager;
pub mod watchguard_fireware;
pub mod webmin;
pub mod wowza_streaming_engine;
pub mod wso2;
pub mod zimbra;
