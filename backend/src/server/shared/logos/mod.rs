//! Every logo Scanopy draws.
//!
//! A service definition, OS family, sign-in provider or email names a [`Logo`] variant; where the
//! logo comes from and what it is drawn on are stated once, in [`Logo::spec`]. `generate-fixtures`
//! downloads every variant into `ui/static/logos/catalog/` and writes `logos.json` (each logo's
//! path and background, which the UI draws from) and `logo-credits.json` (Settings > System).

use crate::server::auth::r#impl::oidc::IdentityProviderLogo;
use crate::server::hosts::r#impl::os::HostOsFamily;
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::shared::fixtures::logo_ext;
use crate::server::shared::types::metadata::TypeMetadataProvider;
use serde::Serialize;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, IntoStaticStr};

mod spec;

/// Every logo Scanopy draws. Serializes as its variant name, the id `logos.json` is keyed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, IntoStaticStr, Serialize)]
pub enum Logo {
    ActiveMq,
    ActualBudget,
    AdguardHome,
    Adminer,
    AirPlay,
    Alexa,
    Ansible,
    ApacheAirflow,
    ApacheDruid,
    ApacheFlink,
    ApacheSolr,
    ApacheSpark,
    ApacheSuperset,
    Apc,
    Apple,
    AppleMobileDevice,
    AppleTv,
    Appwrite,
    ArgoCd,
    Aruba,
    Asterisk,
    Atlassian,
    Audiobookshelf,
    Authelia,
    Authentik,
    Autobrr,
    Avtech,
    Backrest,
    Bacula,
    Bamboo,
    BambuLab,
    Bazarr,
    Beszel,
    BigBlueButton,
    Bitbucket,
    Bitwarden,
    BookLore,
    Bookstack,
    Bose,
    Broadcom,
    Brocade,
    Brother,
    Bugzilla,
    CPanel,
    Cacti,
    Caddy,
    Cadvisor,
    Calibre,
    CalibreWeb,
    Cassandra,
    Celery,
    Ceph,
    CheckPoint,
    Checkmk,
    Cisco,
    Citrix,
    Cleanuperr,
    CloudPanel,
    Cloudflare,
    Cockpit,
    CockroachDb,
    Confluence,
    Consul,
    ControlWebPanel,
    CoolerControl,
    CouchDb,
    Crowdsec,
    CrushFtp,
    Cups,
    Dashy,
    Dawarich,
    Dell,
    Denodo,
    Discord,
    Discourse,
    Docker,
    Dockge,
    Docmost,
    Domoticz,
    Drone,
    Duplicati,
    Eero,
    Elastic,
    Elasticsearch,
    Emby,
    EntraId,
    Epics,
    EspHome,
    Espressif,
    Esxi,
    F5,
    Filezilla,
    Fios,
    FireflyIii,
    Forgejo,
    Fortinet,
    FreeBsd,
    FreeIpa,
    FreePbx,
    FreshRss,
    Frigate,
    Gatus,
    Gerrit,
    Ghost,
    GitHub,
    GitLab,
    Gitea,
    Glance,
    Glances,
    Gogs,
    Google,
    GoogleCast,
    GoogleHome,
    Grafana,
    Graylog,
    Greenbone,
    Grocy,
    HaProxy,
    HashicorpVault,
    Hikvision,
    Homarr,
    HomeAssistant,
    HomeConnect,
    HomeKit,
    HomePod,
    Homepage,
    Hp,
    Hpe,
    Huawei,
    Huntarr,
    IPad,
    IPhone,
    Ibm,
    Icinga,
    Immich,
    InductiveAutomation,
    InfluxDb,
    Ivanti,
    Jaeger,
    Jamf,
    Jellyfin,
    Jellyseerr,
    Jellystat,
    Jenkins,
    Jira,
    JitsiMeet,
    Jotty,
    JuniperNetworks,
    Jupyter,
    Kafka,
    Kanidm,
    Karakeep,
    Keycloak,
    Komga,
    Kong,
    Kubernetes,
    LibreNms,
    Libvirt,
    Lidarr,
    Linkstack,
    Loki,
    Lubelogger,
    Mailcow,
    MariaDb,
    Mattermost,
    Mealie,
    Memos,
    Metabase,
    Metube,
    Microsoft,
    MicrosoftSqlServer,
    MikroTik,
    Minio,
    MongoDb,
    Monit,
    Moodle,
    Mqtt,
    MyQ,
    Mysql,
    N8n,
    Nagios,
    Nats,
    Neo4j,
    NetApp,
    NetBsd,
    NetbootXyz,
    Netdata,
    Netflix,
    Nextcloud,
    NexusRepository,
    NginxProxyManager,
    NoVnc,
    Nomad,
    Ntfy,
    Ntopng,
    Nut,
    Nvidia,
    Observium,
    OctopusDeploy,
    Okta,
    Ollama,
    Ombi,
    OpenBsd,
    OpenHab,
    OpenLdap,
    OpenMediaVault,
    OpenShift,
    OpenSpeedtest,
    OpenStack,
    OpenVpn,
    OpenWebUi,
    OpenWrt,
    Openfire,
    Opengear,
    OpnSense,
    Oracle,
    Overseerr,
    Owncloud,
    PaloAltoNetworks,
    PaperlessNgx,
    Peanut,
    PfSense,
    PhilipsHue,
    PhpMyAdmin,
    PiHole,
    Plesk,
    Plex,
    PocketId,
    Podman,
    Portainer,
    PostgreSql,
    PowerDns,
    Progress,
    Prometheus,
    Prowlarr,
    Proxmox,
    Prtg,
    Pulse,
    QBittorrent,
    Qnap,
    RabbitMq,
    Radarr,
    Radicale,
    Rancher,
    Rapid7,
    RedHat,
    Redis,
    Redmine,
    Restic,
    Ring,
    Roborock,
    RocketChat,
    Roku,
    RolleaseAcmeda,
    Roundcube,
    Rundeck,
    Rustdesk,
    SabNzbd,
    SaltProject,
    Samsung,
    Sangoma,
    Sap,
    Scanopy,
    Seafile,
    Sensu,
    Sentry,
    Serviio,
    Sidekiq,
    Slskd,
    SolarWinds,
    SonarQube,
    Sonarr,
    SonicWall,
    Sonos,
    Spiceworks,
    Spinnaker,
    Splunk,
    Spotify,
    Spring,
    Symantec,
    Syncthing,
    Synology,
    Tasmota,
    Tautulli,
    TeamCity,
    Tenable,
    Termix,
    ThreeCx,
    Tomcat,
    TpLink,
    Traefik,
    TrueNas,
    Tux,
    Tuya,
    Twonky,
    Tyk,
    Ubiquiti,
    Unbound,
    UniFi,
    Unimus,
    Unraid,
    UptimeKuma,
    VMware,
    Vaultwarden,
    Veeam,
    WatchGuard,
    Wazuh,
    Webmin,
    WindowsLogo,
    Wireguard,
    Wiz,
    Wizarr,
    Wordpress,
    Wowza,
    ZWaveJsUi,
    Zabbix,
    Zigbee2Mqtt,
    Zimbra,
    Zitadel,
}

/// Where a logo's file comes from, holding only what varies within the source.
enum LogoSource {
    /// A file in Dashboard Icons, by its path in the set: `"svg/plex.svg"`.
    DashboardIcons(&'static str),
    /// A file in selfh.st/icons, by its path in the set.
    SelfhstIcons(&'static str),
    /// A Simple Icons icon, by its slug. Its files are monochrome: a `Fixed` logo is the set's
    /// brand-colour render, a `Paired` one the black file plus the set's render in its `dark`
    /// colour. Renders are served by slug with no extension; always SVG.
    SimpleIcons(&'static str),
    /// A VectorLogoZone file, by its path under `/logos/`.
    VectorLogoZone(&'static str),
    /// A file under libvirt.org/logos/.
    Libvirt(&'static str),
    /// A Wikimedia Commons file, by its path under `/wikipedia/commons/`, with the terms its page
    /// states.
    Wikimedia {
        path: &'static str,
        /// The file's Commons page, which states its terms.
        page: &'static str,
        title: &'static str,
        author: &'static str,
        terms: &'static str,
    },
    /// A company's own mark, from its own site, used to name its product.
    Vendor {
        owner: &'static str,
        homepage: &'static str,
        url: &'static str,
    },
    /// A project's own mark, from its repository, under the licence the project states for it.
    Project {
        owner: &'static str,
        homepage: &'static str,
        url: &'static str,
        license: &'static str,
        license_url: &'static str,
    },
    Scanopy,
}

/// What fills the frame a logo is drawn in. Read from each file once, when the logo is added: its
/// outline and edge colour, not anything decoded at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Background {
    /// A free-standing mark, on the card's own colour.
    Clear,
    /// The frame takes this colour and the logo keeps its size: the colour of a logo drawn on its
    /// own square or tile.
    Solid(&'static str),
    /// The logo is a disc, drawn at the frame's size so it fills the circle.
    Round,
}

/// How a logo's colours meet the light and dark themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
pub enum Theming {
    /// One file, drawn as it is in both themes: a colour logo or a brand-colour render.
    Fixed,
    /// A black mark, and the file its own set publishes for the dark theme. `dark` addresses it
    /// the way the logo's source addresses its files: a path in a file-addressed set, the render
    /// colour in Simple Icons. The sets name these files inconsistently (`-light` or `-dark`), so
    /// each is stated rather than derived.
    Paired {
        #[serde(skip)]
        dark: &'static str,
    },
    /// Drawn in the surrounding text colour, as the app's own icons are: a `currentColor` file.
    TextColor,
}

/// Whose a logo is and on what terms, as Settings > System lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Credit {
    pub name: &'static str,
    pub homepage: &'static str,
    pub copyright: &'static str,
    /// None for a company's own mark, used under no license to name its product.
    pub license: Option<&'static str>,
    pub license_url: Option<&'static str>,
}

impl Logo {
    /// Where the file is downloaded from, and what an email links.
    pub fn url(&self) -> String {
        match self.spec().0 {
            LogoSource::DashboardIcons(path) => {
                format!("https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/{path}")
            }
            LogoSource::SelfhstIcons(path) => {
                format!("https://cdn.jsdelivr.net/gh/selfhst/icons/{path}")
            }
            LogoSource::SimpleIcons(slug) => match self.theming() {
                // simpleicons.org/icons/ returns 404 for every icon; the npm package holds the same files.
                Theming::Paired { .. } => {
                    format!("https://cdn.jsdelivr.net/npm/simple-icons/icons/{slug}.svg")
                }
                Theming::Fixed | Theming::TextColor => {
                    format!("https://cdn.simpleicons.org/{slug}")
                }
            },
            LogoSource::VectorLogoZone(path) => format!("https://www.vectorlogo.zone/logos/{path}"),
            LogoSource::Libvirt(path) => format!("https://libvirt.org/logos/{path}"),
            LogoSource::Wikimedia { path, .. } => {
                format!("https://upload.wikimedia.org/wikipedia/commons/{path}")
            }
            LogoSource::Vendor { url, .. } | LogoSource::Project { url, .. } => url.to_string(),
            LogoSource::Scanopy => {
                "https://cdn.jsdelivr.net/gh/scanopy/scanopy@main/media/logo.png".to_string()
            }
        }
    }

    /// How it meets the light and dark themes. Every logo not named here is one file drawn as it is.
    pub fn theming(&self) -> Theming {
        match self {
            Self::Apple => Theming::Paired {
                dark: "svg/apple-light.svg",
            },
            Self::Kafka => Theming::Paired {
                dark: "svg/kafka-dark.svg",
            },
            // Kong's own dark-background mark is its brand lime.
            Self::Kong => Theming::Paired {
                dark: "svg/kong-dark.svg",
            },
            Self::NetApp => Theming::Paired {
                dark: "svg/netapp-light.svg",
            },
            Self::Sonos => Theming::Paired {
                dark: "svg/sonos-dark.svg",
            },
            Self::PhilipsHue => Theming::Paired {
                dark: "svg/philips-hue-light.svg",
            },
            Self::Ring => Theming::Paired {
                dark: "svg/ring-light.svg",
            },
            Self::Moodle => Theming::Paired {
                dark: "svg/moodle-light.svg",
            },
            Self::NexusRepository => Theming::Paired {
                dark: "svg/nexus-dark.svg",
            },
            // jsDelivr refuses the dark SVG; the PNG is the same icon.
            Self::Tenable => Theming::Paired {
                dark: "png/tenable-dark.png",
            },
            Self::ApacheSuperset => Theming::Paired {
                dark: "svg/apache-superset-light.svg",
            },
            Self::WatchGuard => Theming::Paired {
                dark: "svg/watchguard-light.svg",
            },
            Self::AirPlay
            | Self::Bose
            | Self::BigBlueButton
            | Self::Sidekiq
            | Self::Citrix
            | Self::InductiveAutomation => Theming::Paired { dark: "white" },
            Self::IPad | Self::AppleMobileDevice | Self::HomePod => Theming::TextColor,
            _ => Theming::Fixed,
        }
    }

    /// Where the dark-theme file is downloaded from: a `Paired` logo's `dark`, addressed within
    /// its own source. None for a source that publishes no theme pairs.
    pub fn dark_url(&self) -> Option<String> {
        let Theming::Paired { dark } = self.theming() else {
            return None;
        };
        match self.spec().0 {
            LogoSource::DashboardIcons(_) => Some(format!(
                "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/{dark}"
            )),
            LogoSource::SelfhstIcons(_) => {
                Some(format!("https://cdn.jsdelivr.net/gh/selfhst/icons/{dark}"))
            }
            LogoSource::SimpleIcons(slug) => {
                Some(format!("https://cdn.simpleicons.org/{slug}/{dark}"))
            }
            LogoSource::VectorLogoZone(_)
            | LogoSource::Libvirt(_)
            | LogoSource::Wikimedia { .. }
            | LogoSource::Vendor { .. }
            | LogoSource::Project { .. }
            | LogoSource::Scanopy => None,
        }
    }

    /// The downloaded copy's name: the variant's id and the file's extension. Keyed by the variant,
    /// so a change of source keeps the path.
    pub fn file_name(&self) -> String {
        let id: &'static str = self.into();
        format!("{id}.{}", logo_ext(&self.url()))
    }

    /// The dark-theme copy's name, beside the light one: `Apple.dark.svg`.
    pub fn dark_file_name(&self) -> Option<String> {
        let id: &'static str = self.into();
        self.dark_url()
            .map(|url| format!("{id}.dark.{}", logo_ext(&url)))
    }

    /// Where the UI serves the dark-theme copy.
    pub fn dark_path(&self) -> Option<String> {
        self.dark_file_name()
            .map(|name| format!("/logos/catalog/{name}"))
    }

    /// Where the UI serves it. Scanopy's own logo ships with the UI rather than being downloaded.
    pub fn path(&self) -> String {
        match self {
            Self::Scanopy => "/logos/scanopy-logo.png".to_string(),
            _ => format!("/logos/catalog/{}", self.file_name()),
        }
    }

    /// Whether `generate-fixtures` downloads it.
    pub fn is_downloaded(&self) -> bool {
        !matches!(self, Self::Scanopy)
    }

    pub fn background(&self) -> Background {
        self.spec().1
    }

    pub fn credit(&self) -> Credit {
        match self.spec().0 {
            LogoSource::DashboardIcons(_) => Credit {
                name: "Dashboard Icons",
                homepage: "https://github.com/homarr-labs/dashboard-icons",
                copyright: "2024 Bjorn Lammers, Meier Lukas, Thomas Camlong and Homarr Labs",
                license: Some("Apache License 2.0"),
                license_url: Some(
                    "https://github.com/scanopy/scanopy/blob/main/backend/assets/dashboard-icons/LICENSE",
                ),
            },
            LogoSource::SelfhstIcons(_) => Credit {
                name: "selfh.st/icons",
                homepage: "https://github.com/selfhst/icons",
                copyright: "selfh.st",
                license: Some("CC BY 4.0"),
                license_url: Some("https://github.com/selfhst/icons/blob/main/LICENSE"),
            },
            LogoSource::SimpleIcons(_) => Credit {
                name: "Simple Icons",
                homepage: "https://github.com/simple-icons/simple-icons",
                copyright: "Simple Icons contributors",
                license: Some("CC0 1.0"),
                license_url: Some(
                    "https://github.com/simple-icons/simple-icons/blob/develop/LICENSE.md",
                ),
            },
            LogoSource::VectorLogoZone(_) => Credit {
                name: "VectorLogoZone",
                homepage: "https://www.vectorlogo.zone/",
                copyright: "The respective trademark owners",
                license: None,
                license_url: None,
            },
            LogoSource::Libvirt(_) => Credit {
                name: "libvirt",
                homepage: "https://libvirt.org/logos/",
                copyright: "The libvirt project authors",
                license: Some("GNU LGPL 2.1 or later"),
                license_url: Some(
                    "https://github.com/scanopy/scanopy/blob/main/backend/assets/libvirt/LICENSE",
                ),
            },
            LogoSource::Wikimedia {
                page,
                title,
                author,
                terms,
                ..
            } => Credit {
                name: title,
                homepage: page,
                copyright: author,
                license: Some(terms),
                license_url: Some(page),
            },
            LogoSource::Vendor {
                owner, homepage, ..
            } => Credit {
                name: owner,
                homepage,
                copyright: owner,
                license: None,
                license_url: None,
            },
            LogoSource::Project {
                owner,
                homepage,
                license,
                license_url,
                ..
            } => Credit {
                name: owner,
                homepage,
                copyright: owner,
                license: Some(license),
                license_url: Some(license_url),
            },
            LogoSource::Scanopy => Credit {
                name: "Scanopy",
                homepage: "https://scanopy.net",
                copyright: "Scanopy LLC",
                license: None,
                license_url: None,
            },
        }
    }
}

/// A logo as `logos.json` lists it for the UI.
#[derive(Debug, Serialize)]
pub struct LogoEntry {
    pub id: Logo,
    pub path: String,
    pub background: Background,
    pub theming: Theming,
    /// The dark-theme copy, for a `Paired` logo.
    pub dark_path: Option<String>,
    /// A `TextColor` logo's SVG, drawn inline in the text colour. Filled in from the downloaded
    /// file when `logos.json` is written.
    pub markup: Option<String>,
}

/// Every logo, for `logos.json`.
pub fn logo_entries() -> Vec<LogoEntry> {
    Logo::iter()
        .map(|logo| LogoEntry {
            id: logo,
            path: logo.path(),
            background: logo.background(),
            theming: logo.theming(),
            dark_path: logo.dark_path(),
            markup: None,
        })
        .collect()
}

/// What each logo stands for, by the names of the services, OS families and sign-in providers that
/// draw it.
fn logo_uses() -> Vec<(Logo, &'static str)> {
    let services = ServiceDefinitionRegistry::all_service_definitions()
        .into_iter()
        .filter_map(|s| s.logo().map(|logo| (logo, ServiceDefinition::name(&s))));
    let os_families = HostOsFamily::iter()
        .filter_map(|f| f.logo().map(|logo| (logo, TypeMetadataProvider::name(&f))));
    let identity_providers = IdentityProviderLogo::iter().map(|p| (p.logo(), p.name()));
    services
        .chain(os_families)
        .chain(identity_providers)
        .collect()
}

#[derive(Debug, Serialize)]
pub struct LogoCredit {
    #[serde(flatten)]
    pub credit: Credit,
    /// What its logos stand for, sorted and without repeats.
    pub used_for: Vec<&'static str>,
}

/// Each credit with what its logos stand for, most-used first. A logo only an email draws is
/// listed by its id.
pub fn logo_credits() -> Vec<LogoCredit> {
    let uses = logo_uses();
    let mut credits: Vec<LogoCredit> = Vec::new();
    for logo in Logo::iter() {
        let credit = logo.credit();
        let mut names: Vec<&'static str> = uses
            .iter()
            .filter(|(used, _)| *used == logo)
            .map(|(_, name)| *name)
            .collect();
        if names.is_empty() {
            names.push(logo.into());
        }
        match credits.iter_mut().find(|c| c.credit == credit) {
            Some(existing) => existing.used_for.extend(names),
            None => credits.push(LogoCredit {
                credit,
                used_for: names,
            }),
        }
    }
    for credit in &mut credits {
        credit
            .used_for
            .sort_unstable_by_key(|name| name.to_lowercase());
        credit.used_for.dedup();
    }
    credits.sort_by(|a, b| {
        b.used_for
            .len()
            .cmp(&a.used_for.len())
            .then_with(|| a.credit.name.cmp(b.credit.name))
    });
    credits
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A slug given where a file path belongs (or the reverse) builds a URL that downloads nothing
    /// and a path the UI cannot serve.
    #[test]
    fn every_logo_builds_a_url_to_an_image_file() {
        let bad: Vec<(Logo, String)> = Logo::iter()
            .flat_map(|logo| {
                std::iter::once(logo.url())
                    .chain(logo.dark_url())
                    .map(move |url| (logo, url))
            })
            // A Simple Icons render is addressed by slug: no file path to get wrong.
            .filter(|(_, url)| !url.starts_with("https://cdn.simpleicons.org/"))
            .filter(|(_, url)| {
                !reqwest::Url::parse(url).is_ok_and(|u| {
                    [".svg", ".png", ".webp"]
                        .iter()
                        .any(|ext| u.path().ends_with(ext))
                })
            })
            .collect();
        assert!(bad.is_empty(), "logos without an image URL: {bad:?}");
    }

    /// A pair on a source that publishes none would draw its black mark alone on a dark card.
    #[test]
    fn every_paired_logo_has_a_dark_file() {
        let unpaired: Vec<Logo> = Logo::iter()
            .filter(|logo| matches!(logo.theming(), Theming::Paired { .. }))
            .filter(|logo| logo.dark_url().is_none())
            .collect();
        assert!(
            unpaired.is_empty(),
            "Paired without a dark file: {unpaired:?}"
        );
    }
}
