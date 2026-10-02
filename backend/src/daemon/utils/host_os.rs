//! The OS of the machine the daemon runs on, as the daemon reads it locally.
//!
//! In a container this is the image's OS (its os-release) with the host's kernel, which is what
//! the daemon runs in. A Docker integration reading the engine host replaces it, by rank.

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

#[cfg(unix)]
fn kernel_release() -> Option<String> {
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
fn kernel_release() -> Option<String> {
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

    #[test]
    fn this_machine_reads_as_the_family_the_binary_was_built_for() {
        assert_eq!(
            own_host_os().family,
            HostOsFamily::from(DaemonOs::current())
        );
    }
}
