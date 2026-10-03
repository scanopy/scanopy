//! Scanopy's own OS matches, for strings the vendored Recog databases leave unnamed.
//!
//! Consulted only after Recog names no OS for a string ([`RecogDatabase::os`]). Each entry states
//! the primary sources its pattern and its mapping rest on; an entry without one does not belong
//! here. Every entry claims no more than its sources support: a family, and a release or kernel
//! where the string carries one.

use std::sync::LazyLock;

use regex::Regex;

use super::recog::RecogDatabase;
use super::{HostOs, HostOsFamily};

/// The OS a string names, from the entry for its database, when Recog named none.
pub(super) fn os(database: RecogDatabase, input: &str) -> Option<HostOs> {
    match database {
        RecogDatabase::SnmpSysDescr => windows_ten_sys_descr(input)
            .or_else(|| net_snmp_linux_sys_descr(input))
            .or_else(|| pfsense_sys_descr(input)),
        RecogDatabase::HttpServer => iis_ten_server_header(input),
        RecogDatabase::MdnsDeviceInfo => mac_model_identifier(input),
        RecogDatabase::SnmpSysObjectId | RecogDatabase::SshBanner | RecogDatabase::ApacheOs => None,
    }
}

fn pattern(source: &str) -> Regex {
    Regex::new(source).expect("supplement patterns are fixed and tested")
}

/// Windows 10, Windows 11 and Windows Server 2016 and later, through the Windows SNMP service.
/// Family Windows and version `10.0.<build>`; no product name.
///
/// - Format: the SNMP service reports `Software: Windows Version 6.3 (Build <build> …)` on these
///   releases. A Windows 10 machine whose `ver` printed `10.0.10240` reported
///   `Hardware: Intel64 Family 6 Model 44 Stepping 2 AT/AT COMPATIBLE - Software: Windows Version
///   6.3 (Build 10240 Multiprocessor Free)`:
///   <https://learn.microsoft.com/en-us/archive/msdn-technet-forums/00c9bdf5-8b3b-48ed-b03a-f4fd83ca4fbf>.
///   Recog's own Windows fingerprints use the same layout up to build 9600.
/// - Why 6.3, and why 10.0: Windows 10, 11 and Server 2016/2019/2022 are version 10.0, and a process
///   manifested only for Windows 8.1 is told 6.3:
///   <https://learn.microsoft.com/en-us/windows/win32/sysinfo/operating-system-version>. The last
///   6.3 release is build 9600 (Windows 8.1 / Server 2012 R2), so any later build is a 10.0 release.
/// - No product name, because a build does not identify one: 17763 is both Windows 10 1809 and
///   Windows Server 2019, and 26100 both Windows 11 24H2 and Windows Server 2025:
///   <https://learn.microsoft.com/en-us/windows/release-health/release-information>,
///   <https://learn.microsoft.com/en-us/windows/release-health/windows11-release-information>,
///   <https://learn.microsoft.com/en-us/windows/release-health/windows-server-release-info>.
fn windows_ten_sys_descr(input: &str) -> Option<HostOs> {
    static PATTERN: LazyLock<Regex> =
        LazyLock::new(|| pattern(r"^Hardware: .* - Software: Windows Version 6\.3 \(Build (\d+) "));
    let build: u32 = PATTERN.captures(input)?.get(1)?.as_str().parse().ok()?;
    if build <= 9600 {
        return None;
    }
    Some(HostOs {
        version: Some(format!("10.0.{build}")),
        ..HostOs::family_only(HostOsFamily::Windows)
    })
}

/// Linux, through net-snmp's default sysDescr. Family and kernel release only.
///
/// - Format: net-snmp builds the default sysDescr from `uname(2)` as `"%s %s %s %s %s"` of sysname,
///   nodename, release, version and machine:
///   <https://github.com/net-snmp/net-snmp/blob/master/agent/mibgroup/mibII/system_mib.c>.
/// - A Linux kernel's sysname is `Linux` (`UTS_SYSNAME`):
///   <https://github.com/torvalds/linux/blob/master/include/linux/uts.h>, and its version begins
///   `#<build>`, then its SMP and PREEMPT flags and its build timestamp (`UTS_VERSION`):
///   <https://github.com/torvalds/linux/blob/master/init/Makefile>.
/// - An operator can replace the string with snmpd.conf's `sysdescr`. The pattern requires the
///   kernel's own release and `#` version layout, so ordinary free text that starts with "Linux"
///   does not match.
/// - Recog rates its generic Linux fingerprints `os.certainty` 0.5 and gives no reason; they report
///   the kernel release as the product version and name no distribution. This claims only the
///   family the kernel reports, and the release as the kernel's.
fn net_snmp_linux_sys_descr(input: &str) -> Option<HostOs> {
    static PATTERN: LazyLock<Regex> =
        LazyLock::new(|| pattern(r"^Linux \S+ (\d+\.\d+\S*) #\S+ .+ \S+$"));
    let release = PATTERN.captures(input)?.get(1)?.as_str().to_string();
    Some(HostOs {
        kernel_version: Some(release),
        ..HostOs::family_only(HostOsFamily::Linux)
    })
}

/// pfSense, which writes its own sysDescr rather than leaving it to the SNMP daemon. Family FreeBSD,
/// product pfSense and the pfSense version.
///
/// - Format: pfSense sets bsnmpd's sysDescr to `"<product_label> <hostname>.<domain>
///   <product_version_string> " . php_uname("s") . " " . php_uname("r") . " " . php_uname("m")`
///   (`src/etc/inc/services.inc`):
///   <https://github.com/pfsense/pfsense/blob/master/src/etc/inc/services.inc>.
/// - Recog's pfSense fingerprint accepts only a FreeBSD release ending `-RELEASE` or `-STABLE`, and
///   pfSense 2.7 runs FreeBSD 14.0-CURRENT and 2.8 runs 15.0-CURRENT:
///   <https://docs.netgate.com/pfsense/en/latest/releases/versions.html>.
fn pfsense_sys_descr(input: &str) -> Option<HostOs> {
    static PATTERN: LazyLock<Regex> =
        LazyLock::new(|| pattern(r"^pfSense \S+ (\S+) FreeBSD \S+ \S+$"));
    let version = PATTERN.captures(input)?.get(1)?.as_str().to_string();
    Some(HostOs {
        name: Some("pfSense".to_string()),
        version: Some(version),
        ..HostOs::family_only(HostOsFamily::FreeBsd)
    })
}

/// A Mac's mDNS device-info record, by its model identifier. Family macOS and nothing more.
///
/// Recog lists exact identifiers up to `MacBookPro18,x` and `iMac20,1`. Every Mac Apple has shipped
/// since 2022 is identified as `Mac<n>,<m>`, which Recog has none of: MacBook Air `Mac14,2` to
/// `Mac17,4`, MacBook Pro `Mac14,5` to `Mac17,9`, Mac mini `Mac14,3` to `Mac18,5`:
/// <https://support.apple.com/en-us/102869>, <https://support.apple.com/en-us/108052>,
/// <https://support.apple.com/en-us/102852>. The macOS release is not in the identifier, so none
/// is claimed.
fn mac_model_identifier(input: &str) -> Option<HostOs> {
    static PATTERN: LazyLock<Regex> = LazyLock::new(|| pattern(r"^model=Mac\d+,\d+$"));
    PATTERN
        .is_match(input)
        .then(|| HostOs::family_only(HostOsFamily::MacOs))
}

/// IIS 10.0 runs only on Windows. Family Windows and nothing more.
///
/// Recog leaves `Microsoft-IIS/10.0` without OS params because IIS 10.0 spans several Windows
/// releases. IIS 10.0 shipped with Windows 10 and Windows Server 2016, and IIS 10.0 version 1809
/// with the Windows 10 October 2018 Update and Windows Server 2019:
/// <https://learn.microsoft.com/en-us/iis/get-started/whats-new-in-iis-10/new-features-introduced-in-iis-10>,
/// <https://learn.microsoft.com/en-us/iis/get-started/whats-new-in-iis-10-version-1809/new-features-introduced-in-iis-10-1809>.
fn iis_ten_server_header(input: &str) -> Option<HostOs> {
    (input == "Microsoft-IIS/10.0").then(|| HostOs::family_only(HostOsFamily::Windows))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The string from the Microsoft forum thread cited on the entry.
    const WINDOWS_10_10240: &str = "Hardware: Intel64 Family 6 Model 44 Stepping 2 AT/AT COMPATIBLE - Software: Windows Version 6.3 (Build 10240 Multiprocessor Free)";

    #[test]
    fn windows_ten_and_later_read_as_version_ten_by_build() {
        let os = RecogDatabase::SnmpSysDescr.os(WINDOWS_10_10240).unwrap();
        assert_eq!(os.family, HostOsFamily::Windows);
        assert_eq!(os.version.as_deref(), Some("10.0.10240"));
        assert_eq!(os.name, None);
    }

    #[test]
    fn build_9600_is_left_to_recog() {
        let windows_8_1 = "Hardware: Intel64 Family 6 Model 44 Stepping 2 AT/AT COMPATIBLE - Software: Windows Version 6.3 (Build 9600 Multiprocessor Free)";
        assert_eq!(windows_ten_sys_descr(windows_8_1), None);
    }

    #[test]
    fn a_net_snmp_linux_description_names_the_family_and_kernel() {
        // net-snmp's "sysname nodename release version machine", with the kernel's version layout.
        let os = RecogDatabase::SnmpSysDescr
            .os("Linux nas-01 6.8.0-45-generic #45-Ubuntu SMP PREEMPT_DYNAMIC Fri Aug 30 12:02:04 UTC 2024 x86_64")
            .unwrap();
        assert_eq!(os.family, HostOsFamily::Linux);
        assert_eq!(os.kernel_version.as_deref(), Some("6.8.0-45-generic"));
        assert_eq!(os.name, None);
    }

    #[test]
    fn free_text_starting_with_linux_is_not_a_net_snmp_description() {
        assert_eq!(
            net_snmp_linux_sys_descr("Linux file server in rack 3"),
            None
        );
    }

    #[test]
    fn pfsense_on_a_current_freebsd_reads_as_pfsense() {
        // pfSense's own "<label> <host>.<domain> <version> <uname -s> <uname -r> <uname -m>".
        let os = RecogDatabase::SnmpSysDescr
            .os("pfSense fw01.example.lan 2.7.2-RELEASE FreeBSD 14.0-CURRENT amd64")
            .unwrap();
        assert_eq!(os.family, HostOsFamily::FreeBsd);
        assert_eq!(os.name.as_deref(), Some("pfSense"));
        assert_eq!(os.version.as_deref(), Some("2.7.2-RELEASE"));
    }

    #[test]
    fn a_current_mac_identifier_reads_as_macos_and_an_older_one_stays_recogs() {
        let os = RecogDatabase::MdnsDeviceInfo.os("model=Mac15,3").unwrap();
        assert_eq!(os, HostOs::family_only(HostOsFamily::MacOs));
        assert_eq!(mac_model_identifier("model=Macmini9,1"), None);
        assert_eq!(
            RecogDatabase::MdnsDeviceInfo
                .os("model=Macmini9,1")
                .map(|os| os.family),
            Some(HostOsFamily::MacOs)
        );
    }

    #[test]
    fn iis_ten_reads_as_windows_through_the_header_database() {
        let os = RecogDatabase::HttpServer.os("Microsoft-IIS/10.0").unwrap();
        assert_eq!(os, HostOs::family_only(HostOsFamily::Windows));
    }

    /// Recog first: where Recog names the OS, its more specific reading stands even though the
    /// broad Linux entry here matches the same string. This example is Recog's own, for its Red Hat
    /// Enterprise Linux 3 fingerprint (os.certainty 0.9).
    #[test]
    fn a_string_recog_names_keeps_recogs_reading() {
        let rhel = "Linux hostname 2.4.21-27.0.2.ELsmp #1 SMP Wed Jan 12 23:35:44 EST 2005 i686";
        assert!(net_snmp_linux_sys_descr(rhel).is_some());
        let os = RecogDatabase::SnmpSysDescr.os(rhel).unwrap();
        assert_eq!(os.name.as_deref(), Some("Enterprise Linux"));
        assert_eq!(os.version.as_deref(), Some("3"));
    }
}
