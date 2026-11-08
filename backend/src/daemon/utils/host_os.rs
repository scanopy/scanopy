//! The OS of the machine the daemon runs on, as the daemon reads it locally.
//!
//! In a container this is the image's OS (its os-release) with the host's kernel, which is what
//! the daemon runs in. It outranks the OS a Docker or Podman engine reports, because the engine
//! may run in a VM on this machine (Docker Desktop, OrbStack, Colima) and describe the VM.

use crate::daemon::discovery::types::warnings::DesktopVm;
use crate::server::daemons::r#impl::base::DaemonOs;
use crate::server::hosts::r#impl::os::{HostOs, HostOsFamily};

/// Read this machine's OS: `os_info` for the product and release, `uname` for the kernel.
pub fn own_host_os() -> HostOs {
    let info = os_info::get();
    let mut os = HostOs::from(&info);
    os.kernel_version = kernel_release();
    os
}

impl From<&os_info::Info> for HostOs {
    fn from(info: &os_info::Info) -> Self {
        use os_info::Type as T;
        let family = match info.os_type() {
            T::Windows | T::Cygwin => HostOsFamily::Windows,
            T::Macos => HostOsFamily::MacOs,
            T::Ios => HostOsFamily::Ios,
            T::FreeBSD | T::HardenedBSD | T::MidnightBSD => HostOsFamily::FreeBsd,
            T::OpenBSD => HostOsFamily::OpenBsd,
            T::NetBSD => HostOsFamily::NetBsd,
            // No daemon is built for these, and `os_info` could not tell which machine it ran on,
            // so the binary's own target, which is always known, names the family.
            T::Unknown
            | T::AIX
            | T::Android
            | T::DragonFly
            | T::Emscripten
            | T::Hurd
            | T::Illumos
            | T::Redox => HostOsFamily::from(DaemonOs::current()),
            // Every remaining type, and every one `os_info` adds, is a Linux distribution.
            _ => HostOsFamily::Linux,
        };
        // The distribution names a Linux host ("Ubuntu"); a generic "Linux" says nothing more
        // than the family. Windows and macOS are named by their family.
        let name = (family == HostOsFamily::Linux && info.os_type() != T::Linux)
            .then(|| info.os_type().to_string());
        let version = match info.version() {
            os_info::Version::Unknown => None,
            os_info::Version::Rolling(_) => None,
            v => Some(v.to_string()),
        };
        HostOs {
            family,
            name,
            version,
            edition: info.edition().map(str::to_string),
            codename: info.codename().map(str::to_string),
            kernel_version: None,
        }
    }
}

/// The desktop container VM this daemon runs inside, if any: Docker Desktop, OrbStack or Colima.
///
/// Docker Desktop and OrbStack name themselves in the kernel release (`6.12.54-linuxkit`,
/// `6.12.0-orbstack-00302-g8a5c3f`), so the daemon's own `uname` is enough. Colima runs a stock
/// distribution kernel, so it is named only by the engine's `info`: its `Name` is the VM's hostname,
/// `colima` or `colima-<profile>`. The engine's `info` counts only when its kernel is the daemon's
/// own, which is what makes the daemon a container in that VM. A daemon installed on the machine
/// itself can reach the same engine through a socket, and it sees the LAN.
pub fn desktop_vm(
    kernel_release: &str,
    engine_info: Option<&bollard::models::SystemInfo>,
) -> Option<DesktopVm> {
    let release = kernel_release.to_ascii_lowercase();
    if release.contains("linuxkit") {
        return Some(DesktopVm::DockerDesktop);
    }
    if release.contains("orbstack") {
        return Some(DesktopVm::OrbStack);
    }
    let info = engine_info.filter(|info| info.kernel_version.as_deref() == Some(kernel_release))?;
    match info.operating_system.as_deref().map(str::trim) {
        Some("Docker Desktop") => return Some(DesktopVm::DockerDesktop),
        Some("OrbStack") => return Some(DesktopVm::OrbStack),
        _ => {}
    }
    info.name
        .as_deref()
        .filter(|name| *name == "colima" || name.starts_with("colima-"))
        .map(|_| DesktopVm::Colima)
}

/// This machine's kernel release, as `uname -r` prints it.
#[cfg(unix)]
pub fn kernel_release() -> Option<String> {
    // SAFETY: `uname` fills the struct it is handed and the fields are NUL-terminated C strings.
    let mut name: libc::utsname = unsafe { std::mem::zeroed() };
    if unsafe { libc::uname(&mut name) } != 0 {
        return None;
    }
    let release = unsafe { std::ffi::CStr::from_ptr(name.release.as_ptr()) };
    let release = release.to_string_lossy().trim().to_string();
    (!release.is_empty()).then_some(release)
}

/// Windows carries its build in the version `os_info` already read.
#[cfg(not(unix))]
pub fn kernel_release() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_distribution_names_a_linux_host() {
        let info = os_info::Info::with_type(os_info::Type::Ubuntu);
        let os = HostOs::from(&info);
        assert_eq!(os.name.as_deref(), Some("Ubuntu"));
    }

    fn engine(operating_system: &str, name: &str, kernel: &str) -> bollard::models::SystemInfo {
        bollard::models::SystemInfo {
            operating_system: Some(operating_system.to_string()),
            name: Some(name.to_string()),
            kernel_version: Some(kernel.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn the_desktop_vms_name_themselves_in_the_kernel_release() {
        assert_eq!(
            desktop_vm("6.12.54-linuxkit", None),
            Some(DesktopVm::DockerDesktop)
        );
        assert_eq!(
            desktop_vm("6.12.0-orbstack-00302-g8a5c3f2b1e9d", None),
            Some(DesktopVm::OrbStack)
        );
    }

    #[test]
    fn the_engine_names_the_vm_the_daemon_shares_a_kernel_with() {
        let ubuntu = "6.8.0-50-generic";
        assert_eq!(
            desktop_vm(
                ubuntu,
                Some(&engine("Ubuntu 24.04.1 LTS", "colima", ubuntu))
            ),
            Some(DesktopVm::Colima)
        );
        assert_eq!(
            desktop_vm(
                ubuntu,
                Some(&engine("Ubuntu 24.04.1 LTS", "colima-work", ubuntu))
            ),
            Some(DesktopVm::Colima)
        );
        assert_eq!(
            desktop_vm(
                ubuntu,
                Some(&engine("Docker Desktop", "docker-desktop", ubuntu))
            ),
            Some(DesktopVm::DockerDesktop)
        );
        assert_eq!(
            desktop_vm(ubuntu, Some(&engine("OrbStack", "orbstack", ubuntu))),
            Some(DesktopVm::OrbStack)
        );
    }

    #[test]
    fn a_daemon_on_the_machine_itself_is_not_in_the_vm_its_engine_runs_in() {
        // macOS, talking to Docker Desktop's socket.
        let vm = engine("Docker Desktop", "docker-desktop", "6.12.54-linuxkit");
        assert_eq!(desktop_vm("24.1.0", Some(&vm)), None);
        // A Linux host, talking to a Colima socket.
        let colima = engine("Ubuntu 24.04.1 LTS", "colima", "6.8.0-50-generic");
        assert_eq!(desktop_vm("6.11.0-19-generic", Some(&colima)), None);
    }

    #[test]
    fn a_plain_linux_host_is_in_no_vm() {
        let kernel = "6.8.0-45-generic";
        assert_eq!(desktop_vm(kernel, None), None);
        assert_eq!(
            desktop_vm(
                kernel,
                Some(&engine("Ubuntu 24.04.1 LTS", "nas-01", kernel))
            ),
            None
        );
        // A hostname that only starts with the word is not the VM.
        assert_eq!(
            desktop_vm(
                kernel,
                Some(&engine("Debian GNU/Linux 12", "colimastore", kernel))
            ),
            None
        );
    }

    #[test]
    fn this_machine_reads_as_the_family_the_binary_was_built_for() {
        assert_eq!(
            own_host_os().family,
            HostOsFamily::from(DaemonOs::current())
        );
    }
}
