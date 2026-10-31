//! What the QEMU guest agent reads inside a VM, whichever hypervisor relays it.
//!
//! Proxmox VE returns the agent's `guest-get-osinfo` reply as JSON under `result`; libvirt's
//! `virsh guestinfo --os` prints the same fields as `os.<field>` lines. Both parse into
//! [`OsInfo`], and the rules for reading an OS out of it live here.

use serde::Deserialize;

use crate::server::hosts::r#impl::os::{HostOs, HostOsFamily};

/// The guest's `/etc/os-release` (or the Windows equivalent), as the agent reads it. The serde
/// names are the agent's own.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct OsInfo {
    /// `debian`, `haos`, `mswindows`, ...
    #[serde(default)]
    pub id: Option<String>,
    /// `Debian GNU/Linux`.
    #[serde(default)]
    pub name: Option<String>,
    /// `12 (bookworm)`.
    #[serde(default)]
    pub version: Option<String>,
    /// `12`.
    #[serde(rename = "version-id", default)]
    pub version_id: Option<String>,
    /// `6.1.0-53-amd64`.
    #[serde(rename = "kernel-release", default)]
    pub kernel_release: Option<String>,
}

/// The OS the agent reads from inside the VM.
///
/// `id` says the family: `mswindows` for Windows, a BSD or Solaris by its own id, and anything
/// else is an os-release distribution id (`debian`, `haos`), which is Linux. The release is
/// `version-id`, dropped when the name already carries it. `version` repeats the release with a
/// parenthesised suffix, which is the codename when it is one lowercase word (`12 (bookworm)`)
/// and a variant otherwise (HAOS's `18.2 (Open Virtual Appliance)`), so only the first is kept.
pub fn os(info: &OsInfo) -> Option<HostOs> {
    let family = match info.id.as_deref().map(str::trim)? {
        "" => return None,
        "mswindows" => HostOsFamily::Windows,
        "freebsd" => HostOsFamily::FreeBsd,
        "openbsd" => HostOsFamily::OpenBsd,
        "netbsd" => HostOsFamily::NetBsd,
        "solaris" => HostOsFamily::Solaris,
        _ => HostOsFamily::Linux,
    };
    let name = info.name.clone();
    let (release, suffix) = match info.version.as_deref().map(str::trim) {
        Some(version) => match version.split_once('(') {
            Some((release, rest)) => (
                Some(release.trim().to_string()),
                rest.strip_suffix(')').map(str::trim),
            ),
            None => (Some(version.to_string()), None),
        },
        None => (None, None),
    };
    let version = info
        .version_id
        .clone()
        .filter(|v| !v.trim().is_empty())
        .or(release)
        .filter(|v| !name.as_deref().is_some_and(|n| n.contains(v.trim())));
    let codename = suffix
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase()))
        .filter(|s| info.version_id.as_deref() != Some(*s))
        .map(str::to_string);
    Some(
        HostOs {
            family,
            name,
            version,
            edition: None,
            codename,
            kernel_version: info.kernel_release.clone(),
        }
        .without_blank_fields(),
    )
}

/// The hostname the agent reads, when it names one.
pub fn hostname(name: Option<&str>) -> Option<String> {
    name.map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}
