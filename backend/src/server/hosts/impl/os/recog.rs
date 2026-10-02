//! OS fingerprints from Rapid7 Recog, matched against strings a host emits about itself.
//!
//! Recog (BSD-2-Clause, `assets/recog/LICENSE`) publishes one XML database per kind of string: SNMP
//! `sysDescr`, SSH banners, HTTP `Server` headers and so on. Each fingerprint is a regex plus the
//! params a match yields (`os.family`, `os.product`, `os.version`), and carries `<example>` strings
//! stating the params they must yield, which the tests below run as a corpus. The files are
//! vendored unmodified from upstream commit `2d1967780c16b13c998966cf0e878768a896c2e2`; refresh by
//! replacing them, and the tests say what a refresh broke.
//!
//! What a match yields is an inference: the string was the host's own, but the OS is our reading of
//! it. Only fingerprints Recog does not mark below [`MIN_CERTAINTY`] are used, and only families
//! that name an operating system ([`classify`]) produce one.

use std::collections::HashMap;
use std::sync::LazyLock;

use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use regex::Regex;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use super::{HostOs, HostOsFamily};

/// The lowest `os.certainty` a fingerprint may carry and still name an OS. Recog leaves it unset on
/// fingerprints it treats as certain; the ones it marks lower identify a product line from a
/// string that other operating systems also emit.
const MIN_CERTAINTY: f32 = 0.9;

/// One vendored Recog database: the kind of string it matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum RecogDatabase {
    /// SNMP `sysDescr.0`.
    SnmpSysDescr,
    /// SNMP `sysObjectID.0`, in dotted form.
    SnmpSysObjectId,
    /// An SSH server's identification line, without the `SSH-2.0-` prefix.
    SshBanner,
    /// An HTTP `Server` header value.
    HttpServer,
    /// An HTTP `Server` header value, read for the platform Apache appends in parentheses.
    ApacheOs,
    /// One `key=value` pair from an mDNS `_device-info._tcp` TXT record.
    MdnsDeviceInfo,
}

impl RecogDatabase {
    fn xml(&self) -> &'static str {
        match self {
            Self::SnmpSysDescr => include_str!("../../../../../assets/recog/snmp_sysdescr.xml"),
            Self::SnmpSysObjectId => include_str!("../../../../../assets/recog/snmp_sysobjid.xml"),
            Self::SshBanner => include_str!("../../../../../assets/recog/ssh_banners.xml"),
            Self::HttpServer => include_str!("../../../../../assets/recog/http_servers.xml"),
            Self::ApacheOs => include_str!("../../../../../assets/recog/apache_os.xml"),
            Self::MdnsDeviceInfo => {
                include_str!("../../../../../assets/recog/mdns_device-info_txt.xml")
            }
        }
    }

    /// The fingerprints in this database, parsed and compiled on first use.
    fn fingerprints(&self) -> &'static [Fingerprint] {
        static DATABASES: LazyLock<HashMap<RecogDatabase, Vec<Fingerprint>>> = LazyLock::new(
            || {
                RecogDatabase::iter()
                    .map(|db| {
                        let fingerprints = parse(db.xml()).unwrap_or_else(|e| {
                            tracing::error!(database = ?db, error = %e, "Unreadable Recog database");
                            Vec::new()
                        });
                        (db, fingerprints)
                    })
                    .collect()
            },
        );
        DATABASES.get(self).map(Vec::as_slice).unwrap_or_default()
    }

    /// The OS `input` names: the first matching Recog fingerprint's, if it names one with enough
    /// certainty, and otherwise Scanopy's own entry for the string ([`super::supplement`]).
    ///
    /// First match only within Recog, which is its own semantics: the databases are ordered from
    /// specific to generic, so a later fingerprint matching too is the less precise reading. Recog
    /// is consulted first because its specific fingerprints name more (a distribution, a product)
    /// than the supplement's broad ones, which exist only to fill what Recog leaves unnamed.
    pub fn os(&self, input: &str) -> Option<HostOs> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }
        self.fingerprints()
            .iter()
            .find_map(|fp| fp.extract(input))
            .and_then(|params| os_from_params(&params))
            .or_else(|| super::supplement::os(*self, input))
    }
}

/// One `<fingerprint>`: a pattern and the params a match yields.
struct Fingerprint {
    pattern: Regex,
    params: Vec<Param>,
    #[cfg_attr(not(test), allow(dead_code))]
    examples: Vec<Example>,
}

/// A `<param>`: a fixed value (`pos="0"`), or capture group `pos` of the match.
struct Param {
    pos: usize,
    name: String,
    value: Option<String>,
}

/// An `<example>`: an input, and the params it states a match yields.
#[cfg_attr(not(test), allow(dead_code))]
struct Example {
    input: String,
    expected: Vec<(String, String)>,
    /// Base64-encoded or file-backed examples are not run; none of the vendored OS ones are.
    runnable: bool,
}

impl Fingerprint {
    /// The params this fingerprint yields for `input`, or `None` when it does not match.
    fn extract(&self, input: &str) -> Option<HashMap<String, String>> {
        let captures = self.pattern.captures(input)?;
        let mut params = HashMap::new();
        for param in &self.params {
            let value = match (&param.value, param.pos) {
                (Some(value), 0) => value.clone(),
                (_, pos) if pos > 0 => match captures.get(pos) {
                    Some(m) if !m.as_str().is_empty() => m.as_str().to_string(),
                    _ => continue,
                },
                _ => continue,
            };
            params.insert(param.name.clone(), value);
        }
        // Fixed values may name another param, as in `{hw.product} Firmware`.
        let snapshot = params.clone();
        for value in params.values_mut() {
            if value.contains('{') {
                for (name, captured) in &snapshot {
                    *value = value.replace(&format!("{{{name}}}"), captured);
                }
            }
        }
        Some(params)
    }
}

/// The OS a match's params name, when the fingerprint is certain enough and its family is an OS.
fn os_from_params(params: &HashMap<String, String>) -> Option<HostOs> {
    let certainty = params
        .get("os.certainty")
        .and_then(|c| c.parse::<f32>().ok());
    if certainty.is_some_and(|c| c < MIN_CERTAINTY) {
        return None;
    }
    let family_text = params.get("os.family")?;
    let product = params.get("os.product").map(String::as_str);
    let RecogFamily::Os(family) = classify(family_text, product) else {
        return None;
    };

    let vendor = params.get("os.vendor").map(String::as_str);
    let meaningful = |s: &&str| !s.trim().is_empty() && *s != family_text;
    // The product names the OS more precisely than the family ("Windows Server 2008 R2"), except
    // where it only repeats it. On Linux the vendor is the distribution ("Ubuntu").
    let name = product
        .filter(meaningful)
        .or_else(|| {
            (family == HostOsFamily::Linux)
                .then_some(vendor)
                .flatten()
                .filter(meaningful)
        })
        .map(str::to_string);

    Some(HostOs {
        family,
        name,
        version: params.get("os.version").filter(|v| !v.is_empty()).cloned(),
        edition: None,
        codename: None,
        // Recog's Linux fingerprints capture the kernel release under this name.
        kernel_version: params
            .get("linux.kernel.version")
            .filter(|v| !v.is_empty())
            .cloned(),
    })
}

/// How a Recog `os.family` value reads as a host OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecogFamily {
    /// The value names an operating system.
    Os(HostOsFamily),
    /// The value names a hardware line or a product family, not an OS. A match writes no OS.
    NotAnOs,
    /// The value is in neither list. A match writes no OS; the vendored data must contain none
    /// (`every_vendored_family_is_classified`), so meeting one at runtime means a family captured
    /// from the matched string that the examples did not cover.
    Unlisted,
}

/// Classify a Recog `os.family`, refined by `os.product` where Recog files several OSes under one
/// family. Case-sensitive on purpose: Recog writes Apple's `iOS` and Cisco's `IOS` as two families.
fn classify(family: &str, product: Option<&str>) -> RecogFamily {
    use HostOsFamily as F;
    let os = match family {
        "Linux" | "OpenWRT" | "UCOS" => F::Linux,
        "Windows" => F::Windows,
        "Mac OS X" => F::MacOs,
        "iOS" => F::Ios,
        "tvOS" => F::TvOs,
        "audioOS" => F::AudioOs,
        "FreeBSD" => F::FreeBsd,
        "OpenBSD" => F::OpenBsd,
        "NetBSD" => F::NetBsd,
        "Solaris" => F::Solaris,
        "AIX" => F::Aix,
        "HP-UX" => F::HpUx,
        "IRIX" => F::Irix,
        "OpenVMS" => F::OpenVms,
        "z/OS" => F::ZOs,
        "OS/400" => F::IbmI,
        "UnixWare" => F::UnixWare,
        "OpenServer" => F::OpenServer,
        "NetWare" => F::NetWare,
        "PalmOS" => F::PalmOs,
        "VMware ESX/ESXi" => F::Esxi,
        "IOS" => match product {
            Some("IOS-XE") => F::CiscoIosXe,
            Some("IOS-XR") => F::CiscoIosXr,
            _ => F::CiscoIos,
        },
        "NX-OS" | "SAN-OS" => F::CiscoNxOs,
        "CatOS" => F::CiscoCatOs,
        "Adaptive Security Appliance" => F::CiscoAsa,
        "Secure Firewall" => F::CiscoFtd,
        "Junos" => F::Junos,
        "ScreenOS" => F::ScreenOs,
        "Aruba" => F::ArubaOs,
        "Comware" => F::Comware,
        "ProCurve" | "Procurve" => F::ProCurve,
        "EOS" => F::AristaEos,
        "RouterOS" => F::RouterOs,
        "VRP" => F::HuaweiVrp,
        "IronWare" => F::IronWare,
        "NetworkOS" => F::BrocadeNetworkOs,
        "BayRS" => F::BayRs,
        "PAN-OS" => F::PanOs,
        "FortiOS" => F::FortiOs,
        "SonicOS" => F::SonicOs,
        "IPSO" => F::Ipso,
        "Data ONTAP" => F::DataOntap,
        // Families Recog uses for several things, of which only some are an OS.
        "Unix" => match product {
            Some("Tru64 Unix") | Some("Digital Unix") => F::Tru64,
            _ => return RecogFamily::NotAnOs,
        },
        "Firewall-1" => match product {
            Some("GAiA OS") => F::GaiaOs,
            _ => return RecogFamily::NotAnOs,
        },
        "Embedded" => match product {
            Some("eCos") => F::Ecos,
            _ => return RecogFamily::NotAnOs,
        },
        other if NOT_AN_OS.contains(&other) => return RecogFamily::NotAnOs,
        _ => return RecogFamily::Unlisted,
    };
    RecogFamily::Os(os)
}

/// Every `os.family` in the vendored databases that names a hardware line or product family rather
/// than an operating system: printers, KVMs, serial servers, switch and appliance model lines.
const NOT_AN_OS: &[&str] = &[
    "1000C",
    "3155 Series",
    "3165 Series",
    "760 Series",
    "Accelar",
    "Access Point",
    "Aficio",
    "AirPort",
    "Application Switch",
    "AR Series",
    "ATM ADSL Unit",
    "ATM ReachDSL Unit",
    "BayStack",
    "BitStorm",
    "bizhub",
    "Blade Switch",
    "BladeCenter",
    "Blue Coat",
    "BOSS",
    "Clariion",
    "CM Series",
    "Color Laser Printer",
    "ColorWave",
    "Connectrix",
    "Copier",
    "CryptoStore",
    "CS 1000",
    "CS Series",
    "Dell Remote Access Controller",
    "Designjet",
    "Device Server",
    "DMT Router",
    "Document Centre",
    "DocuPrint",
    "DS60 Series",
    "e-STUDIO",
    "EDS",
    "ERS",
    "Ethernet Interface",
    "Ethernet Routing Switch",
    "ETS",
    "Fiery",
    "Forms Printer",
    "FrameSaver",
    "FRITZ!Box",
    "G.SHDSL [ATM]",
    "GigaVUE HD",
    "GigaVUE TA",
    "GranDSLAM",
    "HFA",
    "HiPath",
    "HomeConnect",
    "HotWire",
    "HP3000",
    "I-Class",
    "iLO",
    "ILOM",
    "IM Series",
    "imageCLASS",
    "Imagio",
    "Infoprint",
    "Integrity",
    "Intel(R) Active Management Technology",
    "IntelliJack",
    "IP Console Switch",
    "IP KVM",
    "iPR Series",
    "IPSIO",
    "iR Series",
    "JetDirect",
    "Laser Printer",
    "Laser Shot",
    "LaserJet",
    "LinkBuilder",
    "LRS",
    "MarkNet",
    "MatchPort",
    "MCNS Cable Modem",
    "Meraki",
    "Meridian 1",
    "MSA",
    "MSS",
    "Multifunction",
    "MX",
    "MX Series",
    "NC Series",
    "NEO",
    "NetCache",
    "Netopia",
    "NetportExpress",
    "NetQue",
    "Netscaler",
    "NetScaler",
    "NetVanta",
    "Network Printer",
    "NTS",
    "OfficeConnect",
    "Officejet",
    "OPTI-MX",
    "optiPoint",
    "Optra",
    "Packet-Optical",
    "Phaser",
    "Photosmart",
    "PLC",
    "PowerEdge Integrated",
    "PowerVault",
    "Pro",
    "ProLiant",
    "RackBotz",
    "Raptor",
    "Router",
    "RT",
    "RTP Power Controller",
    "SageNET",
    "Scalance",
    "SCS",
    "SDS",
    "SDSL [ATM] Router",
    "Secure Network Access Switch",
    "Secure Router",
    "ShoreGear",
    "SLS",
    "Small Business RV Series Routers",
    "SNMP-Link",
    "SpectraComm",
    "SSL-VPN",
    "ST9000 Series",
    "StorageWorks",
    "StorEdge",
    "Subscriber Networks",
    "Succession 1000/M",
    "Switch",
    "System Storage",
    "T1E1 [COMBO] Router",
    "T3 Termination",
    "TDS750 Series",
    "TelePresence",
    "Time Capsule",
    "TippingPoint",
    "Total Access",
    "TrafficWare",
    "UDS",
    "V1905",
    "V1910",
    "Vantage",
    "VarioPrint",
    "VCX",
    "VG200",
    "VPN",
    "WaveCore",
    "WiBox",
    "Wide Format Printer",
    "WLSE",
    "WorkCentre",
    "WorkCentre Pro",
    "XPort",
    "XPress",
    "ZebraNet",
];

/// Parse one Recog XML database.
fn parse(xml: &str) -> Result<Vec<Fingerprint>, String> {
    let mut reader = Reader::from_str(xml);
    let mut fingerprints = Vec::new();
    let mut current: Option<Fingerprint> = None;
    let mut example: Option<Example> = None;

    loop {
        let event = reader.read_event().map_err(|e| e.to_string())?;
        match event {
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == "fingerprint" => {
                let attrs = attributes(&e)?;
                let pattern = attrs
                    .get("pattern")
                    .ok_or("fingerprint without a pattern")?;
                let pattern = Regex::new(pattern)
                    .map_err(|err| format!("pattern {pattern:?} does not compile: {err}"))?;
                current = Some(Fingerprint {
                    pattern,
                    params: Vec::new(),
                    examples: Vec::new(),
                });
            }
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == "param" => {
                let attrs = attributes(&e)?;
                if let Some(fp) = current.as_mut() {
                    fp.params.push(Param {
                        pos: attrs
                            .get("pos")
                            .and_then(|p| p.parse().ok())
                            .unwrap_or_default(),
                        name: attrs.get("name").cloned().unwrap_or_default(),
                        value: attrs.get("value").cloned(),
                    });
                }
            }
            Event::Start(e) if e.name().as_ref() == "example" => {
                let attrs = attributes(&e)?;
                let runnable = !attrs.contains_key("_encoding") && !attrs.contains_key("_filename");
                example = Some(Example {
                    input: String::new(),
                    expected: attrs
                        .into_iter()
                        .filter(|(k, _)| !k.starts_with('_'))
                        .collect(),
                    runnable,
                });
            }
            Event::Text(t) => {
                if let Some(ex) = example.as_mut() {
                    ex.input.push_str(&t.xml_content(XmlVersion::Implicit1_0));
                }
            }
            Event::GeneralRef(r) => {
                if let Some(ex) = example.as_mut() {
                    let resolved = match r.resolve_char_ref() {
                        Ok(Some(c)) => Some(c.to_string()),
                        _ => quick_xml::escape::resolve_predefined_entity(r.as_ref())
                            .map(str::to_string),
                    };
                    ex.input.push_str(&resolved.unwrap_or_default());
                }
            }
            Event::End(e) if e.name().as_ref() == "example" => {
                if let (Some(ex), Some(fp)) = (example.take(), current.as_mut()) {
                    fp.examples.push(ex);
                }
            }
            Event::End(e) if e.name().as_ref() == "fingerprint" => {
                fingerprints.extend(current.take());
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(fingerprints)
}

fn attributes(e: &BytesStart<'_>) -> Result<HashMap<String, String>, String> {
    e.attributes()
        .map(|attr| {
            let attr = attr.map_err(|err| err.to_string())?;
            let key = attr.key.as_ref().to_string();
            let value = attr
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|err| err.to_string())?
                .into_owned();
            Ok((key, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vendored_database_parses_and_every_pattern_compiles() {
        for db in RecogDatabase::iter() {
            let parsed = parse(db.xml());
            assert!(
                parsed.is_ok(),
                "{db:?}: {}",
                parsed.err().unwrap_or_default()
            );
            assert!(!parsed.unwrap().is_empty(), "{db:?} has no fingerprints");
        }
    }

    /// Recog's own corpus: every example matches its fingerprint and yields the params it states.
    #[test]
    fn every_example_yields_the_params_it_states() {
        let mut failures = Vec::new();
        for db in RecogDatabase::iter() {
            for fp in db.fingerprints() {
                for ex in fp.examples.iter().filter(|ex| ex.runnable) {
                    let Some(params) = fp.extract(&ex.input) else {
                        failures.push(format!("{db:?}: {:?} does not match", ex.input));
                        continue;
                    };
                    for (name, expected) in &ex.expected {
                        if params.get(name) != Some(expected) {
                            failures.push(format!(
                                "{db:?}: {:?} gave {name}={:?}, expected {expected:?}",
                                ex.input,
                                params.get(name)
                            ));
                        }
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// A refresh that adds a family fails here until someone says whether it is an OS.
    #[test]
    fn every_vendored_family_is_classified() {
        let mut unlisted = std::collections::BTreeSet::new();
        for db in RecogDatabase::iter() {
            for fp in db.fingerprints() {
                let fixed = fp
                    .params
                    .iter()
                    .filter(|p| p.name == "os.family")
                    .filter_map(|p| p.value.clone());
                let captured = fp.examples.iter().flat_map(|ex| {
                    ex.expected
                        .iter()
                        .filter(|(k, _)| k == "os.family")
                        .map(|(_, v)| v.clone())
                        .collect::<Vec<_>>()
                });
                for family in fixed.chain(captured) {
                    if classify(&family, None) == RecogFamily::Unlisted {
                        unlisted.insert(family);
                    }
                }
            }
        }
        assert!(
            unlisted.is_empty(),
            "classify() or NOT_AN_OS must list: {unlisted:?}"
        );
    }

    #[test]
    fn a_distribution_banner_names_the_distribution() {
        let os = RecogDatabase::SshBanner
            .os("OpenSSH_6.6.1p1 Ubuntu-2ubuntu2.13")
            .expect("an Ubuntu banner names an OS");
        assert_eq!(os.family, HostOsFamily::Linux);
        assert_eq!(os.name.as_deref(), Some("Ubuntu"));
    }

    #[test]
    fn a_banner_that_names_no_os_yields_none() {
        assert_eq!(RecogDatabase::SshBanner.os("OpenSSH_9.6"), None);
        assert_eq!(RecogDatabase::HttpServer.os("nginx"), None);
    }

    #[test]
    fn a_hardware_line_yields_no_os() {
        assert_eq!(classify("JetDirect", None), RecogFamily::NotAnOs);
    }

    #[test]
    fn cisco_ios_products_resolve_to_their_own_family() {
        let os = RecogDatabase::SnmpSysDescr
            .os("Cisco IOS Software [Dublin], Catalyst L3 Switch Software (CAT9K_LITE_IOSXE), Version 17.12.5, RELEASE SOFTWARE (fc5)")
            .expect("an IOS XE sysDescr names an OS");
        assert_eq!(os.family, HostOsFamily::CiscoIosXe);
        assert_eq!(os.version.as_deref(), Some("17.12.5"));
        assert_eq!(
            classify("IOS", Some("IOS-XE")),
            RecogFamily::Os(HostOsFamily::CiscoIosXe)
        );
        assert_eq!(classify("iOS", None), RecogFamily::Os(HostOsFamily::Ios));
    }

    #[test]
    fn a_low_certainty_fingerprint_names_no_os() {
        let params = HashMap::from([
            ("os.family".to_string(), "Linux".to_string()),
            ("os.certainty".to_string(), "0.5".to_string()),
        ]);
        assert_eq!(os_from_params(&params), None);
    }

    #[test]
    fn a_device_info_model_names_macos() {
        let os = RecogDatabase::MdnsDeviceInfo
            .os("model=MacBookPro18,3")
            .expect("a Mac model names an OS");
        assert_eq!(os.family, HostOsFamily::MacOs);
    }
}
