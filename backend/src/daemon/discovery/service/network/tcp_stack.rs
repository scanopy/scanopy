//! An operating system family read from how a host's TCP/IP stack answers a connection.
//!
//! The port scan opens connections with the kernel's own `connect()`, so the SYN is ours but the
//! SYN-ACK that answers it is entirely the target's stack: its initial TTL, receive window, MSS,
//! window scale and the order it lays its TCP options out in differ between Linux, Windows and the
//! BSDs. A datalink listener on each scanning interface records the first SYN-ACK every address
//! sends while the scan runs, and [`TcpStackCapture::os_family`] reads it against [`SIGNATURES`].
//!
//! The weakest OS reading there is: it names only a family, and is offered through
//! `HostData::offer_fallback_os`, which applies it to a host nothing else names an OS for and never
//! lets it contradict a banner.
//!
//! The signature table is Scanopy's own. Every entry comes from a SYN-ACK captured off a host whose
//! OS we know, kept as bytes in the tests; none is copied from p0f (LGPL) or Nmap (NPSL) data. A
//! SYN-ACK no entry matches is logged at debug with its fingerprint, so a stack the table lacks can
//! be added from what a real scan saw.
//!
//! Needs the raw access ARP's broadcast sweep needs (`arp::broadcast::is_available`). Where a
//! channel will not open, nothing is captured and every host simply gets no reading from here.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pnet::datalink::{self, Channel, DataLinkReceiver, NetworkInterface};
use pnet::packet::Packet;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::tcp::{TcpFlags, TcpOptionNumbers, TcpPacket};

use crate::server::hosts::r#impl::os::HostOsFamily;
use crate::server::subnets::r#impl::base::Subnet;

/// How long one receive waits before the listener checks whether it has been stopped.
const READ_TIMEOUT: Duration = Duration::from_millis(250);

/// What a SYN-ACK says about the stack that sent it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpStackFingerprint {
    /// The TTL or hop limit the stack started from: the received value rounded up to the nearest
    /// common initial value, since each router on the way took one off.
    pub initial_ttl: u8,
    pub window: u16,
    pub mss: Option<u16>,
    pub window_scale: Option<u8>,
    /// The TCP options in the order they were sent: `M` MSS, `N` no-op, `W` window scale, `S` SACK
    /// permitted, `T` timestamps, `E` end of list, `?` anything else.
    pub options: String,
    /// IPv4's Don't Fragment bit. Always `false` over IPv6, which has none.
    pub dont_fragment: bool,
}

/// One known stack: the values its SYN-ACK carries, and the family that sends them.
pub struct Signature {
    pub family: HostOsFamily,
    pub initial_ttl: u8,
    pub options: &'static str,
    /// `None` where the stack's window varies with its configuration and so says nothing.
    pub window: Option<u16>,
    pub window_scale: Option<u8>,
}

/// The stacks Scanopy recognises, each from a captured SYN-ACK (see the module doc). A
/// fingerprint names a family only when exactly one entry matches it.
pub const SIGNATURES: &[Signature] = &[];

impl TcpStackFingerprint {
    /// Read the SYN-ACK in an Ethernet frame, if it is one, with the address that sent it.
    pub fn from_frame(frame: &[u8]) -> Option<(IpAddr, Self)> {
        let ethernet = EthernetPacket::new(frame)?;
        match ethernet.get_ethertype() {
            EtherTypes::Ipv4 => {
                let ip = Ipv4Packet::new(ethernet.payload())?;
                if ip.get_next_level_protocol() != IpNextHeaderProtocols::Tcp {
                    return None;
                }
                let dont_fragment = ip.get_flags() & 0b010 != 0;
                let tcp = TcpPacket::new(ip.payload())?;
                Self::from_syn_ack(&tcp, ip.get_ttl(), dont_fragment)
                    .map(|fp| (IpAddr::V4(ip.get_source()), fp))
            }
            EtherTypes::Ipv6 => {
                let ip = Ipv6Packet::new(ethernet.payload())?;
                if ip.get_next_header() != IpNextHeaderProtocols::Tcp {
                    return None;
                }
                let tcp = TcpPacket::new(ip.payload())?;
                Self::from_syn_ack(&tcp, ip.get_hop_limit(), false)
                    .map(|fp| (IpAddr::V6(ip.get_source()), fp))
            }
            _ => None,
        }
    }

    fn from_syn_ack(tcp: &TcpPacket<'_>, ttl: u8, dont_fragment: bool) -> Option<Self> {
        let flags = tcp.get_flags();
        if flags & (TcpFlags::SYN | TcpFlags::ACK) != TcpFlags::SYN | TcpFlags::ACK {
            return None;
        }
        let mut options = String::new();
        let mut mss = None;
        let mut window_scale = None;
        for option in tcp.get_options_iter() {
            let number = option.get_number();
            let payload = option.payload();
            let letter = match number {
                TcpOptionNumbers::MSS => {
                    mss = payload
                        .get(..2)
                        .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]));
                    'M'
                }
                TcpOptionNumbers::NOP => 'N',
                TcpOptionNumbers::WSCALE => {
                    window_scale = payload.first().copied();
                    'W'
                }
                TcpOptionNumbers::SACK_PERMITTED => 'S',
                TcpOptionNumbers::TIMESTAMPS => 'T',
                TcpOptionNumbers::EOL => 'E',
                _ => '?',
            };
            options.push(letter);
        }
        Some(Self {
            initial_ttl: initial_ttl(ttl),
            window: tcp.get_window(),
            mss,
            window_scale,
            options,
            dont_fragment,
        })
    }

    /// The family exactly one signature names for this fingerprint.
    pub fn os_family(&self, signatures: &[Signature]) -> Option<HostOsFamily> {
        let mut families = signatures
            .iter()
            .filter(|s| {
                s.initial_ttl == self.initial_ttl
                    && s.options == self.options
                    && s.window.is_none_or(|w| w == self.window)
                    && s.window_scale
                        .is_none_or(|ws| Some(ws) == self.window_scale)
            })
            .map(|s| s.family);
        let first = families.next()?;
        families.all(|f| f == first).then_some(first)
    }
}

/// The common initial TTLs, smallest first: 32 (old Windows, some embedded stacks), 64 (Linux,
/// macOS, the BSDs), 128 (Windows), 255 (network equipment, Solaris).
fn initial_ttl(observed: u8) -> u8 {
    [32, 64, 128, 255]
        .into_iter()
        .find(|initial| observed <= *initial)
        .unwrap_or(255)
}

/// The interfaces a capture listens on: those that are up, not loopback, and carry an IPv4
/// address inside a subnet this scan covers. A SYN-ACK answering the scan arrives on one of
/// them. A veth (no IPv4) or a container bridge the session does not scan never does, and a
/// host running dozens of containers has one of each per container.
pub(crate) fn capture_interfaces(
    nics: Vec<NetworkInterface>,
    scanned_subnets: &[Subnet],
) -> Vec<NetworkInterface> {
    nics.into_iter()
        .filter(|nic| nic.is_up() && !nic.is_loopback())
        .filter(|nic| {
            nic.ips.iter().any(|ip| {
                ip.is_ipv4()
                    && scanned_subnets
                        .iter()
                        .any(|s| s.base.cidr.contains(&ip.ip()))
            })
        })
        .collect()
}

/// Read frames until `stop` is set, recording the first SYN-ACK from each address.
///
/// A timeout or an interrupted wait reads again. Any other error ends the loop and is returned:
/// pnet reports a socket the kernel flagged (its device went down or was deleted, as a veth is
/// on every container restart) as an error on every read without blocking, so reading on would
/// spin a core for the rest of the scan.
fn capture_loop(
    rx: &mut dyn DataLinkReceiver,
    stop: &AtomicBool,
    seen: &Mutex<HashMap<IpAddr, TcpStackFingerprint>>,
) -> Option<std::io::Error> {
    while !stop.load(Ordering::Relaxed) {
        let frame = match rx.next() {
            Ok(frame) => frame,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::WouldBlock
                ) =>
            {
                continue;
            }
            Err(e) => return Some(e),
        };
        if let Some((source, fingerprint)) = TcpStackFingerprint::from_frame(frame)
            && let Ok(mut seen) = seen.lock()
        {
            seen.entry(source).or_insert(fingerprint);
        }
    }
    None
}

/// The SYN-ACKs captured on the scanning interfaces while a scan runs: the first one from each
/// address. Listening stops when this is dropped.
pub struct TcpStackCapture {
    seen: Arc<Mutex<HashMap<IpAddr, TcpStackFingerprint>>>,
    stop: Arc<AtomicBool>,
}

impl TcpStackCapture {
    /// Start one listener thread per interface. An interface whose channel will not open (no raw
    /// access) is skipped with a debug log; with none open, the capture records nothing.
    ///
    /// A dedicated OS thread per interface, as DCP's and ARP's: `datalink::channel()` opens a raw
    /// fd, and opening it here, before the deep scans' fd-heavy work ramps up, keeps it low.
    pub fn start(interfaces: Vec<NetworkInterface>) -> Self {
        let seen = Arc::new(Mutex::new(HashMap::new()));
        let stop = Arc::new(AtomicBool::new(false));
        for interface in interfaces {
            let config = datalink::Config {
                read_timeout: Some(READ_TIMEOUT),
                read_buffer_size: 65536,
                ..Default::default()
            };
            let mut rx = match datalink::channel(&interface, config) {
                Ok(Channel::Ethernet(_, rx)) => rx,
                Ok(_) => continue,
                Err(e) => {
                    tracing::debug!(
                        interface = %interface.name,
                        error = %e,
                        "No SYN-ACK capture on this interface: the raw channel did not open"
                    );
                    continue;
                }
            };
            let seen = seen.clone();
            let stop = stop.clone();
            std::thread::spawn(move || {
                if let Some(e) = capture_loop(rx.as_mut(), &stop, &seen) {
                    tracing::debug!(
                        interface = %interface.name,
                        error = %e,
                        "SYN-ACK capture stopped on this interface: its channel returned an error"
                    );
                }
            });
        }
        Self { seen, stop }
    }

    /// The OS family the first SYN-ACK from `ip` names, if one arrived and a signature names it.
    pub fn os_family(&self, ip: IpAddr) -> Option<HostOsFamily> {
        let fingerprint = self.seen.lock().ok()?.get(&ip).cloned()?;
        let family = fingerprint.os_family(SIGNATURES);
        if family.is_none() {
            tracing::debug!(%ip, ?fingerprint, "TCP/IP stack matched no signature");
        }
        family
    }
}

impl Drop for TcpStackCapture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An Ethernet + IPv4 + TCP SYN-ACK with the given TTL, window and raw TCP options.
    fn syn_ack(ttl: u8, window: u16, options: &[u8]) -> Vec<u8> {
        let tcp_len = 20 + options.len();
        let mut frame = vec![0u8; 14 + 20 + tcp_len];
        frame[12..14].copy_from_slice(&[0x08, 0x00]);
        let ip = &mut frame[14..34];
        ip[0] = 0x45;
        ip[2..4].copy_from_slice(&((20 + tcp_len) as u16).to_be_bytes());
        ip[6] = 0x40; // Don't Fragment
        ip[8] = ttl;
        ip[9] = 6;
        ip[12..16].copy_from_slice(&[192, 168, 4, 54]);
        ip[16..20].copy_from_slice(&[192, 168, 4, 27]);
        let tcp = &mut frame[34..];
        tcp[12] = ((tcp_len / 4) as u8) << 4;
        tcp[13] = 0x12; // SYN, ACK
        tcp[14..16].copy_from_slice(&window.to_be_bytes());
        tcp[20..].copy_from_slice(options);
        frame
    }

    /// MSS 1460, SACK permitted, timestamps, NOP, window scale 7: the layout `M,S,T,N,W`.
    const MSTNW: [u8; 20] = [
        2, 4, 0x05, 0xb4, 4, 2, 8, 10, 0, 0, 0, 1, 0, 0, 0, 0, 1, 3, 3, 7,
    ];

    #[test]
    fn a_syn_ack_reads_into_its_fingerprint() {
        // Two hops from a stack that started at 64.
        let (source, fp) = TcpStackFingerprint::from_frame(&syn_ack(62, 65160, &MSTNW)).unwrap();
        assert_eq!(source, IpAddr::from([192, 168, 4, 54]));
        assert_eq!(fp.initial_ttl, 64);
        assert_eq!(fp.window, 65160);
        assert_eq!(fp.mss, Some(1460));
        assert_eq!(fp.window_scale, Some(7));
        assert_eq!(fp.options, "MSTNW");
        assert!(fp.dont_fragment);
    }

    #[test]
    fn a_frame_that_is_not_a_syn_ack_reads_as_nothing() {
        let mut frame = syn_ack(64, 65160, &MSTNW);
        frame[34 + 13] = 0x10; // ACK only
        assert_eq!(TcpStackFingerprint::from_frame(&frame), None);
        assert_eq!(TcpStackFingerprint::from_frame(&[0u8; 10]), None);
    }

    const IFF_UP: u32 = 0x1;

    fn nic(name: &str, ips: &[&str]) -> NetworkInterface {
        NetworkInterface {
            name: name.to_string(),
            description: String::new(),
            index: 0,
            mac: None,
            ips: ips.iter().map(|ip| ip.parse().unwrap()).collect(),
            flags: IFF_UP,
        }
    }

    /// A host running about sixty containers: one LAN NIC, the default bridge, a dozen
    /// per-network bridges and a veth per container. The session scans the LAN only, so the
    /// capture opens one socket.
    #[test]
    fn a_capture_listens_only_on_interfaces_carrying_a_scanned_subnet() {
        let mut nics = vec![
            nic("enp6s18", &["192.168.1.10/24", "fe80::1/64"]),
            nic("docker0", &["172.17.0.1/16", "fe80::2/64"]),
        ];
        for i in 0..12 {
            nics.push(nic(
                &format!("br-{:012x}", 0xabc000 + i),
                &[&format!("172.{}.0.1/16", 18 + i)],
            ));
        }
        for i in 0..46 {
            nics.push(nic(&format!("veth{i:07x}"), &["fe80::3/64"]));
        }
        let lan = Subnet::from_discovery(
            "enp6s18".to_string(),
            &"192.168.1.10/24".parse().unwrap(),
            uuid::Uuid::nil(),
            crate::server::shared::attribution::AttributeSource::DaemonSelfReport,
        )
        .unwrap();

        let names: Vec<String> = capture_interfaces(nics, &[lan])
            .into_iter()
            .map(|nic| nic.name)
            .collect();

        assert_eq!(names, vec!["enp6s18".to_string()]);
    }

    /// Returns `Other` on every read, as pnet does once the kernel has flagged the socket with
    /// an error (a veth deleted under it). Sets `stop` after `limit` reads so a loop that never
    /// gives up still ends the test.
    struct FailingReceiver {
        reads: usize,
        limit: usize,
        stop: Arc<AtomicBool>,
    }

    impl DataLinkReceiver for FailingReceiver {
        fn next(&mut self) -> std::io::Result<&[u8]> {
            self.reads += 1;
            if self.reads >= self.limit {
                self.stop.store(true, Ordering::Relaxed);
            }
            Err(std::io::Error::other("Unexpected poll event"))
        }
    }

    #[test]
    fn a_capture_stops_reading_an_interface_that_errors() {
        let stop = Arc::new(AtomicBool::new(false));
        let mut rx = FailingReceiver {
            reads: 0,
            limit: 1000,
            stop: stop.clone(),
        };

        let error = capture_loop(&mut rx, &stop, &Mutex::new(HashMap::new()));

        assert_eq!(rx.reads, 1);
        assert!(error.is_some());
    }

    #[test]
    fn a_family_needs_exactly_one_signature_to_agree() {
        let (_, fp) = TcpStackFingerprint::from_frame(&syn_ack(64, 65160, &MSTNW)).unwrap();
        let linux = Signature {
            family: HostOsFamily::Linux,
            initial_ttl: 64,
            options: "MSTNW",
            window: None,
            window_scale: Some(7),
        };
        let bsd = Signature {
            family: HostOsFamily::FreeBsd,
            initial_ttl: 64,
            options: "MSTNW",
            window: None,
            window_scale: None,
        };
        assert_eq!(
            fp.os_family(std::slice::from_ref(&linux)),
            Some(HostOsFamily::Linux)
        );
        assert_eq!(fp.os_family(&[linux, bsd]), None);
        assert_eq!(fp.os_family(&[]), None);
    }
}
