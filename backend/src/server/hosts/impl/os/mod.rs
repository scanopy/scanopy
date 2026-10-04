//! A host's operating system, as one source read or inferred it.
//!
//! One value per host, carried as an [`Attributed`](crate::server::shared::attribution::Attributed)
//! so the source rides with it. The source is also the only certainty marker: a value whose source
//! has method [`Inferred`](crate::server::shared::attribution::AttributeMethod::Inferred) was
//! matched from a banner or a description string, and anything read off the host itself replaces it.

pub mod recog;
mod supplement;

use std::fmt;

use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;

use crate::server::daemons::r#impl::base::DaemonOs;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};

/// Operating system of a host, as one source read or inferred it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct HostOs {
    /// The operating system family.
    pub family: HostOsFamily,
    /// Product or distribution, such as "Ubuntu", "Windows Server 2022" or "IOS-XE".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Release, such as "24.04", "10.0.20348" or "15.2(4)M".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Edition, such as "LTS" or "Datacenter".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    /// Release codename, such as "noble".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codename: Option<String>,
    /// Kernel release, such as "6.8.0-45-generic".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_version: Option<String>,
}

impl HostOs {
    /// Whether this reading is `other` in more detail: the same family and product, every field
    /// `other` sets equal here, and at least one more set. An LXC's config names only its
    /// distribution (`Debian`), and the SSH banner adds the release (`Debian 12.0`).
    pub fn refines(&self, other: &HostOs) -> bool {
        let same_name = match (&self.name, &other.name) {
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
            _ => false,
        };
        let fields = |os: &HostOs| {
            [
                os.version.clone(),
                os.edition.clone(),
                os.codename.clone(),
                os.kernel_version.clone(),
            ]
        };
        let (mine, theirs) = (fields(self), fields(other));
        let covers = mine.iter().zip(&theirs).all(|(m, t)| t.is_none() || m == t);
        let adds = mine
            .iter()
            .zip(&theirs)
            .any(|(m, t)| m.is_some() && t.is_none());
        self.family == other.family && same_name && covers && adds
    }

    /// This reading with every blank text field read as absent, the rule a script's other keys
    /// follow: `"codename": ""` from a host without one is no codename, not an empty one.
    pub fn without_blank_fields(self) -> Self {
        let text = |v: Option<String>| v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        Self {
            family: self.family,
            name: text(self.name),
            version: text(self.version),
            edition: text(self.edition),
            codename: text(self.codename),
            kernel_version: text(self.kernel_version),
        }
    }

    /// A reading that names the family and nothing else.
    pub fn family_only(family: HostOsFamily) -> Self {
        Self {
            family,
            name: None,
            version: None,
            edition: None,
            codename: None,
            kernel_version: None,
        }
    }
}

/// What a person reads: the product when one was named, else the family, then the version.
impl fmt::Display for HostOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name.as_deref().unwrap_or(self.family.name());
        match &self.version {
            Some(version) => write!(f, "{name} {version}"),
            None => f.write_str(name),
        }
    }
}

/// Operating system family of a host.
///
/// No catch-all variant. A source that names something outside this list writes no OS at all, so
/// nothing downstream has to handle a family that says nothing. The list covers every OS family the
/// vendored Recog fingerprints name (`recog::tests::every_vendored_family_is_classified` keeps it
/// that way), plus every OS a daemon can report about its own host. Linux distributions are `Linux`
/// with the distribution as the name; variants exist for families, not distributions.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
pub enum HostOsFamily {
    // Desktop, server and mobile
    Linux,
    Windows,
    MacOs,
    Ios,
    TvOs,
    AudioOs,
    FreeBsd,
    OpenBsd,
    NetBsd,
    Solaris,
    Aix,
    HpUx,
    Irix,
    Tru64,
    OpenVms,
    ZOs,
    IbmI,
    UnixWare,
    OpenServer,
    NetWare,
    PalmOs,
    Esxi,
    // Network and appliance operating systems
    CiscoIos,
    CiscoIosXe,
    CiscoIosXr,
    CiscoNxOs,
    CiscoCatOs,
    CiscoAsa,
    CiscoFtd,
    Junos,
    ScreenOs,
    ArubaOs,
    Comware,
    ProCurve,
    AristaEos,
    RouterOs,
    HuaweiVrp,
    IronWare,
    BrocadeNetworkOs,
    BayRs,
    PanOs,
    FortiOs,
    SonicOs,
    GaiaOs,
    Ipso,
    DataOntap,
    // Embedded
    Ecos,
}

impl From<DaemonOs> for HostOsFamily {
    fn from(os: DaemonOs) -> Self {
        match os {
            DaemonOs::Linux => Self::Linux,
            DaemonOs::MacOS => Self::MacOs,
            DaemonOs::Windows => Self::Windows,
            DaemonOs::FreeBsd => Self::FreeBsd,
        }
    }
}

impl HasId for HostOsFamily {
    fn id(&self) -> &'static str {
        self.into()
    }
}

/// Where Simple Icons serves its monochrome SVGs. CC0-1.0.
const SIMPLE_ICONS: &str = "https://simpleicons.org/icons/";
/// Where Dashboard Icons serves its colored SVGs. Apache-2.0; its license ships with the server
/// (`assets/dashboard-icons/LICENSE`).
const DASHBOARD_ICONS: &str = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/";

impl HostOsFamily {
    /// The daemon OS this family is, when a daemon can run on it. Its tag then draws the same icon
    /// the daemon's OS tag does. Derived through `From<DaemonOs>`, so the two cannot disagree.
    pub fn daemon_os(&self) -> Option<DaemonOs> {
        use strum::IntoEnumIterator;
        DaemonOs::iter().find(|os| HostOsFamily::from(*os) == *self)
    }

    /// The vendor's logo, downloaded with the service logos by `generate-fixtures`. Empty for the
    /// daemon OS families, which draw the daemon's own icon, and for families whose vendor neither
    /// Simple Icons nor Dashboard Icons carries (IBM, SGI, SCO, Novell, Palm, Arista, H3C, Nortel,
    /// Check Point, eCos), which show their lucide glyph from [`EntityMetadataProvider::icon`].
    pub fn logo_url(&self) -> &'static str {
        use const_format::concatcp;
        match self {
            Self::Linux | Self::Windows | Self::MacOs | Self::FreeBsd => "",
            Self::Ios | Self::TvOs | Self::AudioOs => concatcp!(SIMPLE_ICONS, "apple.svg"),
            Self::OpenBsd => concatcp!(SIMPLE_ICONS, "openbsd.svg"),
            Self::NetBsd => concatcp!(SIMPLE_ICONS, "netbsd.svg"),
            Self::HpUx | Self::Tru64 | Self::OpenVms | Self::ProCurve => {
                concatcp!(SIMPLE_ICONS, "hp.svg")
            }
            Self::Esxi => concatcp!(SIMPLE_ICONS, "vmware.svg"),
            Self::CiscoIos
            | Self::CiscoIosXe
            | Self::CiscoIosXr
            | Self::CiscoNxOs
            | Self::CiscoCatOs
            | Self::CiscoAsa
            | Self::CiscoFtd => concatcp!(SIMPLE_ICONS, "cisco.svg"),
            Self::Junos | Self::ScreenOs => concatcp!(SIMPLE_ICONS, "junipernetworks.svg"),
            Self::RouterOs => concatcp!(SIMPLE_ICONS, "mikrotik.svg"),
            Self::HuaweiVrp => concatcp!(SIMPLE_ICONS, "huawei.svg"),
            Self::PanOs => concatcp!(SIMPLE_ICONS, "paloaltonetworks.svg"),
            Self::FortiOs => concatcp!(SIMPLE_ICONS, "fortinet.svg"),
            Self::SonicOs => concatcp!(SIMPLE_ICONS, "sonicwall.svg"),
            Self::DataOntap => concatcp!(SIMPLE_ICONS, "netapp.svg"),
            Self::Solaris => concatcp!(DASHBOARD_ICONS, "oracle.svg"),
            Self::ArubaOs => concatcp!(DASHBOARD_ICONS, "aruba.svg"),
            Self::IronWare | Self::BrocadeNetworkOs => concatcp!(DASHBOARD_ICONS, "brocade.svg"),
            Self::Aix
            | Self::ZOs
            | Self::IbmI
            | Self::Irix
            | Self::UnixWare
            | Self::OpenServer
            | Self::NetWare
            | Self::PalmOs
            | Self::AristaEos
            | Self::Comware
            | Self::BayRs
            | Self::GaiaOs
            | Self::Ipso
            | Self::Ecos => "",
        }
    }

    /// Simple Icons are black, which a dark tag would swallow; the service logos from the same
    /// source set the same flag for the same reason.
    pub fn logo_needs_white_background(&self) -> bool {
        self.logo_url().starts_with(SIMPLE_ICONS)
    }
}

impl EntityMetadataProvider for HostOsFamily {
    fn color(&self) -> Color {
        EntityDiscriminants::Host.color()
    }

    /// The glyph a family shows when it has neither a daemon OS icon nor a logo: what kind of
    /// system it runs on. The others keep the host icon, which nothing draws.
    fn icon(&self) -> Icon {
        match self {
            Self::Aix
            | Self::ZOs
            | Self::IbmI
            | Self::Irix
            | Self::UnixWare
            | Self::OpenServer
            | Self::NetWare => Icon::ServerCog,
            Self::AristaEos | Self::Comware | Self::BayRs | Self::GaiaOs | Self::Ipso => {
                Icon::Router
            }
            Self::PalmOs => Icon::Smartphone,
            Self::Ecos => Icon::Cpu,
            _ => EntityDiscriminants::Host.icon(),
        }
    }
}

/// Emitted as `host-os-families.json`, so the UI labels a family it has no product name for.
impl TypeMetadataProvider for HostOsFamily {
    fn name(&self) -> &'static str {
        match self {
            Self::Linux => "Linux",
            Self::Windows => "Windows",
            Self::MacOs => "macOS",
            Self::Ios => "iOS",
            Self::TvOs => "tvOS",
            Self::AudioOs => "audioOS",
            Self::FreeBsd => "FreeBSD",
            Self::OpenBsd => "OpenBSD",
            Self::NetBsd => "NetBSD",
            Self::Solaris => "Solaris",
            Self::Aix => "AIX",
            Self::HpUx => "HP-UX",
            Self::Irix => "IRIX",
            Self::Tru64 => "Tru64 UNIX",
            Self::OpenVms => "OpenVMS",
            Self::ZOs => "z/OS",
            Self::IbmI => "IBM i",
            Self::UnixWare => "UnixWare",
            Self::OpenServer => "OpenServer",
            Self::NetWare => "NetWare",
            Self::PalmOs => "Palm OS",
            Self::Esxi => "VMware ESXi",
            Self::CiscoIos => "Cisco IOS",
            Self::CiscoIosXe => "Cisco IOS XE",
            Self::CiscoIosXr => "Cisco IOS XR",
            Self::CiscoNxOs => "Cisco NX-OS",
            Self::CiscoCatOs => "Cisco CatOS",
            Self::CiscoAsa => "Cisco ASA",
            Self::CiscoFtd => "Cisco Firepower Threat Defense",
            Self::Junos => "Junos OS",
            Self::ScreenOs => "ScreenOS",
            Self::ArubaOs => "ArubaOS",
            Self::Comware => "Comware",
            Self::ProCurve => "ProCurve",
            Self::AristaEos => "Arista EOS",
            Self::RouterOs => "RouterOS",
            Self::HuaweiVrp => "Huawei VRP",
            Self::IronWare => "IronWare",
            Self::BrocadeNetworkOs => "Brocade Network OS",
            Self::BayRs => "BayRS",
            Self::PanOs => "PAN-OS",
            Self::FortiOs => "FortiOS",
            Self::SonicOs => "SonicOS",
            Self::GaiaOs => "Gaia",
            Self::Ipso => "IPSO",
            Self::DataOntap => "Data ONTAP",
            Self::Ecos => "eCos",
        }
    }

    /// How the UI draws the family's tag: the daemon OS icon, else the downloaded logo, else the
    /// lucide glyph in `icon`.
    fn metadata(&self) -> serde_json::Value {
        let url = self.logo_url();
        serde_json::json!({
            "daemon_os": self.daemon_os().map(<&'static str>::from),
            "logo_ext": if url.is_empty() { "" } else { crate::server::shared::fixtures::logo_ext(url) },
            "logo_needs_white_background": self.logo_needs_white_background(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_family_has_an_icon_to_draw() {
        use strum::IntoEnumIterator;
        for family in HostOsFamily::iter() {
            let drawn = family.daemon_os().is_some()
                || !family.logo_url().is_empty()
                || family.icon().to_string() != EntityDiscriminants::Host.icon().to_string();
            assert!(drawn, "{family:?} has no daemon OS icon, logo or glyph");
        }
    }

    #[test]
    fn each_daemon_os_is_the_family_that_names_it() {
        use strum::IntoEnumIterator;
        for os in DaemonOs::iter() {
            assert_eq!(HostOsFamily::from(os).daemon_os(), Some(os));
        }
    }
    use crate::server::hosts::r#impl::attributes::{
        HostOsValue, HostSysDescrValue, HostSysObjectIdValue,
    };
    use crate::server::hosts::r#impl::base::HostBase;
    use crate::server::services::r#impl::patterns::ClientProbe;
    use crate::server::shared::attribution::{AttributeSource, Attributed};

    fn reading(family: HostOsFamily, name: &str, source: AttributeSource) -> HostBase {
        HostBase {
            os: Some(Attributed::new(
                HostOsValue(HostOs {
                    name: Some(name.to_string()),
                    ..HostOs::family_only(family)
                }),
                source,
            )),
            ..Default::default()
        }
    }

    fn os_name(host: &HostBase) -> Option<String> {
        host.os.as_ref().and_then(|os| os.value().0.name.clone())
    }

    #[test]
    fn a_reading_off_the_host_replaces_a_matched_one_and_a_person_has_the_last_word() {
        let mut host = reading(
            HostOsFamily::Linux,
            "matched",
            AttributeSource::SysDescrMatch,
        );

        host.apply_attributes_from(&reading(
            HostOsFamily::Linux,
            "self-report",
            AttributeSource::DaemonSelfReport,
        ));
        assert_eq!(os_name(&host).as_deref(), Some("self-report"));

        host.apply_attributes_from(&reading(
            HostOsFamily::Linux,
            "script",
            AttributeSource::SshScript,
        ));
        assert_eq!(os_name(&host).as_deref(), Some("script"));

        host.apply_attributes_from(&reading(
            HostOsFamily::MacOs,
            "matched later",
            AttributeSource::SysDescrMatch,
        ));
        assert_eq!(os_name(&host).as_deref(), Some("script"));

        host.apply_attributes_from(&reading(
            HostOsFamily::Linux,
            "typed",
            AttributeSource::Manual,
        ));
        host.apply_attributes_from(&reading(
            HostOsFamily::Linux,
            "script again",
            AttributeSource::SshScript,
        ));
        assert_eq!(os_name(&host).as_deref(), Some("typed"));
    }

    #[test]
    fn the_same_source_reading_an_upgrade_replaces_the_old_release() {
        let mut host = reading(
            HostOsFamily::Linux,
            "Ubuntu 22.04",
            AttributeSource::SshScript,
        );
        host.apply_attributes_from(&reading(
            HostOsFamily::Linux,
            "Ubuntu 24.04",
            AttributeSource::SshScript,
        ));
        assert_eq!(os_name(&host).as_deref(), Some("Ubuntu 24.04"));
    }

    fn snmp_host(sys_descr: &str, sys_object_id: Option<&str>) -> HostBase {
        let probe = AttributeSource::Probe(ClientProbe::Snmp);
        HostBase {
            sys_descr: Some(Attributed::new(
                HostSysDescrValue(sys_descr.to_string()),
                probe,
            )),
            sys_object_id: sys_object_id
                .map(|oid| Attributed::new(HostSysObjectIdValue(oid.to_string()), probe)),
            ..Default::default()
        }
    }

    #[test]
    fn an_snmp_host_gets_the_os_its_system_description_names() {
        let mut host = snmp_host(
            "Hardware: AMD64 Family 21 Model 0 Stepping 2 AT/AT COMPATIBLE - Software: Windows Version 6.3 (Build 9600 Multiprocessor Free)",
            Some(".1.3.6.1.4.1.311.1.1.3.1.3"),
        );
        assert!(host.match_os_from_system_strings());
        let os = host.os.as_ref().expect("a Windows sysDescr names an OS");
        assert_eq!(os.value().0.family, HostOsFamily::Windows);
        assert_eq!(
            os.source().method(),
            crate::server::shared::attribution::AttributeMethod::Inferred
        );
    }

    #[test]
    fn a_matched_os_does_not_displace_one_the_payload_read_off_the_host() {
        let mut host = snmp_host("Linux nas-01 6.1.0-18-amd64 #1 SMP x86_64", None);
        host.os = reading(HostOsFamily::Linux, "Debian", AttributeSource::SshScript).os;
        host.match_os_from_system_strings();
        assert_eq!(os_name(&host).as_deref(), Some("Debian"));
        assert_eq!(host.os.unwrap().source(), AttributeSource::SshScript);
    }

    #[test]
    fn a_description_that_names_no_os_leaves_the_host_without_one() {
        let mut host = snmp_host("Some appliance firmware 4.2", None);
        assert!(!host.match_os_from_system_strings());
        assert!(host.os.is_none());
    }

    fn linux(name: &str, version: Option<&str>) -> HostOs {
        HostOs {
            family: HostOsFamily::Linux,
            name: Some(name.to_string()),
            version: version.map(str::to_string),
            edition: None,
            codename: None,
            kernel_version: None,
        }
    }

    #[test]
    fn a_reading_with_more_detail_refines_the_same_os() {
        let config = linux("Debian", None);
        let banner = linux("debian", Some("12.0"));
        assert!(banner.refines(&config), "a release added");
        assert!(!config.refines(&banner), "a release dropped");

        let with_kernel = HostOs {
            kernel_version: Some("6.1.0".into()),
            ..banner.clone()
        };
        assert!(with_kernel.refines(&banner), "a kernel added");

        assert!(!banner.refines(&banner), "the same reading adds nothing");
        assert!(
            !linux("Ubuntu", Some("24.04")).refines(&config),
            "a different distribution disputes it"
        );
        assert!(
            !linux("Debian", Some("13")).refines(&banner),
            "a different release disputes it"
        );
        let bsd = HostOs {
            family: HostOsFamily::FreeBsd,
            ..banner.clone()
        };
        assert!(!bsd.refines(&config), "a different family disputes it");
    }

    /// The order the lab hits: an LXC's config names Debian at the hypervisor's rank, and the SSH
    /// banner's Debian 12.0 still lands; the config's next reading never takes the release away.
    /// A reading that disputes the stored one still goes by rank.
    #[test]
    fn refinement_is_applied_whatever_the_ranks() {
        use crate::server::hosts::r#impl::attributes::{HostOsAttributed, HostOsValue};
        use crate::server::shared::attribution::{AttributeSource, Attributed};
        let at =
            |os: HostOs, source| -> HostOsAttributed { Attributed::new(HostOsValue(os), source) };

        let mut slot = Some(at(linux("Debian", None), AttributeSource::HypervisorConfig));
        assert!(Attributed::apply(
            &mut slot,
            at(
                linux("Debian", Some("12.0")),
                AttributeSource::SshBannerMatch
            )
        ));
        assert!(!Attributed::apply(
            &mut slot,
            at(linux("Debian", None), AttributeSource::HypervisorConfig)
        ));
        assert_eq!(
            slot.as_ref().unwrap().value().0.version.as_deref(),
            Some("12.0")
        );

        // The refined value is the banner's reading, and is filed under it.
        assert_eq!(
            slot.as_ref().unwrap().source(),
            AttributeSource::SshBannerMatch
        );

        let mut disputed = Some(at(linux("Debian", None), AttributeSource::HypervisorConfig));
        assert!(!Attributed::apply(
            &mut disputed,
            at(
                linux("Ubuntu", Some("24.04")),
                AttributeSource::SshBannerMatch
            )
        ));
        assert_eq!(
            disputed.as_ref().unwrap().value().0.name.as_deref(),
            Some("Debian")
        );

        let mut manual = Some(at(linux("Debian", None), AttributeSource::Manual));
        assert!(!Attributed::apply(
            &mut manual,
            at(
                linux("Debian", Some("12.0")),
                AttributeSource::SshBannerMatch
            )
        ));
    }
}
