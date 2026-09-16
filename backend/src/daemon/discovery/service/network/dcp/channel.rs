//! A seam around the raw datalink channel, purely for testability.
//!
//! `arp/broadcast.rs` calls `pnet::datalink::channel` directly and has zero test coverage of its
//! send/receive concurrency as a result (confirmed by reading it — its tests cover only packet
//! construction). This trait wraps the *same* construction ([`PnetDcpChannel::open`] calls
//! `pnet::datalink::channel` exactly as ARP does — no second way to open a raw socket), so the
//! collect loop in `identify.rs` can be driven by a scripted fake in tests instead of a real NIC.

use std::time::Duration;

use anyhow::{Result, anyhow};
use pnet::datalink::{self, Channel, NetworkInterface};

/// One raw Ethernet link: send a frame, receive one with the channel's configured timeout.
pub trait DcpChannel: Send {
    fn send(&mut self, frame: &[u8]) -> std::io::Result<()>;

    /// Blocks up to the channel's read timeout. `Ok(None)` means the timeout elapsed with
    /// nothing received — not an error, the ordinary "nothing arrived yet" case a listen window
    /// spends most of its time in.
    fn recv(&mut self) -> std::io::Result<Option<Vec<u8>>>;
}

/// The production implementation: a real raw-socket channel on a real interface.
pub struct PnetDcpChannel {
    tx: Box<dyn datalink::DataLinkSender>,
    rx: Box<dyn datalink::DataLinkReceiver>,
}

impl PnetDcpChannel {
    pub fn open(interface: &NetworkInterface, read_timeout: Duration) -> Result<Self> {
        let config = datalink::Config {
            read_timeout: Some(read_timeout),
            read_buffer_size: 65536,
            write_buffer_size: 65536,
            ..Default::default()
        };

        match datalink::channel(interface, config)? {
            Channel::Ethernet(tx, rx) => Ok(Self { tx, rx }),
            _ => Err(anyhow!("Unsupported channel type")),
        }
    }
}

impl DcpChannel for PnetDcpChannel {
    fn send(&mut self, frame: &[u8]) -> std::io::Result<()> {
        match self.tx.send_to(frame, None) {
            Some(result) => result,
            None => Err(std::io::Error::other(
                "datalink sender declined to send (unsupported on this channel)",
            )),
        }
    }

    fn recv(&mut self) -> std::io::Result<Option<Vec<u8>>> {
        match self.rx.next() {
            Ok(packet) => Ok(Some(packet.to_vec())),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(None),
            // pnet also surfaces a plain WouldBlock for "nothing yet" on some platforms.
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// A scripted, in-memory fake for testing the collect loop without a socket.
///
/// Records every frame sent; replays a fixed, ordered script of incoming frames, one per `recv()`
/// call, and returns `Ok(None)` (the real "timeout, nothing arrived" case) for every call once the
/// script is exhausted — modeling a listen window that runs out.
#[cfg(test)]
pub struct ScriptedDcpChannel {
    pub sent: Vec<Vec<u8>>,
    incoming: std::collections::VecDeque<Vec<u8>>,
}

#[cfg(test)]
impl ScriptedDcpChannel {
    pub fn new(incoming: Vec<Vec<u8>>) -> Self {
        Self {
            sent: Vec::new(),
            incoming: incoming.into(),
        }
    }
}

#[cfg(test)]
impl DcpChannel for ScriptedDcpChannel {
    fn send(&mut self, frame: &[u8]) -> std::io::Result<()> {
        self.sent.push(frame.to_vec());
        Ok(())
    }

    fn recv(&mut self) -> std::io::Result<Option<Vec<u8>>> {
        Ok(self.incoming.pop_front())
    }
}
