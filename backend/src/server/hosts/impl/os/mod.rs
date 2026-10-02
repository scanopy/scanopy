//! A host's operating system, as one source read or inferred it.
//!
//! One value per host, carried as an [`Attributed`](crate::server::shared::attribution::Attributed)
//! so the source rides with it. The source is also the only certainty marker: a value whose source
//! has method [`Inferred`](crate::server::shared::attribution::AttributeMethod::Inferred) was
//! matched from a banner or a description string, and anything read off the host itself replaces it.

pub mod recog;

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

impl EntityMetadataProvider for HostOsFamily {
    fn color(&self) -> Color {
        EntityDiscriminants::Host.color()
    }

    fn icon(&self) -> Icon {
        EntityDiscriminants::Host.icon()
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
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
