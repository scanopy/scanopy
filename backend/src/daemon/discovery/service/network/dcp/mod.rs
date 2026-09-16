//! PROFINET DCP Identify — finds devices with no IP address at all.
//!
//! Raw Ethernet (EtherType `0x8892`, multicast `01:0E:CF:00:00:00`): one multicast burst per
//! interface, followed by a listen window during which an unknown number of devices reply. No
//! target list, no IP involved on either side — unlike ARP/ICMP/mDNS, which all answer "is this
//! specific address alive". `arp/broadcast.rs` is the template for the raw-socket transport
//! (`channel.rs` wraps the identical `pnet::datalink::channel` construction); shaped like
//! `mdns::browse` for timing (one burst, one fixed window) rather than ARP's multi-round retry,
//! since there is nothing to retry against.
//!
//! | Platform | Method | Fallback |
//! |---|---|---|
//! | Linux | Raw Ethernet (pnet) | none |
//! | macOS | Raw Ethernet (pnet) | none |
//! | Windows | Raw Ethernet via Npcap | none |
//!
//! No fallback anywhere, unlike ARP's `SendARP`: there is no OS syscall for reading raw Ethernet
//! frames with a custom EtherType, on any platform. [`is_available`] gates the phase off cleanly
//! wherever raw-socket access isn't there; the daemon simply does not run it, the same way ARP's
//! own broadcast path already behaves before *it* falls back to something else.
//!
//! Verification honesty (no PROFINET responder exists in this environment — see
//! `tools/dcp/DCP-TEST-ENV.md`): frame construction and response parsing are unit-tested over byte
//! arrays (`packet.rs`); the collect loop's dedup/parse/reject behaviour is unit-tested against a
//! scripted fake channel (`identify.rs`, `channel.rs`). Whether a real device answers this exact
//! frame, and the real-world listen-window timing, are unverified — ships as a claim to falsify,
//! not a given.

mod channel;
mod identify;
mod packet;

pub use identify::{is_available, is_dcp_capable, scan_interface};
pub use packet::DcpIdentifyResponse;
