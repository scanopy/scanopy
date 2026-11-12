//! What the daemon runs on a libvirt host.
//!
//! One SSH exec runs one fixed POSIX `sh` script, so a host with dozens of domains costs one
//! round trip. Each part of its output starts with an `@@` marker line, which [`super::parse`]
//! splits on. `virsh` and `ip` print tables and JSON this daemon parses; nothing here interprets
//! them.

/// The `virsh` invocation for `uri`, read-only or with full access. The URI is quoted for the
/// shell, so a URI with spaces or quotes stays one argument.
pub fn virsh(uri: &str, read_only: bool) -> String {
    let mode = if read_only { " -r" } else { "" };
    format!("virsh -q{mode} -c {}", shell_quote(uri))
}

/// The command the probe runs: `virsh uri` answers with the connection it opened, or fails
/// when the account may not open it. Exit 127 means the host has no `virsh`.
pub fn probe(uri: &str, read_only: bool) -> String {
    format!("{} uri", virsh(uri, read_only))
}

/// The inventory script. `agent` adds the guest-agent reads, which libvirt refuses on a read-only
/// connection.
pub fn inventory(uri: &str, read_only: bool) -> String {
    let agent_reads = if read_only {
        ""
    } else {
        r#"
  echo "@@addr agent"; v domifaddr "$u" --source agent 2>/dev/null
  echo "@@guestinfo"; v guestinfo "$u" --os --hostname 2>/dev/null"#
    };
    format!(
        r#"v() {{ {virsh} "$@"; }}
echo "@@hostname"; hostname
echo "@@ip-addr"; ip -j addr 2>/dev/null || echo "[]"
for b in /sys/class/net/*/brif; do
  [ -d "$b" ] || continue
  echo "@@brif $(basename "$(dirname "$b")")"; ls "$b"
done
for n in $(v net-list --all --name 2>/dev/null); do
  echo "@@net $n"; v net-info "$n" 2>/dev/null
done
domains=$(v list --all --uuid) || {{ echo "@@list-failed"; exit 3; }}
echo "@@domains"
for u in $domains; do
  echo "@@domain $u"
  echo "@@name"; v domname "$u"
  echo "@@state"; v domstate "$u"
  echo "@@iflist"; v domiflist "$u" --inactive
  echo "@@addr lease"; v domifaddr "$u" --source lease 2>/dev/null
  echo "@@addr arp"; v domifaddr "$u" --source arp 2>/dev/null{agent_reads}
done
echo "@@end"
"#,
        virsh = virsh(uri, read_only),
    )
}

/// `value` as one single-quoted shell word.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A URI is one argument however it is written, so a crafted URI cannot add a command: the
    /// shell reads the quoted word back as exactly the URI.
    #[test]
    fn a_uri_stays_one_shell_word() {
        for uri in ["qemu:///system", "x'; touch /tmp/pwned #", "a b\"c'd"] {
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(format!("printf %s {}", shell_quote(uri)))
                .output()
                .expect("sh runs");
            assert_eq!(String::from_utf8_lossy(&output.stdout), uri);
        }
    }

    /// A read-only connection never asks for what libvirt refuses it.
    #[test]
    fn a_read_only_inventory_asks_nothing_of_the_guest_agent() {
        assert!(!inventory("qemu:///system", true).contains("--source agent"));
        assert!(inventory("qemu:///system", false).contains("--source agent"));
    }
}
