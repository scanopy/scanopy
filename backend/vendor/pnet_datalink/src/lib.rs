// Copyright (c) 2014-2016 Robert Clipsham <robert@octarineparrot.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Support for sending and receiving data link layer packets.

#![deny(warnings)]

extern crate ipnetwork;
extern crate libc;
extern crate pnet_base;
extern crate pnet_sys;

#[cfg(feature = "serde")]
extern crate serde;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::io;
use std::option::Option;
use std::time::Duration;

use ipnetwork::IpNetwork;

pub use pnet_base::{MacAddr, ParseMacAddrErr};

mod bindings;

#[cfg(windows)]
#[path = "winpcap.rs"]
mod backend;

#[cfg(windows)]
pub mod winpcap;

#[cfg(all(
    not(feature = "netmap"),
    any(target_os = "linux", target_os = "android")
))]
#[path = "linux.rs"]
mod backend;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod linux;

#[cfg(all(
    not(feature = "netmap"),
    any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "illumos",
        target_os = "solaris",
        target_os = "macos",
        target_os = "ios",
        target_os = "tvos"
    )
))]
#[path = "bpf.rs"]
mod backend;

#[cfg(any(
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "illumos",
    target_os = "solaris",
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos"
))]
pub mod bpf;

#[cfg(feature = "netmap")]
#[path = "netmap.rs"]
mod backend;

#[cfg(feature = "netmap")]
pub mod netmap;

#[cfg(feature = "pcap")]
pub mod pcap;

pub mod dummy;

/// Type alias for an `EtherType`.
pub type EtherType = u16;

/// Type of data link channel to present (Linux only).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChannelType {
    /// Send and receive layer 2 packets directly, including headers.
    Layer2,
    /// Send and receive "cooked" packets - send and receive network layer packets.
    Layer3(EtherType),
}

/// A channel for sending and receiving at the data link layer.
#[non_exhaustive]
pub enum Channel {
    /// A datalink channel which sends and receives Ethernet packets.
    Ethernet(Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>),
}

/// Socket fanout type (Linux only).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FanoutType {
    HASH,
    LB,
    CPU,
    ROLLOVER,
    RND,
    QM,
    CBPF,
    EBPF,
}

/// Fanout settings (Linux only).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FanoutOption {
    pub group_id: u16,
    pub fanout_type: FanoutType,
    pub defrag: bool,
    pub rollover: bool,
}

/// A generic configuration type, encapsulating all options supported by each backend.
///
/// Each option should be treated as a hint - each backend is free to ignore any and all
/// options which don't apply to it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Config {
    /// The size of buffer to use when writing packets. Defaults to 4096.
    pub write_buffer_size: usize,

    /// The size of buffer to use when reading packets. Defaults to 4096.
    pub read_buffer_size: usize,

    /// Linux/BPF/Netmap only: The read timeout. Defaults to None.
    pub read_timeout: Option<Duration>,

    /// Linux/BPF/Netmap only: The write timeout. Defaults to None.
    pub write_timeout: Option<Duration>,

    /// Linux only: Specifies whether to read packets at the datalink layer or network layer.
    /// Defaults to Layer2
    pub channel_type: ChannelType,

    /// BPF/OS X only: The number of /dev/bpf* file descriptors to attempt before failing. Defaults
    /// to: 1000.
    pub bpf_fd_attempts: usize,

    pub linux_fanout: Option<FanoutOption>,

    pub promiscuous: bool,

    /// Linux only: The socket's file descriptor that pnet will use
    pub socket_fd: Option<i32>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            write_buffer_size: 4096,
            read_buffer_size: 4096,
            read_timeout: None,
            write_timeout: None,
            channel_type: ChannelType::Layer2,
            bpf_fd_attempts: 1000,
            linux_fanout: None,
            promiscuous: true,
            socket_fd: None,
        }
    }
}

/// Create a new datalink channel for sending and receiving data.
///
/// This allows for sending and receiving packets at the data link layer.
///
/// A list of network interfaces can be retrieved using datalink::interfaces().
///
/// The configuration serves as a hint to the backend - some or all of it may be used or ignored,
/// depending on which backend is used.
///
/// When matching on the returned channel, make sure to include a catch-all so that code doesn't
/// break when new channel types are added.
#[inline]
pub fn channel(
    network_interface: &NetworkInterface,
    configuration: Config,
) -> io::Result<Channel> {
    backend::channel(
        network_interface,
        (&configuration).into()
    )
}

/// Trait to enable sending `$packet` packets.
pub trait DataLinkSender: Send {
    /// Create and send a number of packets.
    ///
    /// This will call `func` `num_packets` times. The function will be provided with a
    /// mutable packet to manipulate, which will then be sent. This allows packets to be
    /// built in-place, avoiding the copy required for `send`. If there is not sufficient
    /// capacity in the buffer, None will be returned.
    fn build_and_send(
        &mut self,
        num_packets: usize,
        packet_size: usize,
        func: &mut dyn FnMut(&mut [u8]),
    ) -> Option<io::Result<()>>;

    /// Send a packet.
    ///
    /// This may require an additional copy compared to `build_and_send`, depending on the
    /// operating system being used. The second parameter is currently ignored, however
    /// `None` should be passed.
    fn send_to(&mut self, packet: &[u8], dst: Option<NetworkInterface>) -> Option<io::Result<()>>;
}

/// Structure for receiving packets at the data link layer. Should be constructed using
/// `datalink_channel()`.
pub trait DataLinkReceiver: Send {
    /// Get the next ethernet frame in the channel.
    fn next(&mut self) -> io::Result<&[u8]>;
}

/// Represents a network interface and its associated addresses.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct NetworkInterface {
    /// The name of the interface.
    pub name: String,
    /// A description of the interface.
    pub description: String,
    /// The interface index (operating system specific).
    pub index: u32,
    /// A MAC address for the interface.
    pub mac: Option<MacAddr>,
    /// IP addresses and netmasks for the interface.
    pub ips: Vec<IpNetwork>,
    /// Operating system specific flags for the interface.
    #[cfg(not(any(target_os = "illumos", target_os = "solaris")))]
    pub flags: u32,
    #[cfg(any(target_os = "illumos", target_os = "solaris"))]
    pub flags: u64,
}

/// Type alias for an `InterfaceType`.
#[cfg(not(any(target_os = "illumos", target_os = "solaris")))]
pub type InterfaceType = u32;
#[cfg(any(target_os = "illumos", target_os = "solaris"))]
pub type InterfaceType = u64;

impl NetworkInterface {
    pub fn is_up(&self) -> bool {
        self.flags & (pnet_sys::IFF_UP as InterfaceType) != 0
    }

    pub fn is_broadcast(&self) -> bool {
        self.flags & (pnet_sys::IFF_BROADCAST as InterfaceType) != 0
    }

    /// Is the interface a loopback interface?
    pub fn is_loopback(&self) -> bool {
        self.flags & (pnet_sys::IFF_LOOPBACK as InterfaceType) != 0
    }

    pub fn is_point_to_point(&self) -> bool {
        self.flags & (pnet_sys::IFF_POINTOPOINT as InterfaceType) != 0
    }

    pub fn is_multicast(&self) -> bool {
        self.flags & (pnet_sys::IFF_MULTICAST as InterfaceType) != 0
    }

    /// Triggered when the driver has signated netif_carrier_on
    /// Check <https://www.kernel.org/doc/html/latest/networking/operstates.html> for more information
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn is_lower_up(&self) -> bool {
        self.flags & (pnet_sys::IFF_LOWER_UP as InterfaceType) != 0
    }

    /// Triggered when the driver has signated netif_dormant_on
    /// Check <https://www.kernel.org/doc/html/latest/networking/operstates.html> for more information
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn is_dormant(&self) -> bool {
        self.flags & (pnet_sys::IFF_DORMANT as InterfaceType) != 0
    }

    #[cfg(unix)]
    pub fn is_running(&self) -> bool {
        self.flags & (pnet_sys::IFF_RUNNING as InterfaceType) != 0
    }
}

impl ::std::fmt::Display for NetworkInterface {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        const FLAGS: [&'static str; 8] = [
            "UP",
            "BROADCAST",
            "LOOPBACK",
            "POINTOPOINT",
            "MULTICAST",
            "RUNNING",
            "DORMANT",
            "LOWERUP",
        ];
        let flags = if self.flags > 0 {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            let rets = [
                self.is_up(),
                self.is_broadcast(),
                self.is_loopback(),
                self.is_point_to_point(),
                self.is_multicast(),
                self.is_running(),
                self.is_dormant(),
                self.is_lower_up(),
            ];
            #[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
            let rets = [
                self.is_up(),
                self.is_broadcast(),
                self.is_loopback(),
                self.is_point_to_point(),
                self.is_multicast(),
                self.is_running(),
                false,
                false,
            ];
            #[cfg(not(unix))]
            let rets = [
                self.is_up(),
                self.is_broadcast(),
                self.is_loopback(),
                self.is_point_to_point(),
                self.is_multicast(),
                false,
                false,
                false,
            ];

            format!(
                "{:X}<{}>",
                self.flags,
                rets.iter()
                    .zip(FLAGS.iter())
                    .filter(|&(ret, _)| ret == &true)
                    .map(|(_, name)| name.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            )
        } else {
            format!("{:X}", self.flags)
        };

        let mac = self
            .mac
            .map(|mac| mac.to_string())
            .unwrap_or("N/A".to_owned());
        let ips = if self.ips.len() > 0 {
            format!(
                "\n{}",
                self.ips
                    .iter()
                    .map(|ip| {
                        if ip.is_ipv4() {
                            format!("       inet: {}", ip)
                        } else {
                            format!("      inet6: {}", ip)
                        }
                    })
                    .collect::<Vec<String>>()
                    .join("\n")
            )
        } else {
            "".to_string()
        };

        write!(
            f,
            "{}: flags={}
      index: {}
      ether: {}{}",
            self.name, flags, self.index, mac, ips
        )
    }
}

/// Get a list of available network interfaces for the current machine.
///
/// If you need the default network interface, you can choose the first
/// one that is up, not loopback and has an IP. This is not guaranteed to
/// work on each system but should work for basic packet sniffing:
///
/// ```
/// use pnet_datalink::interfaces;
///
/// // Get a vector with all network interfaces found
/// let all_interfaces = interfaces();
///
/// // Search for the default interface - the one that is
/// // up, not loopback and has an IP.
/// let default_interface = all_interfaces
///     .iter()
///     .find(|e| e.is_up() && !e.is_loopback() && !e.ips.is_empty());
///
/// match default_interface {
///     Some(interface) => println!("Found default interface with [{}].", interface.name),
///     None => println!("Error while finding the default interface."),
/// }
///
/// ```
///
pub fn interfaces() -> Vec<NetworkInterface> {
    backend::interfaces()
}
