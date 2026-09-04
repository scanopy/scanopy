//! SSRF protection for server-originated requests to daemon URLs.
//!
//! In cloud deployments the server must never be tricked into issuing requests
//! to internal/metadata addresses via an attacker-influenced daemon URL. In
//! self-hosted deployments the server legitimately reaches LAN daemons, so the
//! guard is a no-op there.
use std::net::IpAddr;

use crate::server::config::DeploymentType;

/// Check if an address is private/loopback/internal (SSRF protection).
///
/// Canonicalizes IPv4-mapped IPv6 (`::ffff:a.b.c.d`) to V4 first so a mapped
/// literal can't slip past the V4 rules, and rejects the ranges an attacker
/// would use to reach internal services: RFC1918 private, loopback, link-local
/// (incl. the `169.254.169.254` cloud-metadata endpoint), unspecified,
/// broadcast, documentation, CGNAT/shared (`100.64.0.0/10`), plus IPv6 ULA
/// (`fc00::/7`) and link-local (`fe80::/10`).
pub(crate) fn is_private_ip(addr: &IpAddr) -> bool {
    match addr.to_canonical() {
        IpAddr::V4(ip) => {
            let o = ip.octets();
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                || ip.is_documentation()
                // CGNAT / shared address space: 100.64.0.0/10
                || (o[0] == 100 && (o[1] & 0xC0) == 0x40)
        }
        IpAddr::V6(ip) => {
            let first = ip.segments()[0];
            ip.is_loopback()
                || ip.is_unspecified()
                // Unique local addresses: fc00::/7
                || (first & 0xfe00) == 0xfc00
                // Link-local unicast: fe80::/10
                || (first & 0xffc0) == 0xfe80
        }
    }
}

/// In cloud deployments, reject a daemon URL whose host resolves to any internal
/// address. No-op for self-hosted deployments. Call before issuing a
/// server-originated request to a daemon-controlled URL.
pub(crate) async fn guard_daemon_url(
    url: &str,
    deployment_type: DeploymentType,
) -> anyhow::Result<()> {
    if deployment_type != DeploymentType::Cloud {
        return Ok(());
    }

    let parsed = url::Url::parse(url).map_err(|_| anyhow::anyhow!("Invalid daemon URL"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Daemon URL has no host"))?;
    let port = parsed.port_or_known_default().unwrap_or(80);

    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| anyhow::anyhow!("Could not resolve daemon host"))?;

    for addr in addrs {
        if is_private_ip(&addr.ip()) {
            anyhow::bail!(
                "Daemon URL resolves to a private/internal address (blocked in cloud mode)"
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn blocks_internal_ranges() {
        // Each of these is a range an SSRF attacker would use to reach an
        // internal service; the classifier must treat them all as private.
        let blocked = [
            "127.0.0.1",              // loopback
            "10.1.2.3",               // RFC1918 /8
            "172.16.5.9",             // RFC1918 /12
            "192.168.1.1",            // RFC1918 /16
            "169.254.169.254",        // link-local / cloud metadata endpoint
            "0.0.0.0",                // unspecified
            "255.255.255.255",        // broadcast
            "100.64.0.1",             // CGNAT / shared 100.64.0.0/10
            "100.127.255.254",        // CGNAT upper edge
            "::1",                    // IPv6 loopback
            "::",                     // IPv6 unspecified
            "fc00::1",                // IPv6 ULA fc00::/7
            "fd12:3456::1",           // IPv6 ULA (fd prefix)
            "fe80::1",                // IPv6 link-local fe80::/10
            "::ffff:10.0.0.1",        // IPv4-mapped IPv6 of an RFC1918 addr
            "::ffff:169.254.169.254", // IPv4-mapped metadata endpoint
        ];
        for s in blocked {
            assert!(is_private_ip(&ip(s)), "{s} should be classified private");
        }
    }

    #[test]
    fn allows_public_addresses() {
        // Public addresses must pass so cloud can reach legitimate daemons.
        let allowed = [
            "8.8.8.8",              // public
            "1.1.1.1",              // public
            "93.184.216.34",        // public (example.com range)
            "100.63.255.255",       // just below CGNAT
            "100.128.0.0",          // just above CGNAT
            "2606:4700:4700::1111", // public IPv6 (Cloudflare)
        ];
        for s in allowed {
            assert!(!is_private_ip(&ip(s)), "{s} should be classified public");
        }
    }

    #[tokio::test]
    async fn guard_is_noop_for_self_hosted() {
        // Self-hosted deployments legitimately reach LAN daemons — the guard
        // must never block them, even for a private URL.
        assert!(
            guard_daemon_url("http://192.168.1.50:8080", DeploymentType::SelfHosted)
                .await
                .is_ok(),
            "guard must be a no-op for self-hosted deployments"
        );
    }

    #[tokio::test]
    async fn guard_blocks_private_literal_in_cloud() {
        // A private IP literal resolves to itself, so cloud mode must reject it.
        assert!(
            guard_daemon_url("http://127.0.0.1:8080", DeploymentType::Cloud)
                .await
                .is_err()
        );
        assert!(
            guard_daemon_url(
                "http://169.254.169.254/latest/meta-data",
                DeploymentType::Cloud
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn guard_allows_public_literal_in_cloud() {
        assert!(
            guard_daemon_url("http://8.8.8.8:443", DeploymentType::Cloud)
                .await
                .is_ok()
        );
    }
}
